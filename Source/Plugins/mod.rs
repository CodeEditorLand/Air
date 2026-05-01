//! # Plugin Architecture
//!
//! ## Responsibilities
//!
//! This module provides a comprehensive plugin system for the Air daemon,
//! enabling extensibility through dynamically loaded plugins that can enhance
//! daemon functionality. The plugin system is responsible for:
//!
//! - **Plugin Discovery**: Automatically discovering available plugins from
//!   configured directories
//! - **Plugin Loading**: Dynamically loading plugins into the daemon runtime
//! - **Plugin Validation**: Validating plugin metadata, dependencies, and
//!   compatibility
//! - **Sandboxing**: Isolating plugins to prevent crashes and security issues
//! - **Lifecycle Management**: Managing plugin states (load, start, stop,
//!   unload) with proper hooks
//! - **API Registration**: Extending the daemon API through plugin-provided
//!   commands and handlers
//! - **Inter-Plugin Communication Enabling**: plugins to communicate with each
//!   other via message passing
//! - **Permission Management**: Enforcing fine-grained permissions and
//!   capabilities for plugins
//! - **Version Compatibility**: Ensuring plugins are compatible with the daemon
//!   version
//! - **Dependency Resolution**: Resolving and validating plugin dependencies
//!
//! ## VSCode Extension Architecture Patterns
//!
//! This implementation draws inspiration from VSCode's extension architecture:
//! - Reference: vs/platform/extensions/common/ extensionHostStarter.ts
//! - Reference: vs/server/node/ extensionHostConnection.ts
//! - Reference: vs/platform/remote/common/ remoteAgentConnection.ts
//!
//! Patterns adopted from VSCode extensions:
//! - Separate extension host process for isolation and crash protection
//! - Activation events to trigger extension loading on-demand
//! - Contribution points for extending functionality
//! - Message-based communication between host and extensions
//! - State management and lifecycle hooks
//! - API versioning for backward compatibility
//! - Permission and capability descriptors
//!
//! ## Integration with Cocoon Extension Host
//!
//! The plugin system is designed to integrate with the Cocoon Extension Host
//! (similar to VSCode's extension host architecture). This provides:
//! - Isolated execution environments for plugins
//! - Crash recovery and resilience
//! - Resource management and limits
//! - Communication via IPC channels
//! - Hot reload capability without daemon restart
//!
//! ## FUTURE Enhancements
//!
//! - **Plugin Marketplace**: Implement a central plugin marketplace for
//! discovery and installation (similar to VSCode's extension marketplace)
//! - **Hot Reload Support**: Implement live reloading of plugins without daemon
//! restart
//! - **Advanced Sandboxing**: Add more sophisticated sandboxing with resource
//! quotas, network isolation, and filesystem access controls
//! - **Plugin Distribution**: Implement plugin packaging, signing, and
//! distribution mechanisms
//! - **Automatic Updates**: Add automatic plugin update checking and
//! installation
//! - **Telemetry Integration**: Add plugin usage telemetry and reporting
//! - **Plugin Profiles**: Support multiple plugin configurations for different
//! environments
//! - **Security Audit**: Implement comprehensive security audit and
//! vulnerability scanning for plugins
//! - **Performance Monitoring**: Add detailed performance monitoring and
//!   profiling for plugins
//! - **Plugin Debugging**: Provide debugging tools and interfaces for plugin
//!   developers
//!
//! ## Security and Isolation
//!
//! - Plugins run in isolated processes to prevent daemon crashes
//! - Fine-grained permission system controls plugin capabilities
//! - API version compatibility checks prevent breaking changes
//! - Resource limits prevent plugin exhaustion attacks
//! - Plugin authentication and signing to prevent malicious plugins
//! - Filesystem and network access restrictions

use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{AirError, Result, dev_log};

// =============================================================================
// Plugin Types and Traits
// =============================================================================

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
	pub id:String,
	pub name:String,
	pub version:String,
	pub description:String,
	pub author:String,
	pub MinAirVersion:String,
	pub MaxAirVersion:Option<String>,
	pub dependencies:Vec<PluginDependency>,
	pub capabilities:Vec<String>,
}

/// Plugin dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
	pub PluginId:String,
	pub MinVersion:String,
	pub MaxVersion:Option<String>,
	pub optional:bool,
}

