use serde::{Deserialize, Serialize};

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
	pub TotalRequest:u64,

	pub SuccessfulRequest:u64,

	pub FailedRequest:u64,

	pub AverageResponseTime:f64,

	pub UptimeSeconds:u64,

	pub LastUpdated:u64,
}
