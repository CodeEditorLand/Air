//! Circuit breaker states.
//!
//! Three-state model (`Closed`, `Open`, `HalfOpen`) used by `CircuitBreaker`
//! to isolate failing services and test recovery.

use serde::{Deserialize, Serialize};

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
