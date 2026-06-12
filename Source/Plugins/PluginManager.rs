//! Main plugin manager: registration, validation, lifecycle management, messaging,
//! dependency resolution, and load-order resolution.

use std::{collections::HashMap, sync::Arc, time::Duration};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::Plugins::Plugin::Plugin;
use crate::Plugins::PluginInfo::PluginInfo;
use crate::Plugins::PluginMessage::PluginMessage;
use crate::Plugins::PluginMetadata::PluginMetadata;
use crate::Plugins::PluginPermission::PluginPermission;
use crate::Plugins::PluginRegistry::PluginRegistry;
use crate::Plugins::PluginSandboxConfig::PluginSandboxConfig;
use crate::Plugins::PluginState::PluginState;
use crate::Plugins::PluginValidationResult::PluginValidationResult;
use crate::{AirError, Result, dev_log};

/// Main plugin manager
pub struct PluginManager {
	plugins:Arc<RwLock<HashMap<String, PluginRegistry>>>,

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
		let SendResult = tokio::time::timeout(self.OperationTimeout, plugin.Message(&message.from, &message)).await;

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
	pub fn version_satisfies(&self, actual:&str, required:&str) -> bool {
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
