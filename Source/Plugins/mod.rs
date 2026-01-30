//! # Plugin Architecture
//!
//! Provides a robust plugin system with lifecycle hooks, inter-plugin
//! communication, dependency resolution, and version compatibility checking.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use log::{info, warn, error, debug};
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
    async fn on_config_changed(&self, old: &serde_json::Value, new: &serde_json::Value) -> Result<()> {
        Ok(())
    }
}

/// Plugin interface trait
#[async_trait]
pub trait Plugin: PluginHooks + Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;
    
    /// Handle inter-plugin message
    async fn handle_message(&self, from: &str, message: &PluginMessage) -> Result<PluginMessage> {
        Err(AirError::Plugin(format!("Plugin {} does not handle messages", from)))
    }
    
    /// Get plugin state for diagnostics
    async fn get_state(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({}))
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
#[derive(Debug)]
pub struct PluginRegistry {
    pub plugin: Arc<Box<dyn Plugin>>,
    pub state: PluginState,
    pub started_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// Main plugin manager
pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<String, PluginRegistry>>>,
    message_queue: Arc<RwLock<Vec<PluginMessage>>>,
    air_version: String,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(air_version: String) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            message_queue: Arc::new(RwLock::new(Vec::new())),
            air_version,
        }
    }
    
    /// Register a plugin
    pub async fn register(&self, plugin: Arc<Box<dyn Plugin>>) -> Result<()> {
        let metadata = plugin.metadata();
        
        info!("[PluginManager] Registering plugin: {} v{}", metadata.name, metadata.version);
        
        // Check Air version compatibility
        self.check_version_compatibility(metadata)?;
        
        // Check dependencies
        self.check_dependencies(metadata).await?;
        
        // Load plugin
        plugin.on_load().await
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
                error: None,
            },
        );
        
        info!("[PluginManager] Plugin registered: {}", metadata.name);
        Ok(())
    }
    
    /// Start a plugin
    pub async fn start(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        let registry = plugins.get_mut(plugin_id)
            .ok_or_else(|| AirError::Plugin(format!("Plugin not found: {}", plugin_id)))?;
        
        if registry.state == PluginState::Running {
            return Ok(());
        }
        
        registry.state = PluginState::Starting;
        
        let plugin = registry.plugin.clone();
        drop(plugins);
        
        match plugin.on_start().await {
            Ok(()) => {
                let mut plugins = self.plugins.write().await;
                if let Some(registry) = plugins.get_mut(plugin_id) {
                    registry.state = PluginState::Running;
                    registry.started_at = Some(Utc::now());
                    registry.error = None;
                }
                info!("[PluginManager] Plugin started: {}", plugin_id);
                Ok(())
            }
            Err(e) => {
                let mut plugins = self.plugins.write().await;
                if let Some(registry) = plugins.get_mut(plugin_id) {
                    registry.state = PluginState::Error;
                    registry.error = Some(e.to_string());
                }
                error!("[PluginManager] Plugin start failed: {}: {}", plugin_id, e);
                Err(e)
            }
        }
    }
    
    /// Stop a plugin
    pub async fn stop(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        let registry = plugins.get_mut(plugin_id)
            .ok_or_else(|| AirError::Plugin(format!("Plugin not found: {}", plugin_id)))?;
        
        if registry.state != PluginState::Running {
            return Ok(());
        }
        
        registry.state = PluginState::Stopping;
        let plugin = registry.plugin.clone();
        drop(plugins);
        
        match plugin.on_stop().await {
            Ok(()) => {
                let mut plugins = self.plugins.write().await;
                if let Some(registry) = plugins.get_mut(plugin_id) {
                    registry.state = PluginState::Loaded;
                }
                info!("[PluginManager] Plugin stopped: {}", plugin_id);
                Ok(())
            }
            Err(e) => {
                let mut plugins = self.plugins.write().await;
                if let Some(registry) = plugins.get_mut(plugin_id) {
                    registry.state = PluginState::Error;
                    registry.error = Some(e.to_string());
                }
                error!("[PluginManager] Plugin stop failed: {}: {}", plugin_id, e);
                Err(e)
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
        
        for plugin_id in plugin_ids {
            if let Err(e) = self.stop(&plugin_id).await {
                warn!("[PluginManager] Failed to stop plugin {}: {}", plugin_id, e);
            }
        }
        
        Ok(())
    }
    
    /// Unload a plugin
    pub async fn unload(&self, plugin_id: &str) -> Result<()> {
        // First stop the plugin
        self.stop(plugin_id).await?;
        
        let mut plugins = self.plugins.write().await;
        let registry = plugins.get(plugin_id)
            .ok_or_else(|| AirError::Plugin(format!("Plugin not found: {}", plugin_id)))?;
        
        let plugin = registry.plugin.clone();
        plugin.on_unload().await?;
        
        plugins.remove(plugin_id);
        
        info!("[PluginManager] Plugin unloaded: {}", plugin_id);
        Ok(())
    }
    
    /// Send message from one plugin to another
    pub async fn send_message(&self, message: PluginMessage) -> Result<PluginMessage> {
        let plugins = self.plugins.read().await;
        
        let target = plugins.get(&message.to)
            .ok_or_else(|| AirError::Plugin(format!("Target plugin not found: {}", message.to)))?;
        
        if target.state != PluginState::Running {
            return Err(AirError::Plugin(format!(
                "Target plugin not running: {} (state: {:?})",
                message.to, target.state
            )));
        }
        
        let plugin = target.plugin.clone();
        drop(plugins);
        
        plugin.handle_message(&message.from, &message).await
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
    
    /// Check version compatibility
    fn check_version_compatibility(&self, metadata: &PluginMetadata) -> Result<()> {
        // Simple version comparison (in production, use semantic versioning library)
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
    
    /// Check plugin dependencies
    async fn check_dependencies(&self, metadata: &PluginMetadata) -> Result<()> {
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
            }
        }
        
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
}
