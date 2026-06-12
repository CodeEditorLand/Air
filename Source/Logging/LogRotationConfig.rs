//! Log rotation configuration and strategies.
//! Controls automatic log file rotation with size-based and time-based policies.

use std::{collections::HashMap,path::{Path, PathBuf},sync::{Arc, Mutex},time::{SystemTime, UNIX_EPOCH}};
use serde::{Deserialize, Serialize};
use tracing_appender::rolling::Rotation;
use crate::{Result, dev_log};
/// Configuration for log rotation and management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
	/// Maximum size of a single log file in bytes before rotation
	pub MaxFileSizeBytes:u64,

	/// Maximum number of rotated log files to retain
	pub MaxFiles:usize,

	/// Rotation strategy (daily, hourly, never)
	pub Rotation:LogRotation,

	/// Whether to compress rotated log files
	pub Compress:bool,

	/// Log directory path
	pub LogDirectory:String,

	/// Log file name prefix
	pub LogFilePrefix:String,
}

/// Log rotation strategies
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LogRotation {
	/// Rotate daily
	Daily,

	/// Rotate every hour
	Hourly,

	/// Rotate every minute (for debugging)
	Minutely,

	/// Never rotate automatically
	Never,
}

impl Default for LogRotation {
	fn default() -> Self { Self::Daily }
}

impl Default for LogRotationConfig {
	fn default() -> Self {
		Self {
			MaxFileSizeBytes:100 * 1024 * 1024, // 100 MB

			MaxFiles:30, // Keep 30 days of logs

			Rotation:LogRotation::Daily,

			Compress:true,

			LogDirectory:"./Log".to_string(),

			LogFilePrefix:"Air".to_string(),
		}
	}
}

impl LogRotationConfig {
	/// Validate log rotation configuration
	pub fn Validate(&self) -> Result<()> {
		if self.MaxFileSizeBytes == 0 {
			return Err("MaxFileSizeBytes must be greater than 0".into());
		}

		if self.MaxFileSizeBytes > 10 * 1024 * 1024 * 1024 {
			// Max 10 GB
			return Err("MaxFileSizeBytes cannot exceed 10 GB".into());
		}

		if self.MaxFiles == 0 {
			return Err("MaxFiles must be greater than 0".into());
		}

		if self.MaxFiles > 365 {
			// Max 1 year retention
			return Err("MaxFiles cannot exceed 365".into());
		}

		Ok(())
	}

	/// Convert to tracing_appender Rotation
	pub fn ToTracingRotation(&self) -> Rotation {
		match self.Rotation {
			LogRotation::Daily => Rotation::DAILY,

			LogRotation::Hourly => Rotation::HOURLY,

			LogRotation::Minutely => Rotation::NEVER, // No minutely support

			LogRotation::Never => Rotation::NEVER,
		}
	}
}
