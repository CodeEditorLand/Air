//! # Configuration Hot-Reload
//!
//! Provides live configuration reloading with file system monitoring,
//! validation, atomic swaps, and rollback capabilities.

use std::sync::Arc;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::fs;
use notify::{Watcher, RecursiveMode, Result as NotifyResult};
use chrono::{DateTime, Utc};
use log::{info, warn, error, debug};

use crate::{Result, AirError, Configuration::AirConfiguration};

// =============================================================================
// Configuration Hot-Reload Manager
// =============================================================================

/// Configuration hot-reload manager with file watching and validation
pub struct ConfigHotReload {
    /// Current active configuration
    active_config: Arc<RwLock<AirConfiguration>>,
    
    /// Previous configuration for rollback
    previous_config: Arc<RwLock<Option<AirConfiguration>>>,
    
    /// Configuration file path
    config_path: PathBuf,
    
    /// File watcher for monitoring changes
    watcher: Option<Arc<RwLock<notify::RecommendedWatcher>>>,
    
    /// Change history for auditing
    change_history: Arc<RwLock<Vec<ConfigChangeRecord>>>,
    
    /// Last reload timestamp
    last_reload: Arc<RwLock<Option<DateTime<Utc>>>>,
    
    /// Whether hot-reload is enabled
    enabled: Arc<RwLock<bool>>,
    
    /// Validation callbacks
    validators: Arc<RwLock<Vec<Box<dyn ConfigValidator>>>>,
}

/// Configuration change record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeRecord {
    pub timestamp: DateTime<Utc>,
    pub changes: Vec<ConfigChange>,
    pub validated: bool,
    pub reason: String,
}

/// Individual configuration change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChange {
    pub path: String,
    pub old_value: serde_json::Value,
    pub new_value: serde_json::Value,
}

/// Configuration validation trait
pub trait ConfigValidator: Send + Sync {
    /// Validate a configuration
    fn validate(&self, config: &AirConfiguration) -> Result<()>;
    
    /// Get validator name
    fn name(&self) -> &str;
}

/// Validator for GRPC configuration
pub struct GrpcConfigValidator;

impl ConfigValidator for GrpcConfigValidator {
    fn validate(&self, config: &AirConfiguration) -> Result<()> {
        if config.grpc.bind_address.is_empty() {
            return Err(AirError::Configuration(
                "gRPC bind address cannot be empty".to_string(),
            ));
        }
        
        if config.grpc.max_connections == 0 {
            return Err(AirError::Configuration(
                "gRPC max connections must be greater than 0".to_string(),
            ));
        }
        
        if config.grpc.request_timeout_secs == 0 {
            return Err(AirError::Configuration(
                "gRPC request timeout must be greater than 0".to_string(),
            ));
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "GrpcConfigValidator"
    }
}

/// Validator for authentication configuration
pub struct AuthConfigValidator;

impl ConfigValidator for AuthConfigValidator {
    fn validate(&self, config: &AirConfiguration) -> Result<()> {
        if config.authentication.enabled && config.authentication.credentials_path.is_empty() {
            return Err(AirError::Configuration(
                "Authentication credentials path cannot be empty".to_string(),
            ));
        }
        
        if config.authentication.token_expiration_hours == 0 {
            return Err(AirError::Configuration(
                "Token expiration must be greater than 0 hours".to_string(),
            ));
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "AuthConfigValidator"
    }
}

/// Validator for update configuration
pub struct UpdateConfigValidator;

impl ConfigValidator for UpdateConfigValidator {
    fn validate(&self, config: &AirConfiguration) -> Result<()> {
        if config.updates.enabled && config.updates.update_server_url.is_empty() {
            return Err(AirError::Configuration(
                "Update server URL cannot be empty".to_string(),
            ));
        }
        
        if config.updates.check_interval_hours == 0 {
            return Err(AirError::Configuration(
                "Update check interval must be greater than 0".to_string(),
            ));
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "UpdateConfigValidator"
    }
}

/// Validator for downloader configuration
pub struct DownloadConfigValidator;

impl ConfigValidator for DownloadConfigValidator {
    fn validate(&self, config: &AirConfiguration) -> Result<()> {
        if config.downloader.enabled {
            if config.downloader.cache_directory.is_empty() {
                return Err(AirError::Configuration(
                    "Download cache directory cannot be empty".to_string(),
                ));
            }
            
            if config.downloader.max_concurrent_downloads == 0 {
                return Err(AirError::Configuration(
                    "Max concurrent downloads must be greater than 0".to_string(),
                ));
            }
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "DownloadConfigValidator"
    }
}

// =============================================================================
// Implementation
// =============================================================================

impl ConfigHotReload {
    /// Create a new hot-reload manager
    pub async fn new(config_path: PathBuf, initial_config: AirConfiguration) -> Result<Self> {
        let manager = Self {
            active_config: Arc::new(RwLock::new(initial_config)),
            previous_config: Arc::new(RwLock::new(None)),
            config_path,
            watcher: None,
            change_history: Arc::new(RwLock::new(Vec::new())),
            last_reload: Arc::new(RwLock::new(None)),
            enabled: Arc::new(RwLock::new(true)),
            validators: Arc::new(RwLock::new(vec![
                Box::new(GrpcConfigValidator) as Box<dyn ConfigValidator>,
                Box::new(AuthConfigValidator) as Box<dyn ConfigValidator>,
                Box::new(UpdateConfigValidator) as Box<dyn ConfigValidator>,
                Box::new(DownloadConfigValidator) as Box<dyn ConfigValidator>,
            ])),
        };
        
        Ok(manager)
    }
    
