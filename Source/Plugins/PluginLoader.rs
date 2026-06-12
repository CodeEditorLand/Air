//! Plugin loader: discovers plugins from configured paths and loads them from
//! discovery results.

use std::sync::Arc;

use crate::{
	AirError,
	Plugins::{Plugin::Plugin, PluginDiscoveryResult::PluginDiscoveryResult},
	Result,
	dev_log,
};

/// Plugin loader for discovering and loading plugins
pub struct PluginLoader {
	PluginPaths:Vec<String>,
}

impl PluginLoader {
	/// Create a new plugin loader
	pub fn new() -> Self {
		Self {
			PluginPaths:vec![
				"/usr/local/lib/Air/plugins".to_string(),
				"~/.local/share/Air/plugins".to_string(),
			],
		}
	}

	/// Add a plugin discovery path
	pub fn add_path(&mut self, path:String) { self.PluginPaths.push(path); }

	/// Discover plugins from all configured paths
	pub async fn discover_all(&self) -> Result<Vec<PluginDiscoveryResult>> {
		let mut results = vec![];

		for path in &self.PluginPaths {
			match self.discover_in_path(path).await {
				Ok(mut discovered) => {
					results.append(&mut discovered);
				},

				Err(e) => {
					dev_log!(
						"extensions",
						"warn: [PluginLoader] Failed to discover plugins in {}: {}",
						path,
						e
					);
				},
			}
		}

		Ok(results)
	}

	/// Discover plugins in a specific path
	pub async fn discover_in_path(&self, path:&str) -> Result<Vec<PluginDiscoveryResult>> {
		let Results = vec![];

		// In production, this would scan the directory for plugin manifests
		// For now, we return an empty list
		dev_log!("extensions", "[PluginLoader] Discovering plugins in: {}", path);

		Ok(Results)
	}

	/// Load a plugin from a discovery result
	pub async fn load_from_discovery(&self, discovery:&PluginDiscoveryResult) -> Result<Arc<Box<dyn Plugin>>> {
		// In production, this would load the plugin from the manifest
		// For now, we return an error
		Err(AirError::Plugin(format!(
			"Plugin loading not yet implemented: {}",
			discovery.plugin_id
		)))
	}
}

impl Default for PluginLoader {
	fn default() -> Self { Self::new() }
}
