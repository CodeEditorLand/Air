use serde::{Deserialize, Serialize};

use super::DegradationLevel::DegradationLevel;

/// Performance degradation indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceIndicators {
	pub ResponseTimeP99Ms:f64,

	pub ResponseTimeP95Ms:f64,

	pub RequestThroughputPerSec:f64,

	pub ErrorRatePercent:f64,

	pub DegradationLevel:DegradationLevel,

	pub BottleneckService:Option<String>,
}

impl Default for PerformanceIndicators {
	fn default() -> Self {
		Self {
			ResponseTimeP99Ms:0.0,

			ResponseTimeP95Ms:0.0,

			RequestThroughputPerSec:0.0,

			ErrorRatePercent:0.0,

			DegradationLevel:DegradationLevel::Optimal,

			BottleneckService:None,
		}
	}
}
