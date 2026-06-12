//! Circuit breaker configuration.
//!
//! Holds the thresholds and timeout that control when the circuit trips,
//! when recovery is attempted, and when it can close again.

use serde::{Deserialize, Serialize};

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