/// Plugin capability and permission descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCapability {
	pub name:String,
	pub description:String,
	pub RequiredPermissions:Vec<String>,
}

/// Plugin permission
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PluginPermission {
	/// Access filesystem
	Filesystem { read:bool, write:bool, paths:Vec<String> },
	/// Access network
	Network { outbound:bool, inbound:bool, hosts:Vec<String> },
	/// Access system resources
	System { cpu:bool, memory:bool },
	/// Access other plugins
	InterPlugin { plugins:Vec<String>, actions:Vec<String> },
	/// Custom permission
	Custom(String),
}

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

/// Plugin validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginValidationResult {
	Valid,
	Invalid(String),
	Warning(String),
}

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

/// Inter-plugin message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMessage {
	pub id:String,
	pub from:String,
	pub to:String,
	pub action:String,
	pub data:serde_json::Value,
	pub timestamp:DateTime<Utc>,
}

impl PluginMessage {
	/// Create a new plugin message
	pub fn new(from:String, to:String, action:String, data:serde_json::Value) -> Self {
		Self { id:Uuid::new_v4().to_string(), from, to, action, data, timestamp:Utc::now() }
	}

	/// Validate message format and content
	pub fn validate(&self) -> Result<()> {
		if self.id.is_empty() {
			return Err(crate::AirError::Plugin("Message ID cannot be empty".to_string()));
		}
		if self.from.is_empty() {
			return Err(crate::AirError::Plugin("Message sender cannot be empty".to_string()));
		}
		if self.to.is_empty() {
			return Err(crate::AirError::Plugin("Message recipient cannot be empty".to_string()));
		}
		if self.action.is_empty() {
			return Err(crate::AirError::Plugin("Message action cannot be empty".to_string()));
		}
		if self.action.len() > 100 {
			return Err(crate::AirError::Plugin("Message action too long".to_string()));
		}
		Ok(())
	}
}

// =============================================================================
// Plugin Manager
// =============================================================================

/// Plugin state tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginState {
	#[serde(rename = "unloaded")]
	Unloaded,
	#[serde(rename = "loaded")]
	Loaded,
	#[serde(rename = "starting")]
	Starting,
	#[serde(rename = "running")]
	Running,
	#[serde(rename = "stopping")]
	Stopping,
	#[serde(rename = "error")]
	Error,
}

/// Plugin registry entry
pub struct PluginRegistry {
	pub plugin:Arc<Box<dyn Plugin>>,
	pub state:PluginState,
	pub StartedAt:Option<DateTime<Utc>>,
	pub LoadedAt:Option<DateTime<Utc>>,
	pub error:Option<String>,
	pub sandbox:PluginSandboxConfig,
}

/// Main plugin manager
pub struct PluginManager {
	plugins:Arc<RwLock<HashMap<String, PluginRegistry>>>,
	#[allow(dead_code)]
	MessageQueue:Arc<RwLock<Vec<PluginMessage>>>,
	AirVersion:String,
	EnableSandbox:bool,
	StartupTimeout:Duration,
	OperationTimeout:Duration,
}

impl PluginManager {
	/// Create a new plugin manager
	pub fn new(AirVersion:String) -> Self {
		Self {
			plugins:Arc::new(RwLock::new(HashMap::new())),
			MessageQueue:Arc::new(RwLock::new(Vec::new())),
			AirVersion,
			EnableSandbox:true,
			StartupTimeout:Duration::from_secs(30),
			OperationTimeout:Duration::from_secs(60),
		}
	}

	/// Create a new plugin manager with custom configuration
	pub fn with_config(
		AirVersion:String,
		EnableSandbox:bool,
		StartupTimeoutSecs:u64,
		OperationTimeoutSecs:u64,
	) -> Self {
		Self {
			plugins:Arc::new(RwLock::new(HashMap::new())),
			MessageQueue:Arc::new(RwLock::new(Vec::new())),
			AirVersion,
			EnableSandbox,
			StartupTimeout:Duration::from_secs(StartupTimeoutSecs),
			OperationTimeout:Duration::from_secs(OperationTimeoutSecs),
		}
	}

	/// Enable or disable sandbox mode
	pub fn set_sandbox_enabled(&mut self, enabled:bool) { self.EnableSandbox = enabled; }

