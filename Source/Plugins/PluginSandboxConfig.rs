//! Plugin sandbox configuration: resource limits and access controls.

use serde::{Deserialize, Serialize};

/// Plugin sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSandboxConfig {
	pub enabled:bool,

	pub MaxMemoryMb:Option<u64>,

	pub MaxCPUPercent:Option<f64>,

	pub NetworkAllowed:bool,

	pub FilesystemAllowed:bool,

	pub AllowedPaths:Vec<String>,

	pub TimeoutSecs:Option<u64>,
}

impl Default for PluginSandboxConfig {
	fn default() -> Self {
		Self {
			enabled:true,

			MaxMemoryMb:Some(128),

			MaxCPUPercent:Some(10.0),

			NetworkAllowed:false,

			FilesystemAllowed:false,

			AllowedPaths:vec![],

			TimeoutSecs:Some(30),
		}
	}
}
