//! # Resilience Patterns Module
//!
//! Provides robust resilience patterns for external service calls:
//! - Exponential backoff retry logic with jitter
//! - Circuit breaker pattern for fault isolation
//! - Bulkhead pattern for resource isolation
//! - Timeout management with cascading deadlines

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    
    /// Initial retry interval (in milliseconds)
    pub initial_interval_ms: u64,
    
    /// Maximum retry interval (in milliseconds)
    pub max_interval_ms: u64,
    
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
    
    /// Jitter percentage (0-1)
    pub jitter_factor: f64,
    
    /// Retry budget per service (max retries per minute)
    pub budget_per_minute: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_interval_ms: 1000,
            max_interval_ms: 32000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            budget_per_minute: 100,
        }
    }
}

/// Retry budget tracker
#[derive(Debug, Clone)]
struct RetryBudget {
    attempts: Vec<Instant>,
    max_per_minute: u32,
}

impl RetryBudget {
    fn new(max_per_minute: u32) -> Self {
        Self {
            attempts: Vec::new(),
            max_per_minute,
        }
    }
    
    fn can_retry(&mut self) -> bool {
        let now = Instant::now();
        let one_minute_ago = now - Duration::from_secs(60);
        
        // Remove attempts older than 1 minute
        self.attempts.retain(|&attempt| attempt > one_minute_ago);
        
        if self.attempts.len() < self.max_per_minute as usize {
            self.attempts.push(now);
            true
        } else {
            false
        }
    }
}

/// Retry manager with budget tracking
pub struct RetryManager {
    policy: RetryPolicy,
    budgets: Arc<Mutex<HashMap<String, RetryBudget>>>,
}

impl RetryManager {
    /// Create a new retry manager
    pub fn new(policy: RetryPolicy) -> Self {
        Self {
            policy,
            budgets: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Calculate next retry delay with exponential backoff and jitter
    pub fn calculate_retry_delay(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(0);
        }
        
        let base_delay = (self.policy.initial_interval_ms as f64
            * self.policy.backoff_multiplier.powi(attempt as i32 - 1))
            .min(self.policy.max_interval_ms as f64) as u64;
        
        // Add jitter
        let jitter = (base_delay as f64 * self.policy.jitter_factor) as u64;
        let random_jitter = (rand::random::<f64>() * jitter as f64) as u64;
        let final_delay = base_delay + random_jitter;
        
        Duration::from_millis(final_delay)
    }
    
    /// Check if retry is possible within budget
    pub async fn can_retry(&self, service: &str) -> bool {
        let mut budgets = self.budgets.lock().await;
        let budget = budgets
            .entry(service.to_string())
            .or_insert_with(|| RetryBudget::new(self.policy.budget_per_minute));
        
        budget.can_retry()
    }
}

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Circuit is closed (normal operation)
    Closed,
    /// Circuit is open (failing fast)
    Open,
    /// Circuit is half-open (testing recovery)
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Failure threshold before tripping
    pub failure_threshold: u32,
    
    /// Success threshold before closing
    pub success_threshold: u32,
    
    /// Timeout before attempting recovery (in seconds)
    pub timeout_secs: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_secs: 60,
        }
    }
}

/// Circuit breaker for fault isolation
pub struct CircuitBreaker {
    name: String,
    state: Arc<RwLock<CircuitState>>,
    config: CircuitBreakerConfig,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(name: String, config: CircuitBreakerConfig) -> Self {
        Self {
            name,
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            config,
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Get current state
    pub async fn get_state(&self) -> CircuitState {
        *self.state.read().await
    }
    
    /// Record a successful call
    pub async fn record_success(&self) {
        let state = self.get_state().await;
        
        match state {
            CircuitState::Closed => {
                // Reset counters
                *self.failure_count.write().await = 0;
            }
            CircuitState::HalfOpen => {
                // Increment success count
                let mut success_count = self.success_count.write().await;
                *success_count += 1;
                
                if *success_count >= self.config.success_threshold {
                    // Close the circuit
                    *self.state.write().await = CircuitState::Closed;
                    *self.failure_count.write().await = 0;
                    *self.success_count.write().await = 0;
                    
                    log::info!("[CircuitBreaker] Circuit closed: {}", self.name);
                }
            }
            _ => {}
        }
    }
    
    /// Record a failed call
    pub async fn record_failure(&self) {
        let state = self.get_state().await;
        
        *self.last_failure_time.write().await = Some(Instant::now());
        
        match state {
            CircuitState::Closed => {
                // Increment failure count
                let mut failure_count = self.failure_count.write().await;
                *failure_count += 1;
                
                if *failure_count >= self.config.failure_threshold {
                    // Open the circuit
                    *self.state.write().await = CircuitState::Open;
                    *self.success_count.write().await = 0;
                    
                    log::warn!("[CircuitBreaker] Circuit opened: {}", self.name);
                }
            }
            CircuitState::HalfOpen => {
                // Return to open state
                *self.state.write().await = CircuitState::Open;
                *self.success_count.write().await = 0;
                
                log::warn!("[CircuitBreaker] Circuit reopened: {}", self.name);
            }
            _ => {}
        }
    }
    
    /// Attempt to transition to half-open if timeout has elapsed
    pub async fn attempt_recovery(&self) -> bool {
        let state = self.get_state().await;
        
        if state == CircuitState::Open {
            if let Some(last_failure) = *self.last_failure_time.read().await {
                if last_failure.elapsed() >= Duration::from_secs(self.config.timeout_secs) {
                    *self.state.write().await = CircuitState::HalfOpen;
                    *self.success_count.write().await = 0;
                    
                    log::info!("[CircuitBreaker] Circuit half-open: {}", self.name);
                    return true;
                }
            }
        }
        
        state == CircuitState::HalfOpen
    }
}

impl Clone for CircuitBreaker {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            state: self.state.clone(),
            config: self.config.clone(),
            failure_count: self.failure_count.clone(),
            success_count: self.success_count.clone(),
            last_failure_time: self.last_failure_time.clone(),
        }
    }
}

