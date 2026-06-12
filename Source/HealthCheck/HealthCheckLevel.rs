use serde::{Deserialize, Serialize};

/// Health check level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthCheckLevel {
	/// Basic liveness check
	Alive,

	/// Service responds to requests
	Responsive,

	/// Service performs its core function
	Functional,
}
