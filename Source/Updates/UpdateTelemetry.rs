//! Update telemetry data for analytics.
//!
//! Records metadata about each update operation (check, download, install,
//! rollback) including success/failure, duration, and optional error messages.

use serde::{Deserialize, Serialize};

/// Update telemetry data for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTelemetry {
	/// Unique telemetry event ID
	pub event_id: String,

	/// Current version
	pub current_version: String,

	/// Target version
	pub target_version: String,

	/// Update channel
	pub channel: String,

	/// Platform identifier
	pub platform: String,

	/// Operation type (check, download, install, rollback)
	pub operation: String,

	/// Success or failure
	pub success: bool,

	/// Duration in milliseconds
	pub duration_ms: u64,

	/// Download size in bytes
	pub download_size: Option<u64>,

	/// Error message (if failed)
	pub error_message: Option<String>,

	/// Timestamp of the event
	pub timestamp: chrono::DateTime<chrono::Utc>,
}
