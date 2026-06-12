use serde::{Deserialize, Serialize};

/// Recovery action types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryActionType {
	/// Restart the service
	RestartService,

	/// Reset connection
	ResetConnection,

	/// Clear cache
	ClearCache,

	/// Reload configuration
	ReloadConfiguration,

	/// Escalate to higher level
	Escalate,
}
