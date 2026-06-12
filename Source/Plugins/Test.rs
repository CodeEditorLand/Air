//! Unit tests for the Plugins module.

#[cfg(test)]
mod tests {

	use std::sync::Arc;

	use async_trait::async_trait;

	use crate::Plugins::{
		ApiVersion::ApiVersion,
		Plugin::Plugin,
		PluginHooks::PluginHooks,
		PluginManager::PluginManager,
		PluginMessage::PluginMessage,
		PluginMetadata::PluginMetadata,
		PluginSandboxConfig::PluginSandboxConfig,
		PluginState::PluginState,
		PluginValidationResult::PluginValidationResult,
	};

	struct TestPlugin;

	// Use Box::leak to create static metadata
	fn test_metadata() -> &'static PluginMetadata {
		Box::leak(Box::new(PluginMetadata {
			id:"test".to_string(),
			name:"Test Plugin".to_string(),
			version:"1.0.0".to_string(),
			description:"A test plugin".to_string(),
			author:"Test".to_string(),
			MinAirVersion:"0.1.0".to_string(),
			MaxAirVersion:None,
			dependencies:vec![],
			capabilities:vec![],
		}))
	}

	#[async_trait]
	impl PluginHooks for TestPlugin {}

	#[async_trait]
	impl Plugin for TestPlugin {
		fn metadata(&self) -> &PluginMetadata { test_metadata() }
	}

	#[tokio::test]
	async fn test_plugin_manager_creation() {
		let manager = PluginManager::new("0.1.0".to_string());

		let plugins = manager.list_plugins().await.unwrap();

		assert!(plugins.is_empty());
	}

	#[tokio::test]
	async fn test_plugin_registration() {
		let manager = PluginManager::new("0.1.0".to_string());

		let plugin = Arc::new(Box::new(TestPlugin) as Box<dyn Plugin>);

		let result = manager.register(plugin.clone()).await;

		assert!(result.is_ok());

		let plugins = manager.list_plugins().await.unwrap();

		assert_eq!(plugins.len(), 1);

		assert_eq!(plugins[0].id, "test");
	}

	#[tokio::test]
	async fn test_plugin_lifecycle() {
		let manager = PluginManager::new("0.1.0".to_string());

		let plugin = Arc::new(Box::new(TestPlugin) as Box<dyn Plugin>);

		manager.register(plugin.clone()).await.unwrap();

		// Start the plugin
		let result = manager.start("test").await;

		assert!(result.is_ok());

		// Check state
		let plugins = manager.list_plugins().await.unwrap();

		assert_eq!(plugins[0].state, PluginState::Running);

		// Stop the plugin
		let result = manager.stop("test").await;

		assert!(result.is_ok());

		// Check state
		let plugins = manager.list_plugins().await.unwrap();

		assert_eq!(plugins[0].state, PluginState::Loaded);
	}

	#[tokio::test]
	async fn test_version_satisfaction() {
		let manager = PluginManager::new("1.0.0".to_string());

		assert!(manager.version_satisfies("1.0.0", "0.1.0"));

		assert!(manager.version_satisfies("1.2.0", "1.0.0"));

		assert!(manager.version_satisfies("1.0.5", "1.0.0"));

		assert!(!manager.version_satisfies("0.9.0", "1.0.0"));
	}

	#[tokio::test]
	async fn test_plugin_message_validation() {
		let message = PluginMessage::new(
			"sender".to_string(),
			"receiver".to_string(),
			"action".to_string(),
			serde_json::json!({}),
		);

		assert!(message.validate().is_ok());
	}

	#[tokio::test]
	async fn test_api_version_compatibility() {
		let v1 = ApiVersion { major:1, minor:0, patch:0, PreRelease:None };

		let v2 = ApiVersion { major:1, minor:1, patch:0, PreRelease:None };

		let v3 = ApiVersion { major:2, minor:0, patch:0, PreRelease:None };

		assert!(v1.IsCompatible(&v2));

		assert!(!v1.IsCompatible(&v3));
	}

	#[tokio::test]
	async fn test_sandbox_config_default() {
		let config = PluginSandboxConfig::default();

		assert!(config.enabled);

		assert_eq!(config.MaxMemoryMb, Some(128));

		assert!(!config.NetworkAllowed);

		assert!(!config.FilesystemAllowed);
	}

	#[tokio::test]
	async fn test_plugin_metadata_validation() {
		let manager = PluginManager::new("1.0.0".to_string());

		// Directly reference TestPlugin to avoid trait bound issues
		let result = manager.validate_plugin(&TestPlugin);

		assert!(matches!(result, PluginValidationResult::Valid));

		// Verify the TestPlugin metadata can be accessed
		let metadata = test_metadata();

		assert_eq!(metadata.id, "test");

		assert_eq!(metadata.name, "Test Plugin");

		assert_eq!(metadata.version, "1.0.0");

		assert_eq!(metadata.author, "Test");

		assert_eq!(metadata.description, "A test plugin");
	}
}
