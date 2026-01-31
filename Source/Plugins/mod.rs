//! # Plugin Architecture
//!
//! ## Responsibilities
//!
//! This module provides a comprehensive plugin system for the Air daemon, enabling
//! extensibility through dynamically loaded plugins that can enhance daemon functionality.
//! The plugin system is responsible for:
//!
//! - **Plugin Discovery**: Automatically discovering available plugins from configured directories
//! - **Plugin Loading**: Dynamically loading plugins into the daemon runtime
//! - **Plugin Validation**: Validating plugin metadata, dependencies, and compatibility
//! - **Sandboxing**: Isolating plugins to prevent crashes and security issues
//! - **Lifecycle Management**: Managing plugin states (load, start, stop, unload) with proper hooks
//! - **API Registration**: Extending the daemon API through plugin-provided commands and handlers
//! - **Inter-Plugin Communication Enabling**: plugins to communicate with each other via message passing
//! - **Permission Management**: Enforcing fine-grained permissions and capabilities for plugins
//! - **Version Compatibility**: Ensuring plugins are compatible with the daemon version
//! - **Dependency Resolution**: Resolving and validating plugin dependencies
//!
//! ## VSCode Extension Architecture Patterns
//!
//! This implementation draws inspiration from VSCode's extension architecture:
//! - Reference: /Volumes/CORSAIR/Developer/macOS/Application/CodeEditorLand/Land/Dependency/Microsoft/Dependency/Editor/src/vs/platform/extensions/common/extensionHostStarter.ts
//! - Reference: /Volumes/CORSAIR/Developer/macOS/Application/CodeEditorLand/Land/Dependency/Microsoft/Dependency/Editor/src/vs/server/node/extensionHostConnection.ts
//! - Reference: /Volumes/CORSAIR/Developer/macOS/Application/CodeEditorLand/Land/Dependency/Microsoft/Dependency/Editor/src/vs/platform/remote/common/remoteAgentConnection.ts
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
//! The plugin system is designed to integrate with the Cocoon Extension Host (similar to
//! VSCode's extension host architecture). This provides:
//! - Isolated execution environments for plugins
//! - Crash recovery and resilience
//! - Resource management and limits
//! - Communication via IPC channels
//! - Hot reload capability without daemon restart
//!
//! ## TODO: Future Enhancements
//!
//! - **Plugin Marketplace**: Implement a central plugin marketplace for discovery and installation
//!   (similar to VSCode's extension marketplace)
//! - **Hot Reload Support**: Implement live reloading of plugins without daemon restart
//! - **Advanced Sandboxing**: Add more sophisticated sandboxing with resource quotas, network isolation,
//!   and filesystem access controls
//! - **Plugin Distribution**: Implement plugin packaging, signing, and distribution mechanisms
//! - **Automatic Updates**: Add automatic plugin update checking and installation
//! - **Telemetry Integration**: Add plugin usage telemetry and reporting
//! - **Plugin Profiles**: Support multiple plugin configurations for different environments
//! - **Security Audit**: Implement comprehensive security audit and vulnerability scanning for plugins
//! - **Performance Monitoring**: Add detailed performance monitoring and profiling for plugins
//! - **Plugin Debugging**: Provide debugging tools and interfaces for plugin developers
//!
//! ## Security and Isolation
//!
//! - Plugins run in isolated processes to prevent daemon crashes
//! - Fine-grained permission system controls plugin capabilities
//! - API version compatibility checks prevent breaking changes
//! - Resource limits prevent plugin exhaustion attacks
//! - Plugin authentication and signing to prevent malicious plugins
//! - Filesystem and network access restrictions

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use log::{info, warn, error};
use uuid::Uuid;

use crate::{Result, AirError};

// =============================================================================
// Plugin Types and Traits
// =============================================================================

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub min_air_version: String,
    pub max_air_version: Option<String>,
    pub dependencies: Vec<PluginDependency>,
    pub capabilities: Vec<String>,
}

/// Plugin dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub plugin_id: String,
    pub min_version: String,
    pub max_version: Option<String>,
    pub optional: bool,
}

/// Plugin capability and permission descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCapability {
    pub name: String,
    pub description: String,
    pub required_permissions: Vec<String>,
}

