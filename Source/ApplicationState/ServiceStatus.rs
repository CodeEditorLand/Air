use serde::{Deserialize, Serialize};

/// Service status enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceStatus {
	Starting,

	Running,

	Stopping,

	Stopped,

	Error(String),
}
