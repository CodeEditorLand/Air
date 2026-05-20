#![allow(non_snake_case, unused_variables, dead_code, unused_imports)]

//! Timeout management with cascading deadlines.
//!
//! `TimeoutManager` tracks a per-operation timeout and an optional global
//! deadline. `effective_timeout()` returns the minimum of the two so every
//! nested operation respects both the local and cascade budgets.
//!
//! All public methods have panic-safe variants (`Remaining`, `EffectiveTimeout`,
//! `IsExceeded`) that catch panics via `catch_unwind` and return conservative
//! fallback values so a transient panic never propagates into the caller.

use std::time::{Duration, Instant};

use crate::dev_log;

/// Timeout manager with optional cascading global deadline.
pub struct TimeoutManager {
	global_deadline:Option<Instant>,
	operation_timeout:Duration,
}

impl TimeoutManager {
	/// Create with an operation-scoped timeout and no global deadline.
	pub fn new(operation_timeout:Duration) -> Self {
		Self { global_deadline:None, operation_timeout }
	}

	/// Create with both a global deadline and an operation timeout.
	pub fn with_deadline(global_deadline:Instant, operation_timeout:Duration) -> Self {
		Self { global_deadline:Some(global_deadline), operation_timeout }
	}

	/// Return an error if `timeout` is zero or exceeds one hour.
	pub fn ValidateTimeout(timeout:Duration) -> Result<(), String> {
		if timeout.is_zero() {
			return Err("Timeout must be greater than 0".to_string());
		}
		if timeout.as_secs() > 3600 {
			return Err("Timeout cannot exceed 1 hour".to_string());
		}
		Ok(())
	}

	/// Return `Ok(timeout)` or an error string; used by fallback paths.
	pub fn ValidateTimeoutResult(timeout:Duration) -> Result<Duration, String> {
		if timeout.is_zero() {
			return Err("Timeout must be greater than 0".to_string());
		}
		if timeout.as_secs() > 3600 {
			return Err("Timeout cannot exceed 1 hour".to_string());
		}
		Ok(timeout)
	}

	/// Time remaining until the global deadline, or `None` if none is set.
	pub fn remaining(&self) -> Option<Duration> {
		self.global_deadline.map(|deadline| {
			deadline
				.checked_duration_since(Instant::now())
				.unwrap_or(Duration::from_secs(0))
		})
	}

	/// Panic-safe `remaining()`. Returns `None` on panic (fail-open).
	pub fn Remaining(&self) -> Option<Duration> {
		std::panic::catch_unwind(|| self.remaining()).unwrap_or_else(|e| {
			dev_log!("resilience", "error: [TimeoutManager] Panic in Remaining: {:?}", e);
			None
		})
	}

	/// Minimum of `operation_timeout` and remaining deadline time.
	pub fn effective_timeout(&self) -> Duration {
		match self.remaining() {
			Some(remaining) => self.operation_timeout.min(remaining),
			None => self.operation_timeout,
		}
	}

	/// Panic-safe `effective_timeout()`. Falls back to 30 s on invalid/panic.
	pub fn EffectiveTimeout(&self) -> Duration {
		std::panic::catch_unwind(|| {
			let timeout = self.effective_timeout();
			match Self::ValidateTimeoutResult(timeout) {
				Ok(valid_timeout) => valid_timeout,
				Err(_) => Duration::from_secs(30),
			}
		})
		.unwrap_or_else(|e| {
			dev_log!("resilience", "error: [TimeoutManager] Panic in EffectiveTimeout: {:?}", e);
			Duration::from_secs(30)
		})
	}

	/// `true` when the global deadline has passed.
	pub fn is_exceeded(&self) -> bool {
		self.global_deadline.map_or(false, |deadline| Instant::now() >= deadline)
	}

	/// Panic-safe `is_exceeded()`. Returns `true` on panic (fail-safe).
	pub fn IsExceeded(&self) -> bool {
		std::panic::catch_unwind(|| self.is_exceeded()).unwrap_or_else(|e| {
			dev_log!("resilience", "error: [TimeoutManager] Panic in IsExceeded: {:?}", e);
			true
		})
	}

	pub fn GetGlobalDeadline(&self) -> Option<Instant> { self.global_deadline }

	pub fn GetOperationTimeout(&self) -> Duration { self.operation_timeout }
}
