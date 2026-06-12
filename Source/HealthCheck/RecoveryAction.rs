use serde::{Deserialize, Serialize};

use super::RecoveryActionType::RecoveryActionType;
use super::RecoveryTrigger::RecoveryTrigger;

/// Recovery action configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
	/// Action name
	pub Name:String,

	/// Service name
	pub ServiceName:String,

	/// Trigger condition
	pub Trigger:RecoveryTrigger,

	/// Action to take
	pub Action:RecoveryActionType,

	/// Maximum retry attempts
	pub MaxRetries:u32,

	/// Current retry count
	pub RetryCount:u32,
}
