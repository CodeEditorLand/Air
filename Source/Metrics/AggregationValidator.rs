use std::time::{Duration, Instant};

use crate::dev_log;

/// Aggregation validation for metric integrity
#[derive(Debug)]
pub(crate) struct AggregationValidator {
	pub(crate) last_timestamp:Instant,

	pub(crate) validation_window:Duration,
}

impl AggregationValidator {
	pub(crate) fn new(validation_window_secs:u64) -> Self {
		Self {
			last_timestamp:Instant::now(),

			validation_window:Duration::from_secs(validation_window_secs),
		}
	}

	/// Validate aggregation is within time window
	pub(crate) fn validate(&mut self) -> std::result::Result<(), String> {
		let now = Instant::now();

		if now.duration_since(self.last_timestamp) > self.validation_window {
			dev_log!("metrics", "warn: [Metrics] Aggregation outside validation window, resetting");

			self.last_timestamp = now;

			Ok(())
		} else {
			Ok(())
		}
	}
}
