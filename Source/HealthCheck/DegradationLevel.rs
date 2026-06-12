use serde::{Deserialize, Serialize};

/// Degradation levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DegradationLevel {
	Optimal,

	Acceptable,

	Degraded,

	Critical,
}