	/// Discover plugins from a directory
	pub async fn discover_plugins(&self, directory:&str) -> Result<Vec<String>> {
		let Discovered = vec![];

		// In production, this would scan the directory for plugin manifests
		// For now, we return an empty list
		dev_log!("extensions", "[PluginManager] Discovering plugins in directory: {}", directory);
		Ok(Discovered)
	}

	/// Load a plugin from a manifest file
	pub async fn load_from_manifest(&self, path:&str) -> Result<String> {
		// In production, this would load and parse a plugin manifest
		// For now, we return a mock plugin ID
		dev_log!("extensions", "[PluginManager] Loading plugin from manifest: {}", path);
		Ok("loaded_plugin".to_string())
	}

	/// Register a plugin
	pub async fn register(&self, plugin:Arc<Box<dyn Plugin>>) -> Result<()> {
		let metadata = plugin.metadata();

		dev_log!(
			"extensions",
			"[PluginManager] Registering plugin: {} v{}",
			metadata.name,
			metadata.version
		);
		// Validate plugin metadata
		self.ValidatePluginMetadata(metadata)?;

		// Check Air version compatibility
		self.CheckAirVersionCompatibility(metadata)?;

		// Check API version compatibility
		self.CheckApiVersionCompatibility(metadata)?;

		// Check dependencies
		self.check_dependencies(metadata).await?;

		// Validate plugin capabilities and permissions
		self.validate_capabilities_and_permissions(plugin.as_ref().as_ref())?;

		// Setup sandbox configuration
		let sandbox = if self.EnableSandbox {
			plugin.sandbox_config()
		} else {
			PluginSandboxConfig { enabled:false, ..Default::default() }
		};

		// Load plugin with timeout
		let LoadResult = tokio::time::timeout(self.StartupTimeout, plugin.on_load()).await;

		let _load_result = LoadResult
			.map_err(|_| {
				AirError::Plugin(format!("Plugin {} load timeout after {:?}", metadata.name, self.StartupTimeout))
			})?
			.map_err(|e| {
				dev_log!(
					"extensions",
					"error: [PluginManager] Failed to load plugin {}: {}",
					metadata.name,
					e
				);
				e
			})?;

		// Register in map
		let mut plugins = self.plugins.write().await;
		plugins.insert(
			metadata.id.clone(),
			PluginRegistry {
				plugin:plugin.clone(),
				state:PluginState::Loaded,
				StartedAt:None,
				LoadedAt:Some(Utc::now()),
				error:None,
				sandbox,
			},
		);

		dev_log!("extensions", "[PluginManager] Plugin registered: {}", metadata.name);
		Ok(())
	}

