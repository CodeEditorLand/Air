#![allow(non_snake_case, unused_variables, dead_code, unused_imports)]

//! Value types for the download lifecycle: state, priority, queue entries,
//! results, statistics, and per-download configuration.
//!
//! These structs are the data contract between `DownloadManager` methods,
//! IPC callers (Cocoon VSIX installs, Mountain status queries), and tests.

use std::{path::PathBuf, sync::Arc, time::Duration};

use serde::{Deserialize, Serialize};

/// Fine-grained state of a single download.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DownloadState {
	Pending,
	Queued,
	Downloading,
	Verifying,
	Completed,
	Failed,
	Cancelled,
	Paused,
	Resuming,
}

/// Scheduling priority for the download queue.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DownloadPriority {
	High = 3,
	Normal = 2,
	Low = 1,
	Background = 0,
}

/// Live status snapshot for one active download.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadStatus {
	pub DownloadId:String,
	pub url:String,
	pub destination:PathBuf,
	pub TotalSize:u64,
	pub downloaded:u64,
	pub progress:f32,
	pub status:DownloadState,
	pub error:Option<String>,
	pub StartedAt:Option<chrono::DateTime<chrono::Utc>>,
	pub CompletedAt:Option<chrono::DateTime<chrono::Utc>>,
	pub ChunksCompleted:usize,
	pub TotalChunks:usize,
	pub DownloadRateBytesPerSec:u64,
	pub ExpectedChecksum:Option<String>,
	pub ActualChecksum:Option<String>,
}

/// Entry in the priority download queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedDownload {
	pub DownloadId:String,
	pub url:String,
	pub destination:PathBuf,
	pub checksum:String,
	pub priority:DownloadPriority,
	pub AddedAt:chrono::DateTime<chrono::Utc>,
	pub MaxFileSize:Option<u64>,
	pub ValidateDiskSpace:bool,
}

/// Final outcome of a completed download.
#[derive(Debug, Clone)]
pub struct DownloadResult {
	pub path:String,
	pub size:u64,
	pub checksum:String,
	pub duration:Duration,
	pub AverageRate:u64,
}

/// Aggregate statistics across all downloads in this session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadStatistics {
	pub TotalDownloads:u64,
	pub SuccessfulDownloads:u64,
	pub FailedDownloads:u64,
	pub CancelledDownloads:u64,
	pub TotalBytesDownloaded:u64,
	pub TotalDownloadTimeSecs:f64,
	pub AverageDownloadRate:f64,
	pub PeakDownloadRate:u64,
	pub ActiveDownloads:usize,
	pub QueuedDownloads:usize,
}

/// Type alias for progress callbacks registered with a download.
pub type ProgressCallback = Arc<dyn Fn(DownloadStatus) + Send + Sync>;

/// Per-download configuration including URL, destination, checksum, and limits.
#[derive(Debug, Clone)]
pub struct DownloadConfig {
	pub url:String,
	pub destination:String,
	pub checksum:String,
	pub MaxFileSize:Option<u64>,
	pub ChunkSize:usize,
	pub MaxRetries:u32,
	pub TimeoutSecs:u64,
	pub priority:DownloadPriority,
	pub ValidateDiskSpace:bool,
}

impl Default for DownloadConfig {
	fn default() -> Self {
		Self {
			url:String::new(),
			destination:String::new(),
			checksum:String::new(),
			MaxFileSize:None,
			ChunkSize:1024 * 1024, // 1 MB
			MaxRetries:3,
			TimeoutSecs:300,
			priority:DownloadPriority::Normal,
			ValidateDiskSpace:true,
		}
	}
}
