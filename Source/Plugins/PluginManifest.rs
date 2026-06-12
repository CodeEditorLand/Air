//! Plugin manifest: plugin metadata, main entry point, and optional sandbox
//! config.

use serde::{Deserialize, Serialize};

use crate::Plugins::{PluginMetadata::PluginMetadata, PluginSandboxConfig::PluginSandboxConfig};

/// Plugin manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
	pub plugin:PluginMetadata,

	pub main:String,

	pub sandbox:Option<PluginSandboxConfig>,
}
