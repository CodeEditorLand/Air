use serde::{Deserialize, Serialize};

/// Recovery trigger conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryTrigger {
	/// Trigger after N consecutive failures
	ConsecutiveFailures(u32),

	/// Trigger when response time exceeds threshold
	ResponseTimeExceeds(u64),

	/// Trigger when service becomes unresponsive
	ServiceUnresponsive,
}