/// Bulkhead configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkheadConfig {
    /// Maximum concurrent requests
    pub max_concurrent: usize,
    
    /// Maximum queue size
    pub max_queue: usize,
    
    /// Request timeout (in seconds)
    pub timeout_secs: u64,
}

impl Default for BulkheadConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            max_queue: 100,
            timeout_secs: 30,
        }
    }
}

/// Bulkhead semaphore for resource isolation
pub struct BulkheadExecutor {
    name: String,
    semaphore: Arc<tokio::sync::Semaphore>,
    config: BulkheadConfig,
    current_requests: Arc<RwLock<u32>>,
    queue_size: Arc<RwLock<u32>>,
}

impl BulkheadExecutor {
    /// Create a new bulkhead executor
    pub fn new(name: String, config: BulkheadConfig) -> Self {
        Self {
            name,
            semaphore: Arc::new(tokio::sync::Semaphore::new(config.max_concurrent)),
            config,
            current_requests: Arc::new(RwLock::new(0)),
            queue_size: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Execute with bulkhead protection
    pub async fn execute<F, R>(&self, f: F) -> Result<R, String>
    where
        F: std::future::Future<Output = Result<R, String>>,
    {
        // Check queue size
        let queue = *self.queue_size.read().await;
        if queue >= self.config.max_queue as u32 {
            return Err("Bulkhead queue full".to_string());
        }
        
        // Increment queue size
        *self.queue_size.write().await += 1;
        
        // Acquire permit with timeout
        let permit = tokio::time::timeout(
            Duration::from_secs(self.config.timeout_secs),
            self.semaphore.acquire(),
        )
        .await
        .map_err(|_| "Bulkhead timeout waiting for permit".to_string())?
        .map_err(|e| format!("Bulkhead semaphore error: {}", e))?;
        
        // Decrement queue size, increment current requests
        *self.queue_size.write().await -= 1;
        *self.current_requests.write().await += 1;
        
        // Execute with timeout
        let result = tokio::time::timeout(
            Duration::from_secs(self.config.timeout_secs),
            f,
        )
        .await
        .map_err(|_| "Bulkhead execution timeout".to_string())?;
        
        // Decrement current requests and drop permit
        *self.current_requests.write().await -= 1;
        drop(permit);
        
        result
    }
    
    /// Get current load
    pub async fn get_load(&self) -> (u32, u32) {
        let current = *self.current_requests.read().await;
        let queue = *self.queue_size.read().await;
        (current, queue)
    }
}

impl Clone for BulkheadExecutor {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            semaphore: self.semaphore.clone(),
            config: self.config.clone(),
            current_requests: self.current_requests.clone(),
            queue_size: self.queue_size.clone(),
        }
    }
}

/// Timeout manager for cascading deadlines
#[derive(Debug, Clone)]
pub struct TimeoutManager {
    global_deadline: Option<Instant>,
    operation_timeout: Duration,
}

impl TimeoutManager {
    /// Create a new timeout manager
    pub fn new(operation_timeout: Duration) -> Self {
        Self {
            global_deadline: None,
            operation_timeout,
        }
    }
    
    /// Create with global deadline
    pub fn with_deadline(global_deadline: Instant, operation_timeout: Duration) -> Self {
        Self {
            global_deadline: Some(global_deadline),
            operation_timeout,
        }
    }
    
    /// Get remaining time until deadline
    pub fn remaining(&self) -> Option<Duration> {
        self.global_deadline.map(|deadline| {
            deadline
                .checked_duration_since(Instant::now())
                .unwrap_or(Duration::from_secs(0))
        })
    }
    
