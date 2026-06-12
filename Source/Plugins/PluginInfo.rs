//! Plugin information for listing: id, metadata, state, uptime, error.

use serde::{Deserialize, Serialize};

use crate::Plugins::PluginMetadata::PluginMetadata;
use crate::Plugins::PluginState::PluginState;

/// Plugin information for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
	pub id:String,

	pub metadata:PluginMetadata,

	pub state:PluginState,

	pub UptimeSecs:u64,

	pub error:Option<String>,
}
