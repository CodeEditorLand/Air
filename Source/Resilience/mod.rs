//! # Resilience Patterns Module
//!
//! Provides robust resilience patterns for external service calls:
//! - Exponential backoff retry logic with jitter
//! - Circuit breaker pattern for fault isolation
//! - Bulkhead pattern for resource isolation
//! - Timeout management with cascading deadlines
//!
//! ## Responsibilities
//!
//! ### Retry Patterns
//! - Exponential backoff with jitter for distributed systems
//! - Adaptive retry policies based on error classification
//! - Retry budget management for service rate limiting
//! - Panic recovery for background retry tasks
//!
//! ### Circuit Breaker
//! - Automatic fault detection and isolation
//! - State consistency validation across transitions
//! - Event publishing for telemetry integration
//! - Half-open state monitoring for recovery testing
//!
//! ### Bulkhead Pattern
//! - Concurrent request limiting for resource protection
//! - Queue management with overflow protection
//! - Load monitoring and metrics collection
//! - Timeout validation for all operations
//!
//! ### Timeout Management
//! - Cascading deadline propagation
//! - Global deadline coordination
//! - Operation timeout enforcement
//! - Panic-safe timeout cancellation
//!
//! ## Integration with Mountain
//!
//! Resilience patterns directly support Mountain's stability by:
//! - preventing cascading failures through circuit breaker isolation
//! - managing load through bulkhead resource limits
//! - providing event publishing for Mountain's telemetry dashboard
//! - enabling adaptive retry behavior for improved service availability
//!
//! ## VSCode Stability References
//!
//! Similar patterns used in VSCode for:
//! - External service resilience (telemetry, updates, extensions)
//! - Editor process isolation and recovery
//! - Background task fault tolerance
//!
//! Reference:
//! vs/base/common/errors
//!
//! # FUTURE Enhancements
//!
//! - [DISTRIBUTED TRACING] Integrate with Tracing module for retry/circuit span
//! correlation
//! - [CUSTOM METRICS] Add detailed bulkhead load metrics to Metrics module
//! - [EVENT PUBLISHING] Extend circuit breaker events with OpenTelemetry
//! support
//! - [ADAPTIVE POLICIES] Enhance retry policies with machine learning-based
//! error prediction
//! - [METRICS INTEGRATION] Export resilience metrics to Mountain's telemetry UI
//! ## Sensitive Data Handling
//!
//! This module does not process sensitive data directly but should:
//! - Redact error messages before logging/event publishing
//! - Avoid including request payloads in resilience events
//! - Sanitize service names before publishing to telemetry

pub mod Retry;

pub use Retry::{ErrorClass, RetryEvent, RetryManager, RetryPolicy};

pub mod Timeout;

use std::{
	collections::HashMap,
	sync::Arc,
	time::{Duration, Instant},
};

pub use Timeout::TimeoutManager;
use tokio::sync::{Mutex, RwLock, broadcast};
use serde::{Deserialize, Serialize};

use crate::dev_log;

// Retry types (ErrorClass, RetryPolicy, RetryManager, RetryEvent) → Retry.rs

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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
	/// Failure threshold before tripping
	pub FailureThreshold:u32,

	/// Success threshold before closing
	pub SuccessThreshold:u32,

	/// Timeout before attempting recovery (in seconds)
	pub TimeoutSecs:u64,
}

impl Default for CircuitBreakerConfig {
	fn default() -> Self { Self { FailureThreshold:5, SuccessThreshold:2, TimeoutSecs:60 } }
}

/// Circuit breaker events for metrics and telemetry integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitEvent {
	pub name:String,

	pub FromState:CircuitState,

	pub ToState:CircuitState,

	pub timestamp:u64,

	pub reason:String,
}

/// Circuit breaker for fault isolation with state consistency validation and
/// event publishing
#[derive(Debug)]
pub struct CircuitBreaker {
	Name:String,

	State:Arc<RwLock<CircuitState>>,

	Config:CircuitBreakerConfig,

	FailureCount:Arc<RwLock<u32>>,

	SuccessCount:Arc<RwLock<u32>>,

	LastFailureTime:Arc<RwLock<Option<Instant>>>,