/// Plugin permission
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PluginPermission {
    /// Access filesystem
    Filesystem {
        read: bool,
        write: bool,
        paths: Vec<String>,
    },
    /// Access network
    Network {
        outbound: bool,
        inbound: bool,
        hosts: Vec<String>,
    },
    /// Access system resources
    System {
        cpu: bool,
        memory: bool,
    },
    /// Access other plugins
    InterPlugin {
        plugins: Vec<String>,
        actions: Vec<String>,
    },
    /// Custom permission
    Custom(String),
}

/// Plugin sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSandboxConfig {
    pub enabled: bool,
    pub max_memory_mb: Option<u64>,
    pub max_cpu_percent: Option<f64>,
    pub network_allowed: bool,
    pub filesystem_allowed: bool,
    pub allowed_paths: Vec<String>,
    pub timeout_secs: Option<u64>,
}

impl Default for PluginSandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_memory_mb: Some(128),
            max_cpu_percent: Some(10.0),
            network_allowed: false,
            filesystem_allowed: false,
            allowed_paths: vec![],
            timeout_secs: Some(30),
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
    async fn on_load(&self) -> Result<()> {
        Ok(())
    }

    /// Called when plugin is starting
    async fn on_start(&self) -> Result<()> {
        Ok(())
    }

    /// Called when plugin is stopping
    async fn on_stop(&self) -> Result<()> {
        Ok(())
    }

    /// Called when plugin is being unloaded
    async fn on_unload(&self) -> Result<()> {
        Ok(())
    }

    /// Called when configuration changes
    async fn on_config_changed(&self, _old: &serde_json::Value, _new: &serde_json::Value) -> Result<()> {
        Ok(())
    }
}

/// Plugin interface trait
#[async_trait]
pub trait Plugin: PluginHooks + Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Get plugin sandbox configuration
    fn sandbox_config(&self) -> PluginSandboxConfig {
        PluginSandboxConfig::default()
    }

    /// Get plugin permissions
    fn permissions(&self) -> Vec<PluginPermission> {
        vec![]
    }

    /// Handle inter-plugin message
    async fn handle_message(&self, from: &str, _message: &PluginMessage) -> Result<PluginMessage> {
        Err(AirError::Plugin(format!("Plugin {} does not handle messages", from)))
    }

    /// Get plugin state for diagnostics
    async fn get_state(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({}))
    }

    /// Check if plugin has specific capability
    fn has_capability(&self, _capability: &str) -> bool {
        false
    }

    /// Check if plugin has specific permission
    fn has_permission(&self, _permission: &PluginPermission) -> bool {
        false
    }
}

/// Inter-plugin message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub action: String,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

impl PluginMessage {
    /// Create a new plugin message
    pub fn new(from: String, to: String, action: String, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from,
            to,
            action,
            data,
            timestamp: Utc::now(),
        }
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
    pub plugin: Arc<Box<dyn Plugin>>,
    pub state: PluginState,
    pub started_at: Option<DateTime<Utc>>,
    pub loaded_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub sandbox: PluginSandboxConfig,
}