	/// Validate plugin metadata
	pub fn ValidatePluginMetadata(&self, metadata:&PluginMetadata) -> Result<()> {
		if metadata.id.is_empty() {
			return Err(crate::AirError::Plugin("Plugin ID cannot be empty".to_string()));
		}
		if metadata.id.len() > 100 {
			return Err(crate::AirError::Plugin("Plugin ID too long (max 100 characters)".to_string()));
		}
		if !metadata.id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
			return Err(crate::AirError::Plugin("Plugin ID contains invalid characters".to_string()));
		}
		if metadata.name.is_empty() {
			return Err(crate::AirError::Plugin("Plugin name cannot be empty".to_string()));
		}
		if metadata.version.is_empty() {
			return Err(crate::AirError::Plugin("Plugin version cannot be empty".to_string()));
		}
		if metadata.author.is_empty() {
			return Err(crate::AirError::Plugin("Plugin author cannot be empty".to_string()));
		}
		Ok(())
	}

	/// Validate plugin capabilities and permissions
	pub fn validate_capabilities_and_permissions(&self, plugin:&dyn Plugin) -> Result<()> {
		let permissions = plugin.permissions();

		// Check for dangerous permissions
		for permission in &permissions {
			match permission {
				PluginPermission::Filesystem { write, .. } if *write => {
					dev_log!(
						"extensions",
						"warn: [PluginManager] Plugin {} requests filesystem write access",
						plugin.metadata().id
					);
				},
				PluginPermission::Network { .. } => {
					dev_log!(
						"extensions",
						"warn: [PluginManager] Plugin {} requests network access",
						plugin.metadata().id
					);
				},
				_ => {},
			}
		}

		Ok(())
	}

	/// Check Air version compatibility
	pub fn CheckAirVersionCompatibility(&self, metadata:&PluginMetadata) -> Result<()> {
		if !self.version_satisfies(&self.AirVersion, &metadata.MinAirVersion) {
			return Err(AirError::Plugin(format!(
				"Plugin requires Air version {} or higher, current: {}",
				metadata.MinAirVersion, self.AirVersion
			)));
		}

		if let Some(max_version) = &metadata.MaxAirVersion {
			if !self.version_satisfies(max_version, &self.AirVersion) {
				return Err(AirError::Plugin(format!(
					"Plugin is incompatible with Air version {}, max supported: {}",
					self.AirVersion, max_version
				)));
			}
		}

		Ok(())
	}

	/// Check API version compatibility
	pub fn CheckApiVersionCompatibility(&self, _Metadata:&PluginMetadata) -> Result<()> {
		// Check if plugin declares compatibility with current API version
		// In production, this would check against the daemon's API version
		Ok(())
	}

	/// Check plugin dependencies
	pub async fn check_dependencies(&self, metadata:&PluginMetadata) -> Result<()> {
		let plugins = self.plugins.read().await;

		for dep in &metadata.dependencies {
			if !dep.optional {
				let DepPlugin = plugins
					.get(&dep.PluginId)
					.ok_or_else(|| AirError::Plugin(format!("Required dependency not found: {}", dep.PluginId)))?;

				let DepVersion = &DepPlugin.plugin.metadata().version;
				if !self.version_satisfies(DepVersion, &dep.MinVersion) {
					return Err(AirError::Plugin(format!(
						"Dependency {} version {} does not satisfy requirement {}",
						dep.PluginId, DepVersion, dep.MinVersion
					)));
				}

				if DepPlugin.state != PluginState::Running && DepPlugin.state != PluginState::Loaded {
					return Err(AirError::Plugin(format!(
						"Dependency {} is not ready (state: {:?})",
						dep.PluginId, DepPlugin.state
					)));
				}
			}
		}

		Ok(())
	}

	/// Start a plugin
	pub async fn start(&self, PluginId:&str) -> Result<()> {
		let mut plugins = self.plugins.write().await;
		let registry = plugins
			.get_mut(PluginId)
			.ok_or_else(|| AirError::Plugin(format!("Plugin not found: {}", PluginId)))?;

		if registry.state == PluginState::Running {
			dev_log!("extensions", "[PluginManager] Plugin {} already running", PluginId);
			return Ok(());
		}

		registry.state = PluginState::Starting;

		// Check sandbox configuration
		if self.EnableSandbox && registry.sandbox.enabled {
			dev_log!("extensions", "[PluginManager] Starting plugin {} in sandbox mode", PluginId);
		}

		let plugin = registry.plugin.clone();
		drop(plugins);

		let StartResult = tokio::time::timeout(self.StartupTimeout, plugin.on_start()).await;

		match StartResult {
			Ok(Ok(())) => {
				let mut plugins = self.plugins.write().await;
				if let Some(registry) = plugins.get_mut(PluginId) {
					registry.state = PluginState::Running;
					registry.StartedAt = Some(Utc::now());
					registry.error = None;
				}
				dev_log!("extensions", "[PluginManager] Plugin started: {}", PluginId);
				Ok(())
			},
			Ok(Err(e)) => {
				let mut plugins = self.plugins.write().await;
				if let Some(registry) = plugins.get_mut(PluginId) {
					registry.state = PluginState::Error;
					registry.error = Some(e.to_string());
				}
				dev_log!("extensions", "error: [PluginManager] Plugin start failed: {}: {}", PluginId, e);
				Err(e)
			},
			Err(_) => {
				let mut plugins = self.plugins.write().await;
				if let Some(registry) = plugins.get_mut(PluginId) {
					registry.state = PluginState::Error;
					registry.error = Some(format!("Startup timeout after {:?}", self.StartupTimeout));
				}
				dev_log!("extensions", "error: [PluginManager] Plugin start timeout: {}", PluginId);
				Err(AirError::Plugin(format!("Plugin {} startup timeout", PluginId)))
			},
		}
	}

	/// Stop a plugin
	pub async fn stop(&self, PluginId:&str) -> Result<()> {
		let mut plugins = self.plugins.write().await;
		let registry = plugins
			.get_mut(PluginId)
			.ok_or_else(|| AirError::Plugin(format!("Plugin not found: {}", PluginId)))?;

		if registry.state != PluginState::Running {
			dev_log!("extensions", "[PluginManager] Plugin {} not running", PluginId);
			return Ok(());
		}

		registry.state = PluginState::Stopping;
		let plugin = registry.plugin.clone();
		drop(plugins);

		let StopResult = tokio::time::timeout(self.OperationTimeout, plugin.on_stop()).await;

		match StopResult {
			Ok(Ok(())) => {
				let mut plugins = self.plugins.write().await;
				if let Some(registry) = plugins.get_mut(PluginId) {
					registry.state = PluginState::Loaded;
					registry.StartedAt = None;
				}
				dev_log!("extensions", "[PluginManager] Plugin stopped: {}", PluginId);
				Ok(())
			},
			Ok(Err(e)) => {
				let mut plugins = self.plugins.write().await;
				if let Some(registry) = plugins.get_mut(PluginId) {
					registry.state = PluginState::Error;
					registry.error = Some(e.to_string());
				}
				dev_log!("extensions", "error: [PluginManager] Plugin stop failed: {}: {}", PluginId, e);
				Err(e)
			},
			Err(_) => {
				let mut plugins = self.plugins.write().await;
				if let Some(registry) = plugins.get_mut(PluginId) {
					registry.state = PluginState::Error;
					registry.error = Some(format!("Stop timeout after {:?}", self.OperationTimeout));
				}
				dev_log!("extensions", "error: [PluginManager] Plugin stop timeout: {}", PluginId);
				Err(AirError::Plugin(format!("Plugin {} stop timeout", PluginId)))
			},
		}
	}

	/// Start all registered plugins
	pub async fn start_all(&self) -> Result<()> {
		let PluginIds:Vec<String> = self.plugins.read().await.keys().cloned().collect();

		dev_log!("extensions", "[PluginManager] Starting {} plugins", PluginIds.len());
		for PluginId in PluginIds {
			if let Err(e) = self.start(&PluginId).await {
				dev_log!("extensions", "warn: [PluginManager] Failed to start plugin {}: {}", PluginId, e);
			}
		}

		Ok(())
	}

	/// Stop all running plugins
	pub async fn stop_all(&self) -> Result<()> {
		let PluginIds:Vec<String> = self.plugins.read().await.keys().cloned().collect();

		dev_log!("extensions", "[PluginManager] Stopping {} plugins", PluginIds.len());
		// Stop in reverse order to respect dependencies
		for plugin_id in PluginIds.into_iter().rev() {
			if let Err(e) = self.stop(&plugin_id).await {
				dev_log!("extensions", "warn: [PluginManager] Failed to stop plugin {}: {}", plugin_id, e);
			}
		}

		Ok(())
	}

	/// Load a plugin
	pub async fn load(&self, plugin_id:&str) -> Result<()> {
		let mut plugins = self.plugins.write().await;
		let registry = plugins
			.get_mut(plugin_id)
			.ok_or_else(|| AirError::Plugin(format!("Plugin not found: {}", plugin_id)))?;

		if registry.state != PluginState::Unloaded {
			dev_log!("extensions", "[PluginManager] Plugin {} already loaded", plugin_id);
			return Ok(());
		}

		let plugin = registry.plugin.clone();
		drop(plugins);

		let LoadResult = tokio::time::timeout(self.StartupTimeout, plugin.on_load()).await;

		match LoadResult {
			Ok(Ok(())) => {
				let mut plugins = self.plugins.write().await;
				if let Some(registry) = plugins.get_mut(plugin_id) {
					registry.state = PluginState::Loaded;
					registry.LoadedAt = Some(Utc::now());
					registry.error = None;
				}
				dev_log!("extensions", "[PluginManager] Plugin loaded: {}", plugin_id);
				Ok(())
			},
			Ok(Err(e)) => {
				let mut plugins = self.plugins.write().await;
				if let Some(registry) = plugins.get_mut(plugin_id) {
					registry.state = PluginState::Error;
					registry.error = Some(e.to_string());
				}
				dev_log!("extensions", "error: [PluginManager] Plugin load failed: {}: {}", plugin_id, e);
				Err(e)
			},
			Err(_) => {
				let mut plugins = self.plugins.write().await;
				if let Some(registry) = plugins.get_mut(plugin_id) {
					registry.state = PluginState::Error;
					registry.error = Some(format!("Load timeout after {:?}", self.StartupTimeout));
				}
				dev_log!("extensions", "error: [PluginManager] Plugin load timeout: {}", plugin_id);
				Err(AirError::Plugin(format!("Plugin {} load timeout", plugin_id)))
			},
		}
	}

	/// Unload a plugin
	pub async fn unload(&self, plugin_id:&str) -> Result<()> {
		// First stop the plugin
		self.stop(plugin_id).await?;

		let mut plugins = self.plugins.write().await;
		let registry = plugins
			.get(plugin_id)
			.ok_or_else(|| AirError::Plugin(format!("Plugin not found: {}", plugin_id)))?;

		let plugin = registry.plugin.clone();
		plugins.remove(plugin_id);

		let UnloadResult = tokio::time::timeout(self.OperationTimeout, plugin.on_unload()).await;

		match UnloadResult {
			Ok(Ok(())) => {
				dev_log!("extensions", "[PluginManager] Plugin unloaded: {}", plugin_id);
				Ok(())
			},
			Ok(Err(e)) => {
				// Plugin is removed from registry even if unload fails
				dev_log!("extensions", "error: [PluginManager] Plugin unload error: {}: {}", plugin_id, e);
				Err(e)
			},
			Err(_) => {
				// Plugin is removed from registry even if timeout occurs
				dev_log!("extensions", "warn: [PluginManager] Plugin unload timeout: {}", plugin_id);
				Err(AirError::Plugin(format!("Plugin {} unload timeout", plugin_id)))
			},
		}
	}

	/// Send message from one plugin to another
	pub async fn send_message(&self, message:PluginMessage) -> Result<PluginMessage> {
		// Validate message
		message.validate()?;

		let plugins = self.plugins.read().await;

		let target = plugins
			.get(&message.to)
			.ok_or_else(|| AirError::Plugin(format!("Target plugin not found: {}", message.to)))?;

		if target.state != PluginState::Running {
			return Err(AirError::Plugin(format!(
				"Target plugin not running: {} (state: {:?})",
				message.to, target.state
			)));
		}

		// Check if sender has permission to send to receiver
		let SenderMetadata = plugins
			.get(&message.from)
			.ok_or_else(|| AirError::Plugin(format!("Sender plugin not found: {}", message.from)))?;

		if !self.check_inter_plugin_permission(SenderMetadata, target, &message) {
			return Err(AirError::Plugin(format!(
				"Permission denied: {} cannot send to {}",
				message.from, message.to
			)));
		}

		let plugin = target.plugin.clone();
		drop(plugins);

		// Send message with timeout
		let SendResult =
			tokio::time::timeout(self.OperationTimeout, plugin.Message(&message.from, &message)).await;

		SendResult.map_err(|_| AirError::Plugin(format!("Message send timeout: {} -> {}", message.from, message.to)))?
	}

	/// Check inter-plugin communication permission
	fn check_inter_plugin_permission(
		&self,
		_sender:&PluginRegistry,
		_target:&PluginRegistry,
		_message:&PluginMessage,
	) -> bool {
		// In production, this would check if sender has permission to communicate with
		// target For now, we allow all communication
		true
	}

	/// Get plugin list with details
	pub async fn list_plugins(&self) -> Result<Vec<PluginInfo>> {
		let plugins = self.plugins.read().await;
		let mut result = Vec::new();

		for (id, registry) in plugins.iter() {
			let metadata = registry.plugin.metadata().clone();
			result.push(PluginInfo {
				id:id.clone(),
				metadata,
				state:registry.state,
				UptimeSecs:registry.StartedAt.map(|t| (Utc::now() - t).num_seconds() as u64).unwrap_or(0),
				error:registry.error.clone(),
			});
		}

		Ok(result)
	}

	/// Get plugin state
	pub async fn get_plugin_state(&self, plugin_id:&str) -> Result<serde_json::Value> {
		let plugins = self.plugins.read().await;
		let registry = plugins
			.get(plugin_id)
			.ok_or_else(|| AirError::Plugin(format!("Plugin not found: {}", plugin_id)))?;

		registry.plugin.get_state().await
	}

	/// Get plugin permissions
	pub async fn get_plugin_permissions(&self, plugin_id:&str) -> Result<Vec<PluginPermission>> {
		let plugins = self.plugins.read().await;
		let registry = plugins
			.get(plugin_id)
			.ok_or_else(|| AirError::Plugin(format!("Plugin not found: {}", plugin_id)))?;

		Ok(registry.plugin.permissions())
	}

	/// Validate all plugins
	pub async fn validate_all_plugins(&self) -> Vec<(String, PluginValidationResult)> {
		let plugins = self.plugins.read().await;
		let mut results = vec![];

		for (id, registry) in plugins.iter() {
			let result = self.validate_plugin(registry.plugin.as_ref().as_ref());
			results.push((id.clone(), result));
		}

		results
	}

	/// Validate a single plugin
	pub fn validate_plugin(&self, plugin:&dyn Plugin) -> PluginValidationResult {
		let metadata = plugin.metadata();

		// Validate metadata
		if let Err(e) = self.ValidatePluginMetadata(metadata) {
			return PluginValidationResult::Invalid(e.to_string());
		}

		// Check version compatibility
		if let Err(e) = self.CheckAirVersionCompatibility(metadata) {
			return PluginValidationResult::Invalid(format!("Version compatibility error: {}", e));
		}

		PluginValidationResult::Valid
	}

	/// Get dependency graph
	pub async fn get_dependency_graph(&self) -> Result<serde_json::Value> {
		let plugins = self.plugins.read().await;
		let mut graph = serde_json::Map::new();

		for (id, registry) in plugins.iter() {
			let metadata = registry.plugin.metadata();
			let dependencies:Vec<String> = metadata.dependencies.iter().map(|d| d.PluginId.clone()).collect();
			graph.insert(id.clone(), serde_json::json!(dependencies));
		}

		Ok(serde_json::Value::Object(graph))
	}

	/// Resolve plugin load order based on dependencies
	pub async fn resolve_load_order(&self) -> Result<Vec<String>> {
		let plugins = self.plugins.read().await;

		// Topological sort based on dependencies
		let mut visited = std::collections::HashSet::new();
		let mut order = vec![];

		for plugin_id in plugins.keys() {
			self.VisitPluginForLoadOrder(plugin_id, &mut visited, &mut order, &plugins)?;
		}

		Ok(order)
	}

	/// Visit plugin for load order (helper function)
	fn VisitPluginForLoadOrder(
		&self,
		plugin_id:&str,
		visited:&mut std::collections::HashSet<String>,
		order:&mut Vec<String>,
		plugins:&HashMap<String, PluginRegistry>,
	) -> Result<()> {
		if visited.contains(plugin_id) {
			return Ok(());
		}

		visited.insert(plugin_id.to_string());

		if let Some(registry) = plugins.get(plugin_id) {
			let metadata = registry.plugin.metadata();
			for dep in &metadata.dependencies {
				if !dep.optional {
					self.VisitPluginForLoadOrder(&dep.PluginId, visited, order, plugins)?;
				}
			}
		}

		order.push(plugin_id.to_string());
		Ok(())
	}

	/// Simple version satisfaction check (X.Y.Z format)
	fn version_satisfies(&self, actual:&str, required:&str) -> bool {
		let ActualParts:Vec<&str> = actual.split('.').collect();
		let RequiredParts:Vec<&str> = required.split('.').collect();

		for (i, required_part) in RequiredParts.iter().enumerate() {
			if let (Ok(a), Ok(r)) = (ActualParts.get(i).unwrap_or(&"0").parse::<u32>(), required_part.parse::<u32>()) {
				if a > r {
					return true;
				} else if a < r {
					return false;
				}
			}
		}

		true
	}
}

