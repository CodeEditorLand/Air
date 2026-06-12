//! Update status with comprehensive state tracking.
//!
//! Provides a complete snapshot of the current update lifecycle including
//! progress, ETA, download speed, and any error messages.

use serde::{Deserialize, Serialize};

use super::{InstallationStatus::InstallationStatus, UpdateChannel::UpdateChannel};

/// Update status with comprehensive state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatus {
	/// Last time updates were checked
	pub last_check:Option<chrono::DateTime<chrono::Utc>>,

	/// Whether an update is available
	pub update_available:bool,

	/// Current installed version
	pub current_version:String,

	/// Available version (if any)
	pub available_version:Option<String>,

	/// Download progress (0.0 to 100.0)
	pub download_progress:Option<f32>,

	/// Current installation status
	pub installation_status:InstallationStatus,

	/// Update channel being used
	pub update_channel:UpdateChannel,

	/// Size of available update in bytes
	pub update_size:Option<u64>,

	/// Release notes for available update
	pub release_notes:Option<String>,

	/// Whether update requires restart
	pub requires_restart:bool,

	/// Download speed in bytes per second
	pub download_speed:Option<f64>,

	/// Estimated time remaining in seconds
	pub eta_seconds:Option<u64>,

	/// Last error message (if any)
	pub last_error:Option<String>,
}
