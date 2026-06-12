//! Plugin sandbox manager: creates, retrieves, and removes sandboxes, and checks sandbox state.

use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::Plugins::PluginSandboxConfig::PluginSandboxConfig;
use crate::Result;

/// Plugin sandbox manager
pub struct PluginSandboxManager {
	sandboxes:Arc<RwLock<HashMap<String, PluginSandboxConfig>>>,
}

impl PluginSandboxManager {
	/// Create a new sandbox manager
	pub fn new() -> Self { Self { sandboxes:Arc::new(RwLock::new(HashMap::new())) } }

	/// Create a sandbox for a plugin
	pub async fn create_sandbox(&self, plugin_id:String, config:PluginSandboxConfig) -> Result<()> {
		let mut sandboxes = self.sandboxes.write().await;

		sandboxes.insert(plugin_id, config);

		Ok(())
	}

	/// Get sandbox configuration
	pub async fn get_sandbox(&self, plugin_id:&str) -> Option<PluginSandboxConfig> {
		let sandboxes = self.sandboxes.read().await;

		sandboxes.get(plugin_id).cloned()
	}

	/// Remove a sandbox
	pub async fn remove_sandbox(&self, plugin_id:&str) {
		let mut sandboxes = self.sandboxes.write().await;

		sandboxes.remove(plugin_id);
	}

	/// Check if a plugin is running in a sandbox
	pub async fn is_sandboxed(&self, plugin_id:&str) -> bool {
		let sandboxes = self.sandboxes.read().await;

		sandboxes.get(plugin_id).map_or(false, |s| s.enabled)
	}
}

impl Default for PluginSandboxManager {
	fn default() -> Self { Self::new() }
}