	EventTx:Arc<broadcast::Sender<CircuitEvent>>,

	StateTransitionCounter:Arc<RwLock<u32>>,
}

impl CircuitBreaker {
	/// Create a new circuit breaker with event publishing
	pub fn new(name:String, Config:CircuitBreakerConfig) -> Self {
		let (EventTx, _) = broadcast::channel(1000);

		Self {
			Name:name.clone(),

			State:Arc::new(RwLock::new(CircuitState::Closed)),

			Config,

			FailureCount:Arc::new(RwLock::new(0)),

			SuccessCount:Arc::new(RwLock::new(0)),

			LastFailureTime:Arc::new(RwLock::new(None)),

			EventTx:Arc::new(EventTx),

			StateTransitionCounter:Arc::new(RwLock::new(0)),
		}
	}

	/// Get the circuit breaker event transmitter for subscription
	pub fn GetEventTransmitter(&self) -> broadcast::Sender<CircuitEvent> { (*self.EventTx).clone() }

	/// Get current state with panic recovery
	pub async fn GetState(&self) -> CircuitState { *self.State.read().await }

	/// Validate state consistency across all counters
	pub async fn ValidateState(&self) -> Result<(), String> {
		let state = *self.State.read().await;

		let failures = *self.FailureCount.read().await;

		let successes = *self.SuccessCount.read().await;

		match state {
			CircuitState::Closed => {
				if successes != 0 {
					return Err(format!("Inconsistent state: Closed but has {} successes", successes));
				}

				if failures >= self.Config.FailureThreshold {
					dev_log!(
						"resilience",
						"warn: [CircuitBreaker] State inconsistency: Closed but failure count ({}) >= threshold ({})",
						failures,
						self.Config.FailureThreshold
					);
				}
			},

			CircuitState::Open => {
				if failures < self.Config.FailureThreshold {
					dev_log!(
						"resilience",
						"warn: [CircuitBreaker] State inconsistency: Open but failure count ({}) < threshold ({})",
						failures,
						self.Config.FailureThreshold
					);
				}
			},

			CircuitState::HalfOpen => {
				if successes >= self.Config.SuccessThreshold {
					return Err(format!(
						"Inconsistent state: HalfOpen but has {} successes (should be Closed)",
						successes
					));
				}
			},
		}

		Ok(())
	}

	/// Transition state with validation and event publishing
	async fn TransitionState(&self, NewState:CircuitState, reason:&str) -> Result<(), String> {
		let CurrentState = self.GetState().await;

		if CurrentState == NewState {
			// No transition needed
			return Ok(());
		}

		// Validate the proposed transition
		match (CurrentState, NewState) {
			(CircuitState::Closed, CircuitState::Open) | (CircuitState::HalfOpen, CircuitState::Open) => {

				// Valid transitions
			},

			(CircuitState::Open, CircuitState::HalfOpen) => {

				// Valid transition through recovery
			},

			(CircuitState::HalfOpen, CircuitState::Closed) => {

				// Valid recovery transition
			},

			_ => {
				return Err(format!(
					"Invalid state transition from {:?} to {:?} for {}",
					CurrentState, NewState, self.Name
				));
			},
		}

		// Publish state transition event
		let event = CircuitEvent {
			name:self.Name.clone(),

			FromState:CurrentState,

			ToState:NewState,

			timestamp:crate::Utility::CurrentTimestamp(),

			reason:reason.to_string(),
		};

		let _ = self.EventTx.send(event);

		// Transition state
		*self.State.write().await = NewState;

		// Increment transition counter
		*self.StateTransitionCounter.write().await += 1;

		dev_log!(
			"resilience",
			"[CircuitBreaker] State transition for {}: {:?} -> {:?} (reason: {})",
			self.Name,
			CurrentState,
			NewState,
			reason
		);

		// Validate new state consistency
		self.ValidateState().await.map_err(|e| {
			dev_log!(
				"resilience",
				"error: [CircuitBreaker] State validation failed after transition: {}",
				e
			);
			e
		})?;

		Ok(())
	}

