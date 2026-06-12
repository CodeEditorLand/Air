use crate::Daemon::Platform::Platform;

/// Daemon status information
#[derive(Debug, Clone)]
pub struct DaemonStatus {
	pub IsRunning:bool,

	pub PidFileExists:bool,

	pub Pid:Option<u32>,

	pub Platform:Platform,

	pub ServiceName:String,

	pub ShutdownRequested:bool,
}

impl DaemonStatus {
	/// Get human-readable status description
	pub fn status_description(&self) -> String {
		if self.IsRunning {
			format!("Running (PID: {})", self.Pid.unwrap_or(0))
		} else if self.PidFileExists {
			"Stale PID file exists".to_string()
		} else {
			"Not running".to_string()
		}
	}
}