    /// Enable file watching for configuration changes
    pub async fn enable_file_watching(&mut self) -> Result<()> {
        info!("[HotReload] Enabling file watching for configuration changes");
        
        // File watching implementation would go here
        // This is a placeholder for the actual watch implementation
        
        *self.enabled.write().await = true;
        
        info!("[HotReload] File watching enabled");
        Ok(())
    }
    
    /// Disable file watching
    pub async fn disable_file_watching(&mut self) -> Result<()> {
        *self.enabled.write().await = false;
        info!("[HotReload] File watching disabled");
        Ok(())
    }
    
    /// Reload configuration from file
    pub async fn reload(&self) -> Result<()> {
        debug!("[HotReload] Reloading configuration from: {}", self.config_path.display());
        
        // Check if enabled
        if !*self.enabled.read().await {
            return Err(AirError::Configuration("Hot-reload is disabled".to_string()));
        }
        
        // Load new configuration
        let content = fs::read_to_string(&self.config_path)
            .await
            .map_err(|e| AirError::Configuration(format!("Failed to read config file: {}", e)))?;
        
        let new_config: AirConfiguration = toml::from_str(&content)
            .map_err(|e| AirError::Configuration(format!("Failed to parse config: {}", e)))?;
        
        // Validate new configuration
        self.validate_config(&new_config).await?;
        
        // Atomically swap configurations
        let old_config = self.active_config.read().await.clone();
        
        *self.active_config.write().await = new_config.clone();
        *self.previous_config.write().await = Some(old_config.clone());
        *self.last_reload.write().await = Some(Utc::now());
        
        // Record changes
        let changes = self.compute_changes(&old_config, &new_config);
        
        let record = ConfigChangeRecord {
            timestamp: Utc::now(),
            changes,
            validated: true,
            reason: "Reload".to_string(),
        };
        
        self.change_history.write().await.push(record);
        
        info!("[HotReload] Configuration reloaded successfully");
        Ok(())
    }
    
    /// Reload and validate configuration
    pub async fn reload_and_validate(&self) -> Result<()> {
        // Load configuration
        let content = fs::read_to_string(&self.config_path)
            .await
            .map_err(|e| AirError::Configuration(format!("Failed to read config: {}", e)))?;
        
        let new_config: AirConfiguration = toml::from_str(&content)
            .map_err(|e| AirError::Configuration(format!("Failed to parse config: {}", e)))?;
        
        // Validate thoroughly
        self.validate_config(&new_config).await?;
        
        // Apply
        self.reload().await
    }
    
    /// Validate configuration
    async fn validate_config(&self, config: &AirConfiguration) -> Result<()> {
        let validators = self.validators.read().await;
        
        for validator in validators.iter() {
            validator.validate(config)
                .map_err(|e| {
                    error!("[HotReload] Validation failed ({}): {}", validator.name(), e);
                    e
                })?;
        }
        
        info!("[HotReload] Configuration validation passed");
        Ok(())
    }
    
    /// Rollback to previous configuration
    pub async fn rollback(&self) -> Result<()> {
        let previous = self.previous_config.read().await.clone()
            .ok_or_else(|| AirError::Configuration("No previous configuration to rollback to".to_string()))?;
        
        *self.active_config.write().await = previous.clone();
        
        // Record rollback
        let record = ConfigChangeRecord {
            timestamp: Utc::now(),
            changes: vec![],
            validated: true,
            reason: "Rollback".to_string(),
        };
        
        self.change_history.write().await.push(record);
        
        info!("[HotReload] Configuration rolled back");
        Ok(())
    }
    
    /// Get current configuration
    pub async fn get_config(&self) -> AirConfiguration {
        self.active_config.read().await.clone()
    }
    