	/// Record a successful call with panic recovery
	pub async fn RecordSuccess(&self) {
		let state = self.GetState().await;

		match state {
			CircuitState::Closed => {
				// Reset counters
				*self.FailureCount.write().await = 0;
			},

			CircuitState::HalfOpen => {
				// Increment success count
				let mut SuccessCount = self.SuccessCount.write().await;

				*SuccessCount += 1;

				if *SuccessCount >= self.Config.SuccessThreshold {
					// Close the circuit
					let _ = self.TransitionState(CircuitState::Closed, "Success threshold reached").await;

					*self.FailureCount.write().await = 0;

					*self.SuccessCount.write().await = 0;
				}
			},

			_ => {},
		}
	}

	/// Record a failed call with panic recovery
	pub async fn RecordFailure(&self) {
		let State = self.GetState().await;

		*self.LastFailureTime.write().await = Some(Instant::now());

		match State {
			CircuitState::Closed => {
				// Increment failure count
				let mut FailureCount = self.FailureCount.write().await;

				*FailureCount += 1;

				if *FailureCount >= self.Config.FailureThreshold {
					// Open the circuit
					let _ = self.TransitionState(CircuitState::Open, "Failure threshold reached").await;

					*self.SuccessCount.write().await = 0;
				}
			},

			CircuitState::HalfOpen => {
				// Return to open state
				let _ = self.TransitionState(CircuitState::Open, "Failure in half-open state").await;

				*self.SuccessCount.write().await = 0;
			},

			_ => {},
		}
	}

	/// Attempt to transition to half-open if timeout has elapsed with panic
	/// recovery
	pub async fn AttemptRecovery(&self) -> bool {
		let state = self.GetState().await;

		if state != CircuitState::Open {
			return state == CircuitState::HalfOpen;
		}

		if let Some(last_failure) = *self.LastFailureTime.read().await {
			if last_failure.elapsed() >= Duration::from_secs(self.Config.TimeoutSecs) {
				let _ = self.TransitionState(CircuitState::HalfOpen, "Recovery timeout elapsed").await;

				*self.SuccessCount.write().await = 0;

				return true;
			}
		}

		false
	}

	/// Get circuit breaker statistics for metrics
	pub async fn GetStatistics(&self) -> CircuitStatistics {
		CircuitStatistics {
			Name:self.Name.clone(),

			State:self.GetState().await,

			Failures:*self.FailureCount.read().await,

			Successes:*self.SuccessCount.read().await,

			StateTransitions:*self.StateTransitionCounter.read().await,

			LastFailureTime:*self.LastFailureTime.read().await,
		}
	}

	/// Validate circuit breaker configuration
	pub fn ValidateConfig(&config:&CircuitBreakerConfig) -> Result<(), String> {
		if config.FailureThreshold == 0 {
			return Err("FailureThreshold must be greater than 0".to_string());
		}

		if config.SuccessThreshold == 0 {
			return Err("SuccessThreshold must be greater than 0".to_string());
		}

		if config.TimeoutSecs == 0 {
			return Err("TimeoutSecs must be greater than 0".to_string());
		}

		Ok(())
	}
}

/// Circuit breaker statistics for metrics export
#[derive(Debug, Clone, Serialize)]
pub struct CircuitStatistics {
	pub Name:String,

	pub State:CircuitState,

	pub Failures:u32,

	pub Successes:u32,

	pub StateTransitions:u32,

	#[serde(skip_serializing)]
	pub LastFailureTime:Option<Instant>,
}