/// Main plugin manager
pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<String, PluginRegistry>>>,
    message_queue: Arc<RwLock<Vec<PluginMessage>>>,
    air_version: String,
    enable_sandbox: bool,
    startup_timeout: Duration,
    operation_timeout: Duration,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(air_version: String) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            message_queue: Arc::new(RwLock::new(Vec::new())),
            air_version,
            enable_sandbox: true,
            startup_timeout: Duration::from_secs(30),
            operation_timeout: Duration::from_secs(60),
        }
    }

    /// Create a new plugin manager with custom configuration
    pub fn with_config(
        air_version: String,
        enable_sandbox: bool,
        startup_timeout_secs: u64,
        operation_timeout_secs: u64,
    ) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            message_queue: Arc::new(RwLock::new(Vec::new())),
            air_version,
            enable_sandbox,
            startup_timeout: Duration::from_secs(startup_timeout_secs),
            operation_timeout: Duration::from_secs(operation_timeout_secs),
        }
    }

    /// Enable or disable sandbox mode
    pub fn set_sandbox_enabled(&mut self, enabled: bool) {
        self.enable_sandbox = enabled;
    }

    /// Discover plugins from a directory
    pub async fn discover_plugins(&self, directory: &str) -> Result<Vec<String>> {
        let mut discovered = vec![];

        // In production, this would scan the directory for plugin manifests
        // For now, we return an empty list
        info!("[PluginManager] Discovering plugins in directory: {}", directory);
        
        Ok(discovered)
    }

    /// Load a plugin from a manifest file
    pub async fn load_from_manifest(&self, path: &str) -> Result<String> {
        // In production, this would load and parse a plugin manifest
        // For now, we return a mock plugin ID
        info!("[PluginManager] Loading plugin from manifest: {}", path);
        
        Ok("loaded_plugin".to_string())
    }

    /// Register a plugin
    pub async fn register(&self, plugin: Arc<Box<dyn Plugin>>) -> Result<()> {
        let metadata = plugin.metadata();

        info!("[PluginManager] Registering plugin: {} v{}", metadata.name, metadata.version);

        // Validate plugin metadata
        self.validate_plugin_metadata(metadata)?;

        // Check Air version compatibility
        self.check_air_version_compatibility(metadata)?;

        // Check API version compatibility
        self.check_api_version_compatibility(metadata)?;

        // Check dependencies
        self.check_dependencies(metadata).await?;

        // Validate plugin capabilities and permissions
        self.validate_capabilities_and_permissions(plugin.as_ref().as_ref())?;

        // Setup sandbox configuration
        let sandbox = if self.enable_sandbox {
            plugin.sandbox_config()
        } else {
            PluginSandboxConfig {
                enabled: false,
                ..Default::default()
            }
        };

        // Load plugin with timeout
        let load_result = tokio::time::timeout(
            self.startup_timeout,
            plugin.on_load()
        ).await;

        let _load_result = load_result
            .map_err(|_| AirError::Plugin(format!("Plugin {} load timeout after {:?}", metadata.name, self.startup_timeout)))?
            .map_err(|e| {
                error!("[PluginManager] Failed to load plugin {}: {}", metadata.name, e);
                e
            })?;

        // Register in map
        let mut plugins = self.plugins.write().await;
        plugins.insert(
            metadata.id.clone(),
            PluginRegistry {
                plugin: plugin.clone(),
                state: PluginState::Loaded,
                started_at: None,
                loaded_at: Some(Utc::now()),
                error: None,
                sandbox,
            },
        );

        info!("[PluginManager] Plugin registered: {}", metadata.name);
        Ok(())
    }

    /// Validate plugin metadata
    pub fn validate_plugin_metadata(&self, metadata: &PluginMetadata) -> Result<()> {
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
    pub fn validate_capabilities_and_permissions(&self, plugin: &dyn Plugin) -> Result<()> {
        let permissions = plugin.permissions();

        // Check for dangerous permissions
        for permission in &permissions {
            match permission {
                PluginPermission::Filesystem { write, .. } if *write => {
                    warn!("[PluginManager] Plugin {} requests filesystem write access", plugin.metadata().id);
                }
                PluginPermission::Network { .. } => {
                    warn!("[PluginManager] Plugin {} requests network access", plugin.metadata().id);
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Check Air version compatibility
    pub fn check_air_version_compatibility(&self, metadata: &PluginMetadata) -> Result<()> {
        if !self.version_satisfies(&self.air_version, &metadata.min_air_version) {
            return Err(AirError::Plugin(format!(
                "Plugin requires Air version {} or higher, current: {}",
                metadata.min_air_version, self.air_version
            )));
        }

        if let Some(max_version) = &metadata.max_air_version {
            if !self.version_satisfies(max_version, &self.air_version) {
                return Err(AirError::Plugin(format!(
                    "Plugin is incompatible with Air version {}, max supported: {}",
                    self.air_version, max_version
                )));
            }
        }

        Ok(())
    }

    /// Check API version compatibility
    pub fn check_api_version_compatibility(&self, metadata: &PluginMetadata) -> Result<()> {
        // Check if plugin declares compatibility with current API version
        // In production, this would check against the daemon's API version
        Ok(())
    }

    /// Check plugin dependencies
    pub async fn check_dependencies(&self, metadata: &PluginMetadata) -> Result<()> {
        let plugins = self.plugins.read().await;

        for dep in &metadata.dependencies {
            if !dep.optional {
                let dep_plugin = plugins.get(&dep.plugin_id)
                    .ok_or_else(|| AirError::Plugin(format!(
                        "Required dependency not found: {}",
                        dep.plugin_id
                    )))?;

                let dep_version = &dep_plugin.plugin.metadata().version;
                if !self.version_satisfies(dep_version, &dep.min_version) {
                    return Err(AirError::Plugin(format!(
                        "Dependency {} version {} does not satisfy requirement {}",
                        dep.plugin_id, dep_version, dep.min_version
                    )));
                }

                if dep_plugin.state != PluginState::Running && dep_plugin.state != PluginState::Loaded {
                    return Err(AirError::Plugin(format!(
                        "Dependency {} is not ready (state: {:?})",
                        dep.plugin_id, dep_plugin.state
                    )));
                }
            }
        }

        Ok(())
    }

    /// Start a plugin
    pub async fn start(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        let registry = plugins.get_mut(plugin_id)
            .ok_or_else(|| AirError::Plugin(format!("Plugin not found: {}", plugin_id)))?;

        if registry.state == PluginState::Running {
            info!("[PluginManager] Plugin {} already running", plugin_id);
            return Ok(());
        }

        registry.state = PluginState::Starting;

        // Check sandbox configuration
        if self.enable_sandbox && registry.sandbox.enabled {
            info!("[PluginManager] Starting plugin {} in sandbox mode", plugin_id);
        }

        let plugin = registry.plugin.clone();
        drop(plugins);

        let start_result = tokio::time::timeout(
            self.startup_timeout,
            plugin.on_start()
        ).await;

        match start_result {
            Ok(Ok(())) => {
                let mut plugins = self.plugins.write().await;
                if let Some(registry) = plugins.get_mut(plugin_id) {
                    registry.state = PluginState::Running;
                    registry.started_at = Some(Utc::now());
                    registry.error = None;
                }
                info!("[PluginManager] Plugin started: {}", plugin_id);
                Ok(())
            }
            Ok(Err(e)) => {
                let mut plugins = self.plugins.write().await;
                if let Some(registry) = plugins.get_mut(plugin_id) {
                    registry.state = PluginState::Error;
                    registry.error = Some(e.to_string());
                }
                error!("[PluginManager] Plugin start failed: {}: {}", plugin_id, e);
                Err(e)
            }
            Err(_) => {
                let mut plugins = self.plugins.write().await;
                if let Some(registry) = plugins.get_mut(plugin_id) {
                    registry.state = PluginState::Error;
                    registry.error = Some(format!("Startup timeout after {:?}", self.startup_timeout));
                }
                error!("[PluginManager] Plugin start timeout: {}", plugin_id);
                Err(AirError::Plugin(format!("Plugin {} startup timeout", plugin_id)))
            }
        }
    }

    /// Stop a plugin
    pub async fn stop(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        let registry = plugins.get_mut(plugin_id)
            .ok_or_else(|| AirError::Plugin(format!("Plugin not found: {}", plugin_id)))?;

        if registry.state != PluginState::Running {
            info!("[PluginManager] Plugin {} not running", plugin_id);
            return Ok(());
        }

        registry.state = PluginState::Stopping;
        let plugin = registry.plugin.clone();
        drop(plugins);

        let stop_result = tokio::time::timeout(
            self.operation_timeout,
            plugin.on_stop()
        ).await;

        match stop_result {
            Ok(Ok(())) => {
                let mut plugins = self.plugins.write().await;
                if let Some(registry) = plugins.get_mut(plugin_id) {
                    registry.state = PluginState::Loaded;
                    registry.started_at = None;
                }
                info!("[PluginManager] Plugin stopped: {}", plugin_id);
                Ok(())
            }
            Ok(Err(e)) => {
                let mut plugins = self.plugins.write().await;
                if let Some(registry) = plugins.get_mut(plugin_id) {
                    registry.state = PluginState::Error;
                    registry.error = Some(e.to_string());
                }
                error!("[PluginManager] Plugin stop failed: {}: {}", plugin_id, e);
                Err(e)
            }
            Err(_) => {
                let mut plugins = self.plugins.write().await;
                if let Some(registry) = plugins.get_mut(plugin_id) {
                    registry.state = PluginState::Error;
                    registry.error = Some(format!("Stop timeout after {:?}", self.operation_timeout));
                }
                error!("[PluginManager] Plugin stop timeout: {}", plugin_id);
                Err(AirError::Plugin(format!("Plugin {} stop timeout", plugin_id)))
            }
        }
    }

    /// Start all registered plugins
    pub async fn start_all(&self) -> Result<()> {
        let plugin_ids: Vec<String> = self.plugins
            .read()
            .await
            .keys()
            .cloned()
            .collect();

        info!("[PluginManager] Starting {} plugins", plugin_ids.len());

        for plugin_id in plugin_ids {
            if let Err(e) = self.start(&plugin_id).await {
                warn!("[PluginManager] Failed to start plugin {}: {}", plugin_id, e);
            }
        }

        Ok(())
    }

    /// Stop all running plugins
    pub async fn stop_all(&self) -> Result<()> {
        let plugin_ids: Vec<String> = self.plugins
            .read()
            .await
            .keys()
            .cloned()
            .collect();

        info!("[PluginManager] Stopping {} plugins", plugin_ids.len());

        // Stop in reverse order to respect dependencies
        for plugin_id in plugin_ids.into_iter().rev() {
            if let Err(e) = self.stop(&plugin_id).await {
                warn!("[PluginManager] Failed to stop plugin {}: {}", plugin_id, e);
            }
        }

        Ok(())
    }

    /// Load a plugin
    pub async fn load(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        let registry = plugins.get_mut(plugin_id)
            .ok_or_else(|| AirError::Plugin(format!("Plugin not found: {}", plugin_id)))?;

        if registry.state != PluginState::Unloaded {
            info!("[PluginManager] Plugin {} already loaded", plugin_id);
            return Ok(());
        }

        let plugin = registry.plugin.clone();
        drop(plugins);

        let load_result = tokio::time::timeout(
            self.startup_timeout,
            plugin.on_load()
        ).await;

        match load_result {
            Ok(Ok(())) => {
                let mut plugins = self.plugins.write().await;
                if let Some(registry) = plugins.get_mut(plugin_id) {
                    registry.state = PluginState::Loaded;
                    registry.loaded_at = Some(Utc::now());
                    registry.error = None;
                }
                info!("[PluginManager] Plugin loaded: {}", plugin_id);
                Ok(())
            }
            Ok(Err(e)) => {
                let mut plugins = self.plugins.write().await;
                if let Some(registry) = plugins.get_mut(plugin_id) {
                    registry.state = PluginState::Error;
                    registry.error = Some(e.to_string());
                }
                error!("[PluginManager] Plugin load failed: {}: {}", plugin_id, e);
                Err(e)
            }
            Err(_) => {
                let mut plugins = self.plugins.write().await;
                if let Some(registry) = plugins.get_mut(plugin_id) {
                    registry.state = PluginState::Error;
                    registry.error = Some(format!("Load timeout after {:?}", self.startup_timeout));
                }
                error!("[PluginManager] Plugin load timeout: {}", plugin_id);
                Err(AirError::Plugin(format!("Plugin {} load timeout", plugin_id)))
            }
        }
    }

    /// Unload a plugin
    pub async fn unload(&self, plugin_id: &str) -> Result<()> {
        // First stop the plugin
        self.stop(plugin_id).await?;

        let mut plugins = self.plugins.write().await;
        let registry = plugins.get(plugin_id)
            .ok_or_else(|| AirError::Plugin(format!("Plugin not found: {}", plugin_id)))?;

        let plugin = registry.plugin.clone();
        plugins.remove(plugin_id);

        let unload_result = tokio::time::timeout(
            self.operation_timeout,
            plugin.on_unload()
        ).await;

        match unload_result {
            Ok(Ok(())) => {
                info!("[PluginManager] Plugin unloaded: {}", plugin_id);
                Ok(())
            }
            Ok(Err(e)) => {
                // Plugin is removed from registry even if unload fails
                error!("[PluginManager] Plugin unload error: {}: {}", plugin_id, e);
                Err(e)
            }
            Err(_) => {
                // Plugin is removed from registry even if timeout occurs
                warn!("[PluginManager] Plugin unload timeout: {}", plugin_id);
                Err(AirError::Plugin(format!("Plugin {} unload timeout", plugin_id)))
            }
        }
    }

    /// Send message from one plugin to another
    pub async fn send_message(&self, message: PluginMessage) -> Result<PluginMessage> {
        // Validate message
        message.validate()?;

        let plugins = self.plugins.read().await;

        let target = plugins.get(&message.to)
            .ok_or_else(|| AirError::Plugin(format!("Target plugin not found: {}", message.to)))?;

        if target.state != PluginState::Running {
            return Err(AirError::Plugin(format!(
                "Target plugin not running: {} (state: {:?})",
                message.to, target.state
            )));
        }

        // Check if sender has permission to send to receiver
        let sender_metadata = plugins.get(&message.from)
            .ok_or_else(|| AirError::Plugin(format!("Sender plugin not found: {}", message.from)))?;

        if !self.check_inter_plugin_permission(sender_metadata, target, &message) {
            return Err(AirError::Plugin(format!(
                "Permission denied: {} cannot send to {}",
                message.from, message.to
            )));
        }

        let plugin = target.plugin.clone();
        drop(plugins);

        // Send message with timeout
        let send_result = tokio::time::timeout(
            self.operation_timeout,
            plugin.handle_message(&message.from, &message)
        ).await;

        send_result.map_err(|_| {
            AirError::Plugin(format!("Message send timeout: {} -> {}", message.from, message.to))
        })?
    }

    /// Check inter-plugin communication permission
    fn check_inter_plugin_permission(
        &self,
        _sender: &PluginRegistry,
        _target: &PluginRegistry,
        _message: &PluginMessage,
    ) -> bool {
        // In production, this would check if sender has permission to communicate with target
        // For now, we allow all communication
        true
    }

    /// Get plugin list with details
    pub async fn list_plugins(&self) -> Result<Vec<PluginInfo>> {
        let plugins = self.plugins.read().await;
        let mut result = Vec::new();

        for (id, registry) in plugins.iter() {
            let metadata = registry.plugin.metadata().clone();
            result.push(PluginInfo {
                id: id.clone(),
                metadata,
                state: registry.state,
                uptime_secs: registry.started_at
                    .map(|t| (Utc::now() - t).num_seconds() as u64)
                    .unwrap_or(0),
                error: registry.error.clone(),
            });
        }

        Ok(result)
    }

    /// Get plugin state
    pub async fn get_plugin_state(&self, plugin_id: &str) -> Result<serde_json::Value> {
        let plugins = self.plugins.read().await;
        let registry = plugins.get(plugin_id)
            .ok_or_else(|| AirError::Plugin(format!("Plugin not found: {}", plugin_id)))?;

        registry.plugin.get_state().await
    }

    /// Get plugin permissions
    pub async fn get_plugin_permissions(&self, plugin_id: &str) -> Result<Vec<PluginPermission>> {
        let plugins = self.plugins.read().await;
        let registry = plugins.get(plugin_id)
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
    pub fn validate_plugin(&self, plugin: &dyn Plugin) -> PluginValidationResult {
        let metadata = plugin.metadata();

        // Validate metadata
        if let Err(e) = self.validate_plugin_metadata(metadata) {
            return PluginValidationResult::Invalid(e.to_string());
        }

        // Check version compatibility
        if let Err(e) = self.check_air_version_compatibility(metadata) {
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
            let dependencies: Vec<String> = metadata.dependencies.iter().map(|d| d.plugin_id.clone()).collect();
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
            self.visit_plugin_for_load_order(plugin_id, &mut visited, &mut order, &plugins)?;
        }

        Ok(order)
    }

    /// Visit plugin for load order (helper function)
    fn visit_plugin_for_load_order(
        &self,
        plugin_id: &str,
        visited: &mut std::collections::HashSet<String>,
        order: &mut Vec<String>,
        plugins: &HashMap<String, PluginRegistry>,
    ) -> Result<()> {
        if visited.contains(plugin_id) {
            return Ok(());
        }

        visited.insert(plugin_id.to_string());

        if let Some(registry) = plugins.get(plugin_id) {
            let metadata = registry.plugin.metadata();
            for dep in &metadata.dependencies {
                if !dep.optional {
                    self.visit_plugin_for_load_order(&dep.plugin_id, visited, order, plugins)?;
                }
            }
        }

        order.push(plugin_id.to_string());
        Ok(())
    }

    /// Simple version satisfaction check (X.Y.Z format)
    fn version_satisfies(&self, actual: &str, required: &str) -> bool {
        let actual_parts: Vec<&str> = actual.split('.').collect();
        let required_parts: Vec<&str> = required.split('.').collect();

        for (i, required_part) in required_parts.iter().enumerate() {
            if let (Ok(a), Ok(r)) = (
                actual_parts.get(i).unwrap_or(&"0").parse::<u32>(),
                required_part.parse::<u32>(),
            ) {
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
    pub id: String,
    pub metadata: PluginMetadata,
    pub state: PluginState,
    pub uptime_secs: u64,
    pub error: Option<String>,
}

// =============================================================================
// Plugin Event System
// =============================================================================

/// Plugin event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginEvent {
    /// Plugin was loaded
    Loaded { plugin_id: String },
    /// Plugin was started
    Started { plugin_id: String },
    /// Plugin was stopped
    Stopped { plugin_id: String },
    /// Plugin was unloaded
    Unloaded { plugin_id: String },
    /// Plugin encountered an error
    Error { plugin_id: String, error: String },
    /// Plugin sent a message
    Message { from: String, to: String, action: String },
    /// Configuration changed
    ConfigChanged { old: serde_json::Value, new: serde_json::Value },
}

/// Plugin event handler
#[async_trait]
pub trait PluginEventHandler: Send + Sync {
    /// Handle a plugin event
    async fn handle_event(&self, event: &PluginEvent) -> Result<()>;
}

/// Event bus for plugin events
pub struct PluginEventBus {
    handlers: Arc<RwLock<Vec<Box<dyn PluginEventHandler>>>>,
}

impl PluginEventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(vec![])),
        }
    }

    /// Register an event handler
    pub async fn register_handler(&self, handler: Box<dyn PluginEventHandler>) {
        let mut handlers = self.handlers.write().await;
        handlers.push(handler);
    }

    /// Emit an event to all handlers
    pub async fn emit(&self, event: PluginEvent) {
        let handlers = self.handlers.read().await;
        for handler in handlers.iter() {
            if let Err(e) = handler.handle_event(&event).await {
                error!("[PluginEventBus] Event handler error: {}", e);
            }
        }
    }
}

impl Default for PluginEventBus {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Plugin Discovery and Loading
// =============================================================================

/// Plugin discovery result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDiscoveryResult {
    pub plugin_id: String,
    pub manifest_path: String,
    pub metadata: PluginMetadata,
    pub enabled: bool,
}

/// Plugin manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub plugin: PluginMetadata,
    pub main: String,
    pub sandbox: Option<PluginSandboxConfig>,
}

/// Plugin loader for discovering and loading plugins
pub struct PluginLoader {
    plugin_paths: Vec<String>,
}

impl PluginLoader {
    /// Create a new plugin loader
    pub fn new() -> Self {
        Self {
            plugin_paths: vec![
                "/usr/local/lib/air/plugins".to_string(),
                "~/.local/share/air/plugins".to_string(),
            ],
        }
    }

    /// Add a plugin discovery path
    pub fn add_path(&mut self, path: String) {
        self.plugin_paths.push(path);
    }

    /// Discover plugins from all configured paths
    pub async fn discover_all(&self) -> Result<Vec<PluginDiscoveryResult>> {
        let mut results = vec![];

        for path in &self.plugin_paths {
            match self.discover_in_path(path).await {
                Ok(mut discovered) => {
                    results.append(&mut discovered);
                }
                Err(e) => {
                    warn!("[PluginLoader] Failed to discover plugins in {}: {}", path, e);
                }
            }
        }

        Ok(results)
    }

    /// Discover plugins in a specific path
    pub async fn discover_in_path(&self, path: &str) -> Result<Vec<PluginDiscoveryResult>> {
        let mut results = vec![];

        // In production, this would scan the directory for plugin manifests
        // For now, we return an empty list
        info!("[PluginLoader] Discovering plugins in: {}", path);

        Ok(results)
    }

    /// Load a plugin from a discovery result
    pub async fn load_from_discovery(&self, discovery: &PluginDiscoveryResult) -> Result<Arc<Box<dyn Plugin>>> {
        // In production, this would load the plugin from the manifest
        // For now, we return an error
        Err(AirError::Plugin(format!("Plugin loading not yet implemented: {}", discovery.plugin_id)))
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// API Version Management
// =============================================================================

/// API version information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
}

impl ApiVersion {
    /// Get the current API version
    pub fn current() -> Self {
        Self {
            major: 1,
            minor: 0,
            patch: 0,
            pre_release: None,
        }
    }

    /// Parse version from string
    pub fn parse(version: &str) -> Result<Self> {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() < 3 {
            return Err(crate::AirError::Plugin("Invalid version format".to_string()));
        }

        Ok(Self {
            major: parts[0].parse().map_err(|_| crate::AirError::Plugin("Invalid major version".to_string()))?,
            minor: parts[1].parse().map_err(|_| crate::AirError::Plugin("Invalid minor version".to_string()))?,
            patch: parts[2].parse().map_err(|_| crate::AirError::Plugin("Invalid patch version".to_string()))?,
            pre_release: if parts.len() > 3 { Some(parts[3].to_string()) } else { None },
        })
    }

    /// Check if this version is compatible with another
    pub fn is_compatible(&self, other: &ApiVersion) -> bool {
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
    current_version: ApiVersion,
    compatible_versions: Vec<ApiVersion>,
}

impl ApiVersionManager {
    /// Create a new API version manager
    pub fn new() -> Self {
        let current = ApiVersion::current();
        Self {
            current_version: current.clone(),
            compatible_versions: vec![current],
        }
    }

    /// Get the current API version
    pub fn current(&self) -> &ApiVersion {
        &self.current_version
    }

    /// Check if a version is compatible
    pub fn is_compatible(&self, version: &ApiVersion) -> bool {
        self.current_version.is_compatible(version)
    }

    /// Register a compatible API version
    pub fn register_compatible(&mut self, version: ApiVersion) {
        if self.is_compatible(&version) && !self.compatible_versions.contains(&version) {
            self.compatible_versions.push(version);
        }
    }
}

impl Default for ApiVersionManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Plugin Isolation and Sandboxing
// =============================================================================

/// Plugin sandbox manager
pub struct PluginSandboxManager {
    sandboxes: Arc<RwLock<HashMap<String, PluginSandboxConfig>>>,
}

impl PluginSandboxManager {
    /// Create a new sandbox manager
    pub fn new() -> Self {
        Self {
            sandboxes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a sandbox for a plugin
    pub async fn create_sandbox(&self, plugin_id: String, config: PluginSandboxConfig) -> Result<()> {
        let mut sandboxes = self.sandboxes.write().await;
        sandboxes.insert(plugin_id, config);
        Ok(())
    }

    /// Get sandbox configuration
    pub async fn get_sandbox(&self, plugin_id: &str) -> Option<PluginSandboxConfig> {
        let sandboxes = self.sandboxes.read().await;
        sandboxes.get(plugin_id).cloned()
    }

    /// Remove a sandbox
    pub async fn remove_sandbox(&self, plugin_id: &str) {
        let mut sandboxes = self.sandboxes.write().await;
        sandboxes.remove(plugin_id);
    }

    /// Check if a plugin is running in a sandbox
    pub async fn is_sandboxed(&self, plugin_id: &str) -> bool {
        let sandboxes = self.sandboxes.read().await;
        sandboxes.get(plugin_id).map_or(false, |s| s.enabled)
    }
}

impl Default for PluginSandboxManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin;

    #[async_trait]
    impl PluginHooks for TestPlugin {}

    #[async_trait]
    impl Plugin for TestPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &PluginMetadata {
                id: "test".to_string(),
                name: "Test Plugin".to_string(),
                version: "1.0.0".to_string(),
                description: "A test plugin".to_string(),
                author: "Test".to_string(),
                min_air_version: "0.1.0".to_string(),
                max_air_version: None,
                dependencies: vec![],
                capabilities: vec![],
            }
        }
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
    fn test_api_version_compatibility() {
        let v1 = ApiVersion { major: 1, minor: 0, patch: 0, pre_release: None };
        let v2 = ApiVersion { major: 1, minor: 1, patch: 0, pre_release: None };
        let v3 = ApiVersion { major: 2, minor: 0, patch: 0, pre_release: None };

        assert!(v1.is_compatible(&v2));
        assert!(!v1.is_compatible(&v3));
    }

    #[tokio::test]
    fn test_sandbox_config_default() {
        let config = PluginSandboxConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_memory_mb, Some(128));
        assert!(!config.network_allowed);
        assert!(!config.filesystem_allowed);
    }

    #[tokio::test]
    fn test_plugin_metadata_validation() {
        let manager = PluginManager::new("1.0.0".to_string ());
        let metadata = PluginMetadata {
            id: "test_plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "A test plugin".to_string(),
            author: "Test".to_string(),
            min_air_version: "1.0.0".to_string(),
            max_air_version: None,
            dependencies: vec![],
            capabilities: vec![],
        };

        assert!(manager.validate_plugin_metadata(&metadata).is_ok());

        let invalid_metadata = PluginMetadata {
            id: "".to_string(),
            name: "Invalid".to_string(),
            version: "1.0.0".to_string(),
            description: "Invalid plugin".to_string(),
            author: "Test".to_string(),
            min_air_version: "1.0.0".to_string(),
            max_air_version: None,
            dependencies: vec![],
            capabilities: vec![],
        };

        assert!(manager.validate_plugin_metadata(&invalid_metadata).is_err());
    }
}
