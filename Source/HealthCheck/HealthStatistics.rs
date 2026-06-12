use serde::{Deserialize, Serialize};

/// Health statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatistics {
	pub TotalServices:usize,

	pub HealthyServices:usize,

	pub DegradedServices:usize,

	pub UnhealthyServices:usize,

	pub TotalChecks:usize,

	pub AverageResponseTimeMs:f64,

	pub SuccessRate:f64,
}

impl HealthStatistics {
	/// Get overall health percentage
	pub fn OverallHealthPercentage(&self) -> f64 {
		if self.TotalServices == 0 {
			return 0.0;
		}

		(self.HealthyServices as f64 / self.TotalServices as f64) * 100.0
	}
}