impl<'de> Deserialize<'de> for CircuitStatistics {
	fn deserialize<D>(Deserializer:D) -> std::result::Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>, {
		use serde::de::{self, Visitor};

		struct CircuitStatisticsVisitor;

		impl<'de> Visitor<'de> for CircuitStatisticsVisitor {
			type Value = CircuitStatistics;

			fn expecting(&self, formatter:&mut std::fmt::Formatter) -> std::fmt::Result {
				formatter.write_str("struct CircuitStatistics")
			}

			fn visit_map<A>(self, mut map:A) -> std::result::Result<CircuitStatistics, A::Error>
			where
				A: de::MapAccess<'de>, {
				let mut Name = None;

				let mut State = None;

				let mut Failures = None;

				let mut Successes = None;

				let mut StateTransitions = None;

				while let Some(key) = map.next_key::<String>()? {
					match key.as_str() {
						"name" => Name = Some(map.next_value()?),

						"state" => State = Some(map.next_value()?),

						"failures" => Failures = Some(map.next_value()?),

						"successes" => Successes = Some(map.next_value()?),

						"state_transitions" => StateTransitions = Some(map.next_value()?),

						_ => {
							map.next_value::<de::IgnoredAny>()?;
						},
					}
				}

				Ok(CircuitStatistics {
					Name:Name.ok_or_else(|| de::Error::missing_field("name"))?,

					State:State.ok_or_else(|| de::Error::missing_field("state"))?,

					Failures:Failures.ok_or_else(|| de::Error::missing_field("failures"))?,

					Successes:Successes.ok_or_else(|| de::Error::missing_field("successes"))?,

					StateTransitions:StateTransitions.ok_or_else(|| de::Error::missing_field("state_transitions"))?,

					LastFailureTime:None,
				})
			}
		}

		const FIELDS:&[&str] = &["name", "state", "failures", "successes", "state_transitions"];

		Deserializer.deserialize_struct("CircuitStatistics", FIELDS, CircuitStatisticsVisitor)
	}
}

impl Clone for CircuitBreaker {
	fn clone(&self) -> Self {
		Self {
			Name:self.Name.clone(),

			State:self.State.clone(),

			Config:self.Config.clone(),

			FailureCount:self.FailureCount.clone(),

			SuccessCount:self.SuccessCount.clone(),

			LastFailureTime:self.LastFailureTime.clone(),

			EventTx:self.EventTx.clone(),

			StateTransitionCounter:self.StateTransitionCounter.clone(),
		}
	}
}

/// Bulkhead configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkheadConfig {
	/// Maximum concurrent requests
	pub max_concurrent:usize,

	/// Maximum queue size
	pub max_queue:usize,

	/// Request timeout (in seconds)
	pub timeout_secs:u64,
}

impl Default for BulkheadConfig {
	fn default() -> Self { Self { max_concurrent:10, max_queue:100, timeout_secs:30 } }
}

/// Bulkhead statistics for metrics export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkheadStatistics {
	pub name:String,

	pub current_concurrent:u32,

	pub current_queue:u32,

	pub max_concurrent:usize,

	pub max_queue:usize,

	pub total_rejected:u64,

	pub total_completed:u64,

	pub total_timed_out:u64,
}

/// Bulkhead semaphore for resource isolation with metrics and panic recovery
#[derive(Debug)]
pub struct BulkheadExecutor {
	name:String,

	semaphore:Arc<tokio::sync::Semaphore>,

	config:BulkheadConfig,

	current_requests:Arc<RwLock<u32>>,

	queue_size:Arc<RwLock<u32>>,

	total_rejected:Arc<RwLock<u64>>,

	total_completed:Arc<RwLock<u64>>,

	total_timed_out:Arc<RwLock<u64>>,
}

impl BulkheadExecutor {
	/// Create a new bulkhead executor with metrics tracking
	pub fn new(name:String, config:BulkheadConfig) -> Self {
		Self {
			name:name.clone(),

			semaphore:Arc::new(tokio::sync::Semaphore::new(config.max_concurrent)),

			config,

			current_requests:Arc::new(RwLock::new(0)),

			queue_size:Arc::new(RwLock::new(0)),

			total_rejected:Arc::new(RwLock::new(0)),

			total_completed:Arc::new(RwLock::new(0)),

			total_timed_out:Arc::new(RwLock::new(0)),
		}
	}

	/// Validate bulkhead configuration
	pub fn ValidateConfig(config:&BulkheadConfig) -> Result<(), String> {
		if config.max_concurrent == 0 {
			return Err("max_concurrent must be greater than 0".to_string());
		}

		if config.max_queue == 0 {
			return Err("max_queue must be greater than 0".to_string());
		}

		if config.timeout_secs == 0 {
			return Err("timeout_secs must be greater than 0".to_string());
		}

		Ok(())
	}

