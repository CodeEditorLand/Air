use crate::daemon::Platform;

/// Platform-specific daemon information
#[derive(Debug)]
pub struct PlatformInfo {
	/// Platform type
	pub Platform:Platform,

	/// Service name for system integration
	pub ServiceName:String,

	/// User under which daemon runs
	pub RunAsUser:Option<String>,
}