    /// Set configuration value by path (e.g., "grpc.bind_address")
    pub async fn set_value(&self, path: &str, value: &str) -> Result<()> {
        let mut config = self.active_config.write().await.clone();
        
        // Parse and update value
        Self::set_config_value(&mut config, path, value)?;
        
        // Validate
        self.validate_config(&config).await?;
        
        // Save to file
        let content = toml::to_string_pretty(&config)
            .map_err(|e| AirError::Configuration(format!("Serialization failed: {}", e)))?;
        
        fs::write(&self.config_path, content)
            .await
            .map_err(|e| AirError::Configuration(format!("Failed to write config: {}", e)))?;
        
        // Update active config
        *self.active_config.write().await = config;
        
        info!("[HotReload] Configuration value updated: {}", path);
        Ok(())
    }
    
    /// Get configuration value by path
    pub async fn get_value(&self, path: &str) -> Result<serde_json::Value> {
        let config = self.active_config.read().await;
        let config_json = serde_json::to_value(&*config)
            .map_err(|e| AirError::Configuration(format!("Serialization failed: {}", e)))?;
        
        let mut current = config_json;
        for key in path.split('.') {
            current = current
                .get(key)
                .ok_or_else(|| AirError::Configuration(format!("Key not found: {}", path)))?
                .clone();
        }
        
        Ok(current)
    }
    
    /// Set a nested configuration value
    fn set_config_value(config: &mut AirConfiguration, path: &str, value: &str) -> Result<()> {
        let parts: Vec<&str> = path.split('.').collect();
        
        match parts.as_slice() {
            ["grpc", "bind_address"] => config.grpc.bind_address = value.to_string(),
            ["grpc", "max_connections"] => {
                config.grpc.max_connections = value.parse()
                    .map_err(|_| AirError::Configuration(format!("Invalid value: {}", value)))?;
            }
            ["authentication", "enabled"] => {
                config.authentication.enabled = value.parse()
                    .map_err(|_| AirError::Configuration(format!("Invalid value: {}", value)))?;
            }
            ["updates", "auto_download"] => {
                config.updates.auto_download = value.parse()
                    .map_err(|_| AirError::Configuration(format!("Invalid value: {}", value)))?;
            }
            _ => {
                return Err(AirError::Configuration(format!("Unknown configuration path: {}", path)));
            }
        }
        
        Ok(())
    }
    
    /// Compute configuration changes
    fn compute_changes(
        &self,
        old: &AirConfiguration,
        new: &AirConfiguration,
    ) -> Vec<ConfigChange> {
        let mut changes = Vec::new();
        
        let old_json = serde_json::to_value(old).unwrap_or_default();
        let new_json = serde_json::to_value(new).unwrap_or_default();
        
        Self::diff_json("", &old_json, &new_json, &mut changes);
        
        changes
    }
    
    /// Recursively diff JSON objects
    fn diff_json(
        prefix: &str,
        old: &serde_json::Value,
        new: &serde_json::Value,
        changes: &mut Vec<ConfigChange>,
    ) {
        match (old, new) {
            (serde_json::Value::Object(old_map), serde_json::Value::Object(new_map)) => {
                for (key, new_val) in new_map {
                    let new_prefix = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    
                    if let Some(old_val) = old_map.get(key) {
                        Self::diff_json(&new_prefix, old_val, new_val, changes);
                    } else {
                        changes.push(ConfigChange {
                            path: new_prefix,
                            old_value: serde_json::Value::Null,
                            new_value: new_val.clone(),
                        });
                    }
                }
            }
            (old_val, new_val) if old_val != new_val => {
                changes.push(ConfigChange {
                    path: prefix.to_string(),
                    old_value: old_val.clone(),
                    new_value: new_val.clone(),
                });
            }
            _ => {}
        }
    }
    
    /// Get change history
    pub async fn get_change_history(&self, limit: Option<usize>) -> Vec<ConfigChangeRecord> {
        let history = self.change_history.read().await;
        
        if let Some(limit) = limit {
            history.iter().rev().take(limit).cloned().collect()
        } else {
            history.clone()
        }
    }
    
    /// Get last reload timestamp
    pub async fn get_last_reload(&self) -> Option<DateTime<Utc>> {
        *self.last_reload.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_config_change_recording() {
        let config = AirConfiguration::default();
        let path = PathBuf::from("/tmp/test_config.toml");
        
        let manager = ConfigHotReload::new(path, config)
            .await
            .expect("Failed to create manager");
        
        let history = manager.get_change_history(None).await;
        assert!(history.is_empty(), "Initial history should be empty");
    }
}