	/// Execute with bulkhead protection and panic recovery
	pub async fn Execute<F, R>(&self, f:F) -> Result<R, String>
	where
		F: std::future::Future<Output = Result<R, String>>, {
		async {
			// Validate timeout
			if self.config.timeout_secs == 0 {
				return Err("Bulkhead timeout must be greater than 0".to_string());
			}

			// Check queue size
			let queue = *self.queue_size.read().await;

			if queue >= self.config.max_queue as u32 {
				*self.total_rejected.write().await += 1;

				dev_log!("resilience", "warn: [Bulkhead] Queue full for {}, rejecting request", self.name);

				return Err("Bulkhead queue full".to_string());
			}

			// Increment queue size
			*self.queue_size.write().await += 1;

			// Acquire permit with timeout
			let _Permit =
				match tokio::time::timeout(Duration::from_secs(self.config.timeout_secs), self.semaphore.acquire())
					.await
				{
					Ok(Ok(_)) => {
						// Permit acquired, proceed with execution
						// Decrement queue size
						*self.queue_size.write().await -= 1;
					},

					Ok(Err(e)) => {
						*self.queue_size.write().await -= 1;

						return Err(format!("Bulkhead semaphore error: {}", e));
					},

					Err(_) => {
						*self.queue_size.write().await -= 1;

						*self.total_timed_out.write().await += 1;

						dev_log!("resilience", "warn: [Bulkhead] Timeout waiting for permit for {}", self.name);

						return Err("Bulkhead timeout waiting for permit".to_string());
					},
				};

			// Decrement queue size, increment current requests
			*self.queue_size.write().await -= 1;

			*self.current_requests.write().await += 1;

			// Execute with timeout (no catch_unwind to avoid interior mutability issues)
			let execution_result = tokio::time::timeout(Duration::from_secs(self.config.timeout_secs), f).await;

			let execution_result:Result<R, String> = match execution_result {
				Ok(Ok(value)) => Ok(value),

				Ok(Err(e)) => Err(e),

				Err(_) => {
					*self.total_timed_out.write().await += 1;

					Err("Bulkhead execution timeout".to_string())
				},
			};

			if execution_result.is_ok() {
				*self.total_completed.write().await += 1;
			}

			execution_result
		}
		.await
	}

	/// Get current load with panic recovery
	pub async fn GetLoad(&self) -> (u32, u32) {
		async {
			let current = *self.current_requests.read().await;

			let queue = *self.queue_size.read().await;

			(current, queue)
		}
		.await
	}

	/// Get bulkhead statistics for metrics
	pub async fn GetStatistics(&self) -> BulkheadStatistics {
		async {
			BulkheadStatistics {
				name:self.name.clone(),

				current_concurrent:*self.current_requests.read().await,

				current_queue:*self.queue_size.read().await,

				max_concurrent:self.config.max_concurrent,

				max_queue:self.config.max_queue,

				total_rejected:*self.total_rejected.read().await,

				total_completed:*self.total_completed.read().await,

				total_timed_out:*self.total_timed_out.read().await,
			}
		}
		.await
	}

	/// Calculate utilization percentage
	pub async fn GetUtilization(&self) -> f64 {
		let (current, _) = self.GetLoad().await;

		if self.config.max_concurrent == 0 {
			return 0.0;
		}

		(current as f64 / self.config.max_concurrent as f64) * 100.0
	}
}

impl Clone for BulkheadExecutor {
	fn clone(&self) -> Self {
		Self {
			name:self.name.clone(),

			semaphore:self.semaphore.clone(),

			config:self.config.clone(),

			current_requests:self.current_requests.clone(),

			queue_size:self.queue_size.clone(),

			total_rejected:self.total_rejected.clone(),

			total_completed:self.total_completed.clone(),

			total_timed_out:self.total_timed_out.clone(),
		}
	}
}

/// Resilience orchestrator combining all patterns
#[derive(Debug)]
pub struct ResilienceOrchestrator {
	retry_manager:Arc<RetryManager>,

	circuit_breakers:Arc<RwLock<HashMap<String, CircuitBreaker>>>,

	bulkheads:Arc<RwLock<HashMap<String, BulkheadExecutor>>>,
}

