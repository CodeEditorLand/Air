//! Plugin discovery result: plugin_id, manifest path, metadata, and enabled flag.

use serde::{Deserialize, Serialize};

use crate::Plugins::PluginMetadata::PluginMetadata;

/// Plugin discovery result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDiscoveryResult {
	pub plugin_id:String,

	pub ManifestPath:String,

	pub metadata:PluginMetadata,

	pub enabled:bool,
}