    /// Get effective timeout (minimum of operation timeout and remaining time)
    pub fn effective_timeout(&self) -> Duration {
        match self.remaining() {
            Some(remaining) => self.operation_timeout.min(remaining),
            None => self.operation_timeout,
        }
    }
    
    /// Check if deadline has been exceeded
    pub fn is_exceeded(&self) -> bool {
        self.global_deadline.map_or(false, |deadline| Instant::now() >= deadline)
    }
}

/// Resilience orchestrator combining all patterns
pub struct ResilienceOrchestrator {
    retry_manager: Arc<RetryManager>,
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
    bulkheads: Arc<RwLock<HashMap<String, BulkheadExecutor>>>,
}

impl ResilienceOrchestrator {
    /// Create a new resilience orchestrator
    pub fn new(retry_policy: RetryPolicy) -> Self {
        Self {
            retry_manager: Arc::new(RetryManager::new(retry_policy)),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            bulkheads: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Get or create circuit breaker
    pub async fn get_circuit_breaker(
        &self,
        service: &str,
        config: CircuitBreakerConfig,
    ) -> Arc<CircuitBreaker> {
        let mut breakers = self.circuit_breakers.write().await;
        Arc::new(
            breakers
                .entry(service.to_string())
                .or_insert_with(|| CircuitBreaker::new(service.to_string(), config))
                .clone(),
        )
    }
    
    /// Get or create bulkhead
    pub async fn get_bulkhead(
        &self,
        service: &str,
        config: BulkheadConfig,
    ) -> Arc<BulkheadExecutor> {
        let mut bulkheads = self.bulkheads.write().await;
        Arc::new(
            bulkheads
                .entry(service.to_string())
                .or_insert_with(|| BulkheadExecutor::new(service.to_string(), config))
                .clone(),
        )
    }
    
    /// Execute with full resilience
    pub async fn execute_resilient<F, R>(
        &self,
        service: &str,
        retry_policy: &RetryPolicy,
        circuit_config: CircuitBreakerConfig,
        bulkhead_config: BulkheadConfig,
        f: F,
    ) -> Result<R, String>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R, String>> + Send>>,
    {
        let breaker = self.get_circuit_breaker(service, circuit_config).await;
        let bulkhead = self.get_bulkhead(service, bulkhead_config).await;
        
        // Check circuit state
        if breaker.get_state().await == CircuitState::Open {
            if !breaker.attempt_recovery().await {
                return Err("Circuit breaker is open".to_string());
            }
        }
        
        // Execute with bulkhead protection and retry logic
        let mut attempt = 0;
        
        loop {
            let result = bulkhead
                .execute(f())
                .await;
            
            match result {
                Ok(value) => {
                    breaker.record_success().await;
                    return Ok(value);
                }
                Err(e) => {
                    breaker.record_failure().await;
                    
                    if attempt < retry_policy.max_retries
                        && self.retry_manager.can_retry(service).await
                    {
                        let delay = self.retry_manager.calculate_retry_delay(attempt);
                        log::debug!(
                            "[ResilienceOrchestrator] Retrying {} (attempt {}/{}) after {:?}",
                            service,
                            attempt + 1,
                            retry_policy.max_retries,
                            delay
                        );
                        
                        tokio::time::sleep(delay).await;
                        attempt += 1;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }
}

impl Clone for ResilienceOrchestrator {
    fn clone(&self) -> Self {
        Self {
            retry_manager: self.retry_manager.clone(),
            circuit_breakers: self.circuit_breakers.clone(),
            bulkheads: self.bulkheads.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_retry_delay_calculation() {
        let policy = RetryPolicy::default();
        let manager = RetryManager::new(policy);
        
        let delay_1 = manager.calculate_retry_delay(1);
        let delay_2 = manager.calculate_retry_delay(2);
        
        // delay_2 should be roughly double delay_1 (with some jitter)
        assert!(delay_2 >= delay_1);
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_state_transitions() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 1,
            timeout_secs: 1,
        };
        let breaker = CircuitBreaker::new("test".to_string(), config);
        
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
        
        breaker.record_failure().await;
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
        
        breaker.record_failure().await;
        assert_eq!(breaker.get_state().await, CircuitState::Open);
        
        assert!(breaker.attempt_recovery().await);
        assert_eq!(breaker.get_state().await, CircuitState::HalfOpen);
        
        breaker.record_success().await;
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
    }
    
    #[tokio::test]
    async fn test_bulkhead_resource_isolation() {
        let config = BulkheadConfig {
            max_concurrent: 2,
            max_queue: 5,
            timeout_secs: 10,
        };
        let bulkhead = BulkheadExecutor::new("test".to_string(), config);
        
        let (_current, _queue) = bulkhead.get_load().await;
        assert_eq!(_current, 0);
        assert_eq!(_queue, 0);
    }
}