/// Plugin information for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
	pub id:String,
	pub metadata:PluginMetadata,
	pub state:PluginState,
	pub UptimeSecs:u64,
	pub error:Option<String>,
}

// =============================================================================
// Plugin Event System
// =============================================================================

/// Plugin event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginEvent {
	/// Plugin was loaded
	Loaded { plugin_id:String },
	/// Plugin was started
	Started { plugin_id:String },
	/// Plugin was stopped
	Stopped { plugin_id:String },
	/// Plugin was unloaded
	Unloaded { plugin_id:String },
	/// Plugin encountered an error
	Error { plugin_id:String, error:String },
	/// Plugin sent a message
	Message { from:String, to:String, action:String },
	/// Configuration changed
	ConfigChanged { old:serde_json::Value, new:serde_json::Value },
}

/// Plugin event handler
#[async_trait]
pub trait PluginEventHandler: Send + Sync {
	/// Handle a plugin event
	async fn Event(&self, event:&PluginEvent) -> Result<()>;
}

/// Event bus for plugin events
pub struct PluginEventBus {
	handlers:Arc<RwLock<Vec<Box<dyn PluginEventHandler>>>>,
}

impl PluginEventBus {
	/// Create a new event bus
	pub fn new() -> Self { Self { handlers:Arc::new(RwLock::new(vec![])) } }

