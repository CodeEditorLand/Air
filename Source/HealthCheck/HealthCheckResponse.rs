use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::Utility;
use super::{
	HealthStatistics::HealthStatistics,
	HealthStatus::HealthStatus,
	PerformanceIndicators::PerformanceIndicators,
	ResourceWarning::ResourceWarning,
	ServiceHealth::ServiceHealth,
};

/// Health check response for gRPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
	pub OverallStatus:HealthStatus,

	pub ServiceHealth:HashMap<String, ServiceHealth>,

	pub Statistics:HealthStatistics,

	pub PerformanceIndicators:PerformanceIndicators,

	pub ResourceWarnings:Vec<ResourceWarning>,

	pub Timestamp:u64,
}

impl HealthCheckResponse {
	/// Create a new health check response
	pub fn new(
		OverallStatus:HealthStatus,

		ServiceHealth:HashMap<String, ServiceHealth>,

		Statistics:HealthStatistics,
	) -> Self {
		Self {
			OverallStatus,

			ServiceHealth,

			Statistics,

			PerformanceIndicators:PerformanceIndicators::default(),

			ResourceWarnings:Vec::new(),

			Timestamp:Utility::CurrentTimestamp(),
		}
	}

	/// Create with performance indicators
	pub fn with_performance_indicators(mut self, indicators:PerformanceIndicators) -> Self {
		self.PerformanceIndicators = indicators;

		self
	}

	/// Create with resource warnings
	pub fn with_resource_warnings(mut self, warnings:Vec<ResourceWarning>) -> Self {
		self.ResourceWarnings = warnings;

		self
	}
}
