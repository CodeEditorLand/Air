//! Rollback history for automatic and manual rollback.
//!
//! Maintains a bounded list of previous version backups with automatic
//! cleanup of old entries.

use serde::{Deserialize, Serialize};

use super::RollbackState::RollbackState;

/// Rollback history for automatic and manual rollback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RollbackHistory {
	/// Previous versions available for rollback
	pub(super) versions:Vec<RollbackState>,

	/// Maximum number of versions to keep
	pub(super) max_versions:usize,
}