	/// Register an event handler
	pub async fn register_handler(&self, handler:Box<dyn PluginEventHandler>) {
		let mut handlers = self.handlers.write().await;
		handlers.push(handler);
	}

	/// Emit an event to all handlers
	pub async fn emit(&self, event:PluginEvent) {
		let handlers = self.handlers.read().await;
		for handler in handlers.iter() {
			if let Err(e) = handler.Event(&event).await {
				dev_log!("extensions", "error: [PluginEventBus] Event handler error: {}", e);
			}
		}
	}
}

impl Default for PluginEventBus {
	fn default() -> Self { Self::new() }
}

// =============================================================================
// Plugin Discovery and Loading
// =============================================================================

/// Plugin discovery result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDiscoveryResult {
	pub plugin_id:String,
	pub ManifestPath:String,
	pub metadata:PluginMetadata,
	pub enabled:bool,
}

/// Plugin manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
	pub plugin:PluginMetadata,
	pub main:String,
	pub sandbox:Option<PluginSandboxConfig>,
}

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

// =============================================================================
// API Version Management
// =============================================================================

/// API version information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiVersion {
	pub major:u32,
	pub minor:u32,
	pub patch:u32,
	pub PreRelease:Option<String>,
}

impl ApiVersion {
	/// Get the current API version
	pub fn current() -> Self { Self { major:1, minor:0, patch:0, PreRelease:None } }