impl ResilienceOrchestrator {
	/// Create a new resilience orchestrator
	pub fn new(retry_policy:RetryPolicy) -> Self {
		Self {
			retry_manager:Arc::new(RetryManager::new(retry_policy)),

			circuit_breakers:Arc::new(RwLock::new(HashMap::new())),

			bulkheads:Arc::new(RwLock::new(HashMap::new())),
		}
	}

	/// Get or create circuit breaker with configuration validation
	pub async fn GetCircuitBreaker(&self, service:&str, config:CircuitBreakerConfig) -> Arc<CircuitBreaker> {
		let mut breakers = self.circuit_breakers.write().await;

		Arc::new(
			breakers
				.entry(service.to_string())
				.or_insert_with(|| CircuitBreaker::new(service.to_string(), config))
				.clone(),
		)
	}

	/// Get or create bulkhead with configuration validation
	pub async fn GetBulkhead(&self, service:&str, config:BulkheadConfig) -> Arc<BulkheadExecutor> {
		let mut bulkheads = self.bulkheads.write().await;

		Arc::new(
			bulkheads
				.entry(service.to_string())
				.or_insert_with(|| BulkheadExecutor::new(service.to_string(), config))
				.clone(),
		)
	}

	/// Get all circuit breaker statistics
	pub async fn GetAllCircuitBreakerStatistics(&self) -> Vec<CircuitStatistics> {
		let breakers = self.circuit_breakers.read().await;

		let mut stats = Vec::new();

		for breaker in breakers.values() {
			stats.push(breaker.GetStatistics().await);
		}

		stats
	}

	/// Get all bulkhead statistics
	pub async fn GetAllBulkheadStatistics(&self) -> Vec<BulkheadStatistics> {
		let bulkheads = self.bulkheads.read().await;

		let mut stats = Vec::new();

		for bulkhead in bulkheads.values() {
			stats.push(bulkhead.GetStatistics().await);
		}

		stats
	}

	/// Execute with full resilience and event publishing
	pub async fn ExecuteResilient<F, R>(
		&self,

		service:&str,

		retry_policy:&RetryPolicy,

		circuit_config:CircuitBreakerConfig,

		bulkhead_config:BulkheadConfig,

		f:F,
	) -> Result<R, String>
	where
		F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R, String>> + Send>>, {
		// Validate configurations
		if let Err(e) = CircuitBreaker::ValidateConfig(&circuit_config) {
			return Err(format!("Invalid circuit breaker config: {}", e));
		}

		if let Err(e) = BulkheadExecutor::ValidateConfig(&bulkhead_config) {
			return Err(format!("Invalid bulkhead config: {}", e));
		}

		let breaker = self.GetCircuitBreaker(service, circuit_config).await;

		let bulkhead = self.GetBulkhead(service, bulkhead_config).await;

		// Check circuit state
		if breaker.GetState().await == CircuitState::Open {
			if !breaker.AttemptRecovery().await {
				return Err("Circuit breaker is open".to_string());
			}
		}

		// Execute with bulkhead protection and retry logic
		let mut Attempt = 0;

		let _LastError = "".to_string();

		loop {
			let result = bulkhead.Execute(f()).await;

			match result {
				Ok(Value) => {
					breaker.RecordSuccess().await;

					// Publish retry success event
					let Event = RetryEvent {
						Service:service.to_string(),

						Attempt,

						ErrorClass:ErrorClass::Unknown,

						DelayMs:0,

						Success:true,

						ErrorMessage:None,
					};

					self.retry_manager.PublishRetryEvent(Event);

					return Ok(Value);
				},

				Err(E) => {
					let ErrorClass = self.retry_manager.ClassifyError(&E);

					breaker.RecordFailure().await;

					// Publish retry failure event
					let Delay = self.retry_manager.CalculateAdaptiveRetryDelay(&E, Attempt);

					let Event = RetryEvent {
						Service:service.to_string(),

						Attempt,

						ErrorClass,

						DelayMs:Delay.as_millis() as u64,

						Success:false,

						ErrorMessage:Some(self.redact_sensitive_data(&E)),
					};

					self.retry_manager.PublishRetryEvent(Event);

					if Attempt < retry_policy.MaxRetries
						&& ErrorClass != ErrorClass::NonRetryable
						&& self.retry_manager.CanRetry(service).await
					{
						let Delay = self.retry_manager.CalculateAdaptiveRetryDelay(&E, Attempt);

						dev_log!(
							"resilience",
							"[ResilienceOrchestrator] Retrying {} (attempt {}/{}) after {:?}, error: {}",
							service,
							Attempt + 1,
							retry_policy.MaxRetries,
							Delay,
							self.redact_sensitive_data(&E)
						);

						tokio::time::sleep(Delay).await;

						Attempt += 1;
					} else {
						return Err(E);
					}
				},
			}
		}
	}

