use serde::{Deserialize, Serialize};

use super::HealthStatus::HealthStatus;

/// Health check record for history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckRecord {
	/// Timestamp
	pub Timestamp:u64,

	/// Service name
	pub ServiceName:String,

	/// Health status
	pub Status:HealthStatus,

	/// Response time in milliseconds
	pub ResponseTimeMs:Option<u64>,

	/// Error message (if any)
	pub ErrorMessage:Option<String>,
}
