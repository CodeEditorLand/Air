//! Plugin lifecycle hooks: on_load, on_start, on_stop, on_unload,
//! on_config_changed.

use async_trait::async_trait;

use crate::Result;

/// Plugin lifecycle hooks
#[async_trait]
pub trait PluginHooks: Send + Sync {
	/// Called when plugin is being loaded
	async fn on_load(&self) -> Result<()> { Ok(()) }

	/// Called when plugin is starting
	async fn on_start(&self) -> Result<()> { Ok(()) }

	/// Called when plugin is stopping
	async fn on_stop(&self) -> Result<()> { Ok(()) }

	/// Called when plugin is being unloaded
	async fn on_unload(&self) -> Result<()> { Ok(()) }

	/// Called when configuration changes
	async fn on_config_changed(&self, _old:&serde_json::Value, _new:&serde_json::Value) -> Result<()> { Ok(()) }
}
