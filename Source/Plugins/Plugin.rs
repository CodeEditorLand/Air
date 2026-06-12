//! Plugin interface trait: extends PluginHooks with metadata, sandbox,
//! permissions, message handling, state, and capability checks.

use async_trait::async_trait;

use crate::{
	AirError,
	Plugins::{
		PluginHooks::PluginHooks,
		PluginMessage::PluginMessage,
		PluginMetadata::PluginMetadata,
		PluginPermission::PluginPermission,
		PluginSandboxConfig::PluginSandboxConfig,
	},
	Result,
};

/// Plugin interface trait
#[async_trait]
pub trait Plugin: PluginHooks + Send + Sync {
	/// Get plugin metadata
	fn metadata(&self) -> &PluginMetadata;

	/// Get plugin sandbox configuration
	fn sandbox_config(&self) -> PluginSandboxConfig { PluginSandboxConfig::default() }

	/// Get plugin permissions
	fn permissions(&self) -> Vec<PluginPermission> { vec![] }

	/// Handle inter-plugin message
	async fn Message(&self, from:&str, _message:&PluginMessage) -> Result<PluginMessage> {
		Err(AirError::Plugin(format!("Plugin {} does not handle messages", from)))
	}

	/// Get plugin state for diagnostics
	async fn get_state(&self) -> Result<serde_json::Value> { Ok(serde_json::json!({})) }

	/// Check if plugin has specific capability
	fn has_capability(&self, _capability:&str) -> bool { false }

	/// Check if plugin has specific permission
	fn has_permission(&self, _permission:&PluginPermission) -> bool { false }
}
