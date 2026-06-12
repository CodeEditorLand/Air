//! Download session for resumable downloads.
//!
//! Tracks the state of an in-progress download including bytes downloaded,
//! total size, and cancellation status for graceful resume capability.

use std::path::PathBuf;

/// Download session for resumable downloads
#[derive(Debug, Clone)]
pub(super) struct DownloadSession {
	/// Session unique identifier
	pub(super) session_id:String,

	/// Original update URL
	pub(super) download_url:String,

	/// Current file path
	pub(super) temp_path:PathBuf,

	/// Bytes downloaded so far
	pub(super) downloaded_bytes:u64,

	/// Total file size
	pub(super) total_bytes:u64,

	/// Whether download is complete
	pub(super) complete:bool,

	/// Cancellation flag for download
	pub(super) cancelled:bool,
}
