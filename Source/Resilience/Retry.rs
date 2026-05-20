#![allow(non_snake_case, unused_variables, dead_code, unused_imports)]

//! Exponential-backoff retry logic with jitter and budget management.
//!
//! Three cooperating types:
//! - `ErrorClass` - classifies errors so `RetryManager` can pick the right
//!   delay strategy (transient, rate-limited, server error, non-retryable).
//! - `RetryPolicy` - configurable max-attempts, intervals, backoff multiplier,
//!   jitter factor, and a per-service call budget.
//! - `RetryManager` - applies the policy: computes delays, tracks per-service
//!   budgets, classifies errors, and publishes `RetryEvent` to a broadcast
//!   channel for telemetry subscribers.

use std::{
	collections::HashMap,
	sync::Arc,
	time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, broadcast};

use crate::dev_log;

/// Error classification for adaptive retry policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorClass {
	/// Transient: network timeouts, temporary failures.
	Transient,
	/// Non-retryable: authentication errors, invalid requests.
	NonRetryable,
	/// Rate-limited: 429 Too Many Requests.
	RateLimited,
	/// Server errors: 500-599.
	ServerError,
	/// Unrecognised error pattern.
	Unknown,
}

/// Retry policy configuration - controls all delay and budget parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
	pub MaxRetries:u32,
	pub InitialIntervalMs:u64,
	pub MaxIntervalMs:u64,
	pub BackoffMultiplier:f64,
	/// Jitter fraction 0-1 added on top of the base delay.
	pub JitterFactor:f64,
	pub BudgetPerMinute:u32,
	pub ErrorClassification:HashMap<String, ErrorClass>,
}

impl Default for RetryPolicy {
	fn default() -> Self {
		let mut ErrorClassification = HashMap::new();
		ErrorClassification.insert("timeout".to_string(), ErrorClass::Transient);
		ErrorClassification.insert("connection_refused".to_string(), ErrorClass::Transient);
		ErrorClassification.insert("connection_reset".to_string(), ErrorClass::Transient);
		ErrorClassification.insert("rate_limit_exceeded".to_string(), ErrorClass::RateLimited);
		ErrorClassification.insert("authentication_failed".to_string(), ErrorClass::NonRetryable);
		ErrorClassification.insert("unauthorized".to_string(), ErrorClass::NonRetryable);
		ErrorClassification.insert("not_found".to_string(), ErrorClass::NonRetryable);
		ErrorClassification.insert("server_error".to_string(), ErrorClass::ServerError);
		ErrorClassification.insert("internal_server_error".to_string(), ErrorClass::ServerError);
		ErrorClassification.insert("service_unavailable".to_string(), ErrorClass::ServerError);
		ErrorClassification.insert("gateway_timeout".to_string(), ErrorClass::Transient);
		Self {
			MaxRetries:3,
			InitialIntervalMs:1000,
			MaxIntervalMs:32000,
			BackoffMultiplier:2.0,
			JitterFactor:0.1,
			BudgetPerMinute:100,
			ErrorClassification,
		}
	}
}

/// Per-service retry budget: tracks attempt timestamps and enforces the
/// calls-per-minute cap from `RetryPolicy::BudgetPerMinute`.
#[derive(Debug, Clone)]
struct RetryBudget {
	Attempts:Vec<Instant>,
	MaxPerMinute:u32,
}

impl RetryBudget {
	fn new(MaxPerMinute:u32) -> Self { Self { Attempts:Vec::new(), MaxPerMinute } }

	fn can_retry(&mut self) -> bool {
		let Now = Instant::now();
		let OneMinuteAgo = Now - Duration::from_secs(60);
		self.Attempts.retain(|&attempt| attempt > OneMinuteAgo);
		if self.Attempts.len() < self.MaxPerMinute as usize {
			self.Attempts.push(Now);
			true
		} else {
			false
		}
	}
}

/// Telemetry event published after every retry attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryEvent {
	pub Service:String,
	pub Attempt:u32,
	pub ErrorClass:ErrorClass,
	pub DelayMs:u64,
	pub Success:bool,
	pub ErrorMessage:Option<String>,
}

