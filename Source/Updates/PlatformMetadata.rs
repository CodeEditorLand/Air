//! Platform-specific update metadata.
//!
//! Contains platform-dependent fields for an update package including
//! the package format, installation instructions, disk requirements,
//! and privilege needs.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Platform-specific update metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformMetadata {
	/// Package format (exe, dmg, appimage, etc.)
	pub package_format: String,

	/// Installation instructions
	pub install_instructions: Vec<String>,

	/// Required disk space in bytes
	pub required_disk_space: u64,

	/// Whether admin privileges are required
	pub requires_admin: bool,

	/// Additional platform-specific parameters
	pub additional_params: HashMap<String, serde_json::Value>,
}
