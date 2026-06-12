use serde::{Deserialize, Serialize};

/// Health status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
	/// Service is healthy
	Healthy,

	/// Service is degraded but functional
	Degraded,

	/// Service is unhealthy
	Unhealthy,

	/// Service is unknown/unchecked
	Unknown,
}
