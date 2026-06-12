//! Rollback state for a single version backup.
//!
//! Captures the metadata needed to restore a previous version:
//! version string, backup path, timestamp, and integrity checksum.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackState {
	pub version:String,

	pub backup_path:PathBuf,

	pub timestamp:chrono::DateTime<chrono::Utc>,

	pub checksum:String,
}
