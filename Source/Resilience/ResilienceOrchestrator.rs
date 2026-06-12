//! Resilience orchestrator combining all resilience patterns.
//!
//! Owns a `RetryManager`, a set of `CircuitBreaker`s, and a set of
//! `BulkheadExecutor`s, all keyed by service name. `ExecuteResilient`
//! is the main entry point — it validates configuration, checks the
//! circuit, acquires a bulkhead permit, and retries with adaptive delay.

use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio::sync::RwLock;

use crate::dev_log;
use super::{
	BulkheadConfig::BulkheadConfig,
	BulkheadExecutor::BulkheadExecutor,
	BulkheadStatistics::BulkheadStatistics,
	CircuitBreaker::CircuitBreaker,
	CircuitBreakerConfig::CircuitBreakerConfig,
	CircuitState::CircuitState,
	CircuitStatistics::CircuitStatistics,
	Retry::{ErrorClass, RetryEvent, RetryManager, RetryPolicy},
	Timeout::TimeoutManager,
};

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