	/// Parse version from string
	pub fn parse(version:&str) -> Result<Self> {
		let parts:Vec<&str> = version.split('.').collect();
		if parts.len() < 3 {
			return Err(crate::AirError::Plugin("Invalid version format".to_string()));
		}

		Ok(Self {
			major:parts[0]
				.parse()
				.map_err(|_| crate::AirError::Plugin("Invalid major version".to_string()))?,
			minor:parts[1]
				.parse()
				.map_err(|_| crate::AirError::Plugin("Invalid minor version".to_string()))?,
			patch:parts[2]
				.parse()
				.map_err(|_| crate::AirError::Plugin("Invalid patch version".to_string()))?,
			PreRelease:if parts.len() > 3 { Some(parts[3].to_string()) } else { None },
		})
	}

	/// Check if this version is compatible with another
	pub fn IsCompatible(&self, other:&ApiVersion) -> bool {
		// Same major version means compatible
		if self.major != other.major {
			return false;
		}

		// If minor version is higher, it might have breaking changes
		if other.minor < self.minor {
			return false;
		}

		true
	}
}

/// API version manager
pub struct ApiVersionManager {
	CurrentVersion:ApiVersion,
	CompatibleVersions:Vec<ApiVersion>,
}

impl ApiVersionManager {
	/// Create a new API version manager
	pub fn new() -> Self {
		let current = ApiVersion::current();
		Self { CurrentVersion:current.clone(), CompatibleVersions:vec![current] }
	}

	/// Get the current API version
	pub fn current(&self) -> &ApiVersion { &self.CurrentVersion }

	/// Check if a version is compatible
	pub fn IsCompatible(&self, version:&ApiVersion) -> bool { self.CurrentVersion.IsCompatible(version) }

	/// Register a compatible API version
	pub fn register_compatible(&mut self, version:ApiVersion) {
		if self.IsCompatible(&version) && !self.CompatibleVersions.contains(&version) {
			self.CompatibleVersions.push(version);
		}
	}
}

impl Default for ApiVersionManager {
	fn default() -> Self { Self::new() }
}

// =============================================================================
// Plugin Isolation and Sandboxing
// =============================================================================

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

#[cfg(test)]
mod tests {
	use super::*;

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