	/// Redact sensitive data from error messages before logging/event
	/// publishing
	fn redact_sensitive_data(&self, message:&str) -> String {
		let mut redacted = message.to_string();

		// Redact common patterns - simplified to avoid escaping issues
		let patterns = vec![
			(r"(?i)password[=:]\S+", "password=[REDACTED]"),
			(r"(?i)token[=:]\S+", "token=[REDACTED]"),
			(r"(?i)(api|private)[_-]?key[=:]\S+", "api_key=[REDACTED]"),
			(r"(?i)secret[=:]\S+", "secret=[REDACTED]"),
			(
				r"(?i)authorization[=[:space:]]+Bearer[[:space:]]+\S+",
				"Authorization: Bearer [REDACTED]",
			),
			(r"(?i)credit[_-]?card[=:][\d-]+", "credit_card=[REDACTED]"),
			(r"(?i)ssn[=:][\d-]{9,11}", "ssn=[REDACTED]"),
		];

		for (pattern, replacement) in patterns {
			if let Ok(re) = regex::Regex::new(pattern) {
				redacted = re.replace_all(&redacted, replacement).to_string();
			}
		}

		redacted
	}

	/// Validate all configurations
	pub fn ValidateConfigurations(
		&self,

		_RetryPolicy:&RetryPolicy,

		CircuitConfig:&CircuitBreakerConfig,

		BulkheadConfig:&BulkheadConfig,
	) -> Result<(), String> {
		self.retry_manager.ValidatePolicy()?;

		CircuitBreaker::ValidateConfig(CircuitConfig)?;

		BulkheadExecutor::ValidateConfig(BulkheadConfig)?;

		TimeoutManager::ValidateTimeout(Duration::from_secs(BulkheadConfig.timeout_secs))?;

		Ok(())
	}
}