/// Applies `RetryPolicy`: computes delays, tracks budgets per service,
/// classifies errors, and publishes `RetryEvent`s.
pub struct RetryManager {
	Policy:RetryPolicy,
	Budgets:Arc<Mutex<HashMap<String, RetryBudget>>>,
	EventTx:Arc<broadcast::Sender<RetryEvent>>,
}

impl RetryManager {
	pub fn new(policy:RetryPolicy) -> Self {
		let (EventTx, _) = broadcast::channel(1000);
		Self {
			Policy:policy,
			Budgets:Arc::new(Mutex::new(HashMap::new())),
			EventTx:Arc::new(EventTx),
		}
	}

	pub fn GetEventTransmitter(&self) -> broadcast::Sender<RetryEvent> { (*self.EventTx).clone() }

	/// Exponential backoff with jitter: `base * multiplier^(attempt-1) +
	/// jitter`.
	pub fn CalculateRetryDelay(&self, Attempt:u32) -> Duration {
		if Attempt == 0 {
			return Duration::from_millis(0);
		}
		let BaseDelay = (self.Policy.InitialIntervalMs as f64 * self.Policy.BackoffMultiplier.powi(Attempt as i32 - 1))
			.min(self.Policy.MaxIntervalMs as f64) as u64;
		let Jitter = (BaseDelay as f64 * self.Policy.JitterFactor) as u64;
		let RandomJitter = (rand::random::<f64>() * Jitter as f64) as u64;
		Duration::from_millis(BaseDelay + RandomJitter)
	}

	/// Choose delay strategy based on classified error type.
	pub fn CalculateAdaptiveRetryDelay(&self, ErrorType:&str, attempt:u32) -> Duration {
		let Class = self
			.Policy
			.ErrorClassification
			.get(ErrorType)
			.copied()
			.unwrap_or(ErrorClass::Unknown);
		match Class {
			ErrorClass::RateLimited => Duration::from_millis(((attempt + 1) * 5000) as u64),
			ErrorClass::ServerError => {
				let BaseDelay = self.Policy.InitialIntervalMs * 2_u64.pow(attempt);
				Duration::from_millis(BaseDelay.min(self.Policy.MaxIntervalMs))
			},
			ErrorClass::Transient => self.CalculateRetryDelay(attempt),
			ErrorClass::NonRetryable | ErrorClass::Unknown => Duration::from_millis(100),
		}
	}

	pub fn ClassifyError(&self, ErrorMessage:&str) -> ErrorClass {
		let Lower = ErrorMessage.to_lowercase();
		for (pattern, class) in &self.Policy.ErrorClassification {
			if Lower.contains(pattern) {
				return *class;
			}
		}
		ErrorClass::Unknown
	}

	pub async fn CanRetry(&self, service:&str) -> bool {
		let mut budgets = self.Budgets.lock().await;
		let budget = budgets
			.entry(service.to_string())
			.or_insert_with(|| RetryBudget::new(self.Policy.BudgetPerMinute));
		budget.can_retry()
	}

	pub fn PublishRetryEvent(&self, event:RetryEvent) { let _ = self.EventTx.send(event); }

	pub fn ValidatePolicy(&self) -> Result<(), String> {
		if self.Policy.MaxRetries == 0 {
			return Err("MaxRetries must be greater than 0".to_string());
		}
		if self.Policy.InitialIntervalMs == 0 {
			return Err("InitialIntervalMs must be greater than 0".to_string());
		}
		if self.Policy.InitialIntervalMs > self.Policy.MaxIntervalMs {
			return Err("InitialIntervalMs cannot be greater than MaxIntervalMs".to_string());
		}
		if self.Policy.BackoffMultiplier <= 1.0 {
			return Err("BackoffMultiplier must be greater than 1.0".to_string());
		}
		if self.Policy.JitterFactor < 0.0 || self.Policy.JitterFactor > 1.0 {
			return Err("JitterFactor must be between 0 and 1".to_string());
		}
		if self.Policy.BudgetPerMinute == 0 {
			return Err("BudgetPerMinute must be greater than 0".to_string());
		}
		Ok(())
	}
}
