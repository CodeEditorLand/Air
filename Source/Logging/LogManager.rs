//! Log file manager for rotation and cleanup.

use std::{path::{Path, PathBuf},sync::{Arc, Mutex},time::{SystemTime, UNIX_EPOCH}};
use crate::{Result, dev_log};
use crate::Logging::LogRotationConfig::LogRotationConfig;
/// Log file manager for rotation and cleanup
pub struct LogManager {
	Config:LogRotationConfig,

	CurrentFile:Arc<Mutex<Option<PathBuf>>>,

	CurrentSize:Arc<Mutex<u64>>,
}

impl LogManager {
	fn new(Config:LogRotationConfig) -> Result<Self> {
		Config.Validate()?;

		// Ensure log directory exists
		std::fs::create_dir_all(&Config.LogDirectory)?;

		Ok(Self {
			Config,
			CurrentFile:Arc::new(Mutex::new(None)),
			CurrentSize:Arc::new(Mutex::new(0)),
		})
	}

	/// Check if log rotation is needed
	fn ShouldRotate(&self) -> bool {
		let size = *self.CurrentSize.lock().unwrap_or_else(|e| e.into_inner());

		size >= self.Config.MaxFileSizeBytes
	}

	/// Perform log rotation
	fn Rotate(&self) -> Result<()> {
		let CurrentFile = self.CurrentFile.lock().unwrap_or_else(|e| e.into_inner());

		if let Some(ref FilePath) = *CurrentFile {
			// Rename current file with timestamp
			let Timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

			let RotatedPath = format!("{}.{}.log", FilePath.display(), Timestamp);

			std::fs::rename(FilePath, &RotatedPath)?;

			// Compress if enabled
			if self.Config.Compress {
				self.CompressFile(&RotatedPath)?;
			}

			// Cleanup old log files
			self.CleanupOldLogs()?;
		}

		*self.CurrentSize.lock().unwrap_or_else(|e| e.into_inner()) = 0;

		Ok(())
	}

	/// Compress a log file
	fn CompressFile(&self, path:&str) -> crate::Result<()> {
		// Basic compression - in production would use actual compression
		let _ = path;

		Ok(())
	}

	/// Cleanup old log files
	fn CleanupOldLogs(&self) -> Result<()> {
		let log_dir = Path::new(&self.Config.LogDirectory);

		if !log_dir.exists() {
			return Ok(());
		}

		let mut log_files:Vec<_> = std::fs::read_dir(log_dir)?
			.filter_map(|e| e.ok())
			.filter(|e| {
				e.path()
					.extension()
					.and_then(|s| s.to_str())
					.map(|ext| ext == "log")
					.unwrap_or(false)
			})
			.collect();

		// Sort by modification time (newest first)
		log_files.sort_by(|a, b| {
			let a_time = a.metadata().and_then(|m| m.modified()).unwrap_or(UNIX_EPOCH);

			let b_time = b.metadata().and_then(|m| m.modified()).unwrap_or(UNIX_EPOCH);

			b_time.cmp(&a_time)
		});

		// Keep only max_files
		for file in log_files.into_iter().skip(self.Config.MaxFiles) {
			let _ = std::fs::remove_file(file.path());
		}

		Ok(())
	}
}
