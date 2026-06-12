use crate::dev_log;

/// Overflow-protected metric update helper
pub(crate) struct MetricGuard {
	pub(crate) current:u64,

	pub(crate) max:u64,
}

impl MetricGuard {
	pub(crate) fn new(current:u64, max:u64) -> Self { Self { current, max } }

	/// Increment with overflow protection
	pub(crate) fn increment(&mut self) -> bool {
		if self.current < self.max.saturating_sub(1) {
			self.current += 1;

			true
		} else {
			dev_log!("metrics", "warn: [Metrics] Metric overflow detected, wrapping around");

			self.current = 0;

			true
		}
	}
}
