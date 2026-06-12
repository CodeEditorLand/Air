use serde::{Deserialize, Serialize};

use super::HealthCheckLevel::HealthCheckLevel;
use super::HealthStatus::HealthStatus;

/// Service health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
	/// Service name
	pub ServiceName:String,

	/// Current health status
	pub Status:HealthStatus,

	/// Last check timestamp
	pub LastCheck:u64,

	/// Last successful check timestamp
	pub LastSuccess:Option<u64>,

	/// Failure count
	pub FailureCount:u32,

	/// Error message (if any)
	pub ErrorMessage:Option<String>,

	/// Response time in milliseconds
	pub ResponseTimeMs:Option<u64>,

	/// Health check level
	pub CheckLevel:HealthCheckLevel,
}