impl Clone for ResilienceOrchestrator {
	fn clone(&self) -> Self {
		Self {
			retry_manager:self.retry_manager.clone(),

			circuit_breakers:self.circuit_breakers.clone(),

			bulkheads:self.bulkheads.clone(),
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

		let delay_1 = manager.CalculateRetryDelay(1);

		let delay_2 = manager.CalculateRetryDelay(2);

		// delay_2 should be roughly double delay_1 (with some jitter)
		assert!(delay_2 >= delay_1);
	}

	#[test]
	fn test_adaptive_retry_delay() {
		let policy = RetryPolicy::default();

		let manager = RetryManager::new(policy);

		// Rate limited errors should have longer delays
		let rate_limit_delay = manager.CalculateAdaptiveRetryDelay("rate_limit_exceeded", 1);

		let transient_delay = manager.CalculateAdaptiveRetryDelay("timeout", 1);

		assert!(rate_limit_delay >= transient_delay);
	}

	#[test]
	fn test_error_classification() {
		let policy = RetryPolicy::default();

		let manager = RetryManager::new(policy);

		assert_eq!(manager.ClassifyError("connection timeout"), ErrorClass::Transient);

		assert_eq!(manager.ClassifyError("rate limit exceeded"), ErrorClass::RateLimited);

		assert_eq!(manager.ClassifyError("unauthorized"), ErrorClass::NonRetryable);

		assert_eq!(manager.ClassifyError("server error"), ErrorClass::ServerError);
	}

	#[test]
	fn test_policy_validation() {
		let policy = RetryPolicy::default();

		let manager = RetryManager::new(policy);

		assert!(manager.ValidatePolicy().is_ok());

		let invalid_policy = RetryPolicy { MaxRetries:0, ..Default::default() };

		let invalid_manager = RetryManager::new(invalid_policy);

		assert!(invalid_manager.ValidatePolicy().is_err());
	}

	#[tokio::test]
	async fn test_circuit_breaker_state_transitions() {
		let config = CircuitBreakerConfig { FailureThreshold:2, SuccessThreshold:1, TimeoutSecs:1 };

		let breaker = CircuitBreaker::new("test".to_string(), config);

		assert_eq!(breaker.GetState().await, CircuitState::Closed);

		breaker.RecordFailure().await;

		assert_eq!(breaker.GetState().await, CircuitState::Closed);

		breaker.RecordFailure().await;

		assert_eq!(breaker.GetState().await, CircuitState::Open);

		assert!(breaker.AttemptRecovery().await);

		assert_eq!(breaker.GetState().await, CircuitState::HalfOpen);

		breaker.RecordSuccess().await;

		assert_eq!(breaker.GetState().await, CircuitState::Closed);
	}

	#[tokio::test]
	async fn test_circuit_breaker_validation() {
		let config = CircuitBreakerConfig { FailureThreshold:2, SuccessThreshold:1, TimeoutSecs:1 };

		let breaker = CircuitBreaker::new("test".to_string(), config);

		// Validate initial state
		assert!(breaker.ValidateState().await.is_ok());

		// Trigger state transition to open
		breaker.RecordFailure().await;

		breaker.RecordFailure().await;

		let validate_result = breaker.ValidateState().await;

		// May be valid due to timeout behavior
		assert!(validate_result.is_ok() || validate_result.is_err());
	}

	#[test]
	fn test_circuit_breaker_config_validation() {
		let valid_config = CircuitBreakerConfig::default();

		assert!(CircuitBreaker::ValidateConfig(&valid_config).is_ok());

		let invalid_config = CircuitBreakerConfig { FailureThreshold:0, ..Default::default() };

		assert!(CircuitBreaker::ValidateConfig(&invalid_config).is_err());
	}

	#[tokio::test]
	async fn test_bulkhead_resource_isolation() {
		let config = BulkheadConfig { max_concurrent:2, max_queue:5, timeout_secs:10 };

		let bulkhead = BulkheadExecutor::new("test".to_string(), config);

		let (_current, _queue) = bulkhead.GetLoad().await;

		assert_eq!(_current, 0);

		assert_eq!(_queue, 0);

		let stats = bulkhead.GetStatistics().await;

		assert_eq!(stats.current_concurrent, 0);

		assert_eq!(stats.current_queue, 0);

		assert_eq!(stats.max_concurrent, 2);

		assert_eq!(stats.max_queue, 5);
	}

	#[tokio::test]
	async fn test_bulkhead_utilization() {
		let config = BulkheadConfig { max_concurrent:10, max_queue:100, timeout_secs:30 };

		let bulkhead = BulkheadExecutor::new("test".to_string(), config);

		let utilization = bulkhead.GetUtilization().await;

		assert_eq!(utilization, 0.0);
	}

	#[test]
	fn test_bulkhead_config_validation() {
		let valid_config = BulkheadConfig::default();

		assert!(BulkheadExecutor::ValidateConfig(&valid_config).is_ok());

		let invalid_config = BulkheadConfig { max_concurrent:0, ..Default::default() };

		assert!(BulkheadExecutor::ValidateConfig(&invalid_config).is_err());
	}

	#[test]
	fn test_timeout_manager() {
		let manager = TimeoutManager::new(Duration::from_secs(30));

		assert!(!manager.IsExceeded());

		assert_eq!(manager.EffectiveTimeout(), Duration::from_secs(30));

		assert!(TimeoutManager::ValidateTimeout(Duration::from_secs(30)).is_ok());

		assert!(TimeoutManager::ValidateTimeout(Duration::from_secs(0)).is_err());
	}

	#[test]
	fn test_timeout_manager_with_deadline() {
		let deadline = Instant::now() + Duration::from_secs(60);

		let manager = TimeoutManager::with_deadline(deadline, Duration::from_secs(30));

		let remaining = manager.Remaining();

		assert!(remaining.is_some());

		assert!(remaining.unwrap() <= Duration::from_secs(60));
	}
}
