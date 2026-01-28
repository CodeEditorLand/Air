//! # Configuration Management
//!
//! Handles configuration loading, validation, and management for the Air daemon.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};


use crate::{Result, AirError, DEFAULT_CONFIG_FILE};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirConfiguration {
    /// gRPC server configuration
    pub grpc: GrpcConfig,
    
    /// Authentication configuration
    pub authentication: AuthConfig,
    
    /// Update configuration
    pub updates: UpdateConfig,
    
    /// Download configuration
    pub downloader: DownloadConfig,
    
    /// Indexing configuration
    pub indexing: IndexingConfig,
    
    /// Logging configuration
    pub logging: LoggingConfig,
    
    /// Performance configuration
    pub performance: PerformanceConfig,
}

/// gRPC server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConfig {
    /// Bind address for gRPC server
    pub bind_address: String,
    
    /// Maximum concurrent connections
    pub max_connections: u32,
    
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Enable authentication service
    pub enabled: bool,
    
    /// Path to credentials storage
    pub credentials_path: String,
    
    /// Token expiration in hours
    pub token_expiration_hours: u32,
    
    /// Maximum concurrent auth sessions
    pub max_sessions: u32,
}

/// Update configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// Enable update service
    pub enabled: bool,
    
    /// Update check interval in hours
    pub check_interval_hours: u32,
    
    /// Update server URL
    pub update_server_url: String,
    
    /// Auto-download updates
    pub auto_download: bool,
    
    /// Auto-install updates
    pub auto_install: bool,
}

/// Download configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadConfig {
    /// Enable download service
    pub enabled: bool,
    
    /// Maximum concurrent downloads
    pub max_concurrent_downloads: u32,
    
    /// Download timeout in seconds
    pub download_timeout_secs: u64,
    
    /// Maximum retry attempts
    pub max_retries: u32,
    
    /// Download cache directory
    pub cache_directory: String,
}

/// Indexing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    /// Enable indexing service
    pub enabled: bool,
    
    /// Maximum file size to index (MB)
    pub max_file_size_mb: u32,
    
    /// File types to index
    pub file_types: Vec<String>,
    
    /// Index update interval in minutes
    pub update_interval_minutes: u32,
    
    /// Index storage directory
    pub index_directory: String,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    
    /// Log file path
    pub file_path: Option<String>,
    
    /// Enable console logging
    pub console_enabled: bool,
    
    /// Maximum log file size (MB)
    pub max_file_size_mb: u32,
    
    /// Maximum log files to keep
    pub max_files: u32,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Memory usage limit (MB)
    pub memory_limit_mb: u32,
    
    /// CPU usage limit (%)
    pub cpu_limit_percent: u32,
    
    /// Disk usage limit (MB)
    pub disk_limit_mb: u32,
    
    /// Background task interval in seconds
    pub background_task_interval_secs: u64,
}

impl Default for AirConfiguration {
    fn default() -> Self {
        Self {
            grpc: GrpcConfig {
                bind_address: "[::1]:50052".to_string(),
                max_connections: 100,
                request_timeout_secs: 30,
            },
            authentication: AuthConfig {
                enabled: true,
                credentials_path: "~/.air/credentials".to_string(),
                token_expiration_hours: 24,
                max_sessions: 10,
            },
            updates: UpdateConfig {
                enabled: true,
                check_interval_hours: 6,
                update_server_url: "https://updates.editor.land".to_string(),
                auto_download: true,
                auto_install: false,
            },
            downloader: DownloadConfig {
                enabled: true,
                max_concurrent_downloads: 5,
                download_timeout_secs: 300,
                max_retries: 3,
                cache_directory: "~/.air/cache".to_string(),
            },
            indexing: IndexingConfig {
                enabled: true,
                max_file_size_mb: 10,
                file_types: vec![
                    "*.rs".to_string(),
                    "*.ts".to_string(),
                    "*.js".to_string(),
                    "*.json".to_string(),
                    "*.toml".to_string(),
                    "*.md".to_string(),
                ],
                update_interval_minutes: 30,
                index_directory: "~/.air/index".to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file_path: Some("~/.air/logs/air.log".to_string()),
                console_enabled: true,
                max_file_size_mb: 10,
                max_files: 5,
            },
            performance: PerformanceConfig {
                memory_limit_mb: 512,
                cpu_limit_percent: 50,
                disk_limit_mb: 1024,
                background_task_interval_secs: 60,
            },
        }
    }
}

/// Configuration manager
pub struct ConfigurationManager {
    config_path: Option<PathBuf>,
}

impl ConfigurationManager {
    /// Create a new configuration manager
    pub fn new(config_path: Option<String>) -> Result<Self> {
        let path = config_path.map(PathBuf::from);
        Ok(Self { config_path: path })
    }
    
    /// Load configuration from file or create default
    pub async fn load_configuration(&self) -> Result<AirConfiguration> {
        // Try to load from specified path
        if let Some(ref path) = self.config_path {
            if path.exists() {
                return self.load_from_file(path).await;
            }
        }
        
        // Try to load from default location
        let default_path = Self::get_default_config_path()?;
        if default_path.exists() {
            return self.load_from_file(&default_path).await;
        }
        
        // Create default configuration
        log::info!("No configuration file found, creating default configuration");
        let config = AirConfiguration::default();
        
        // Save default configuration
        self.save_configuration(&config).await?;
        
        Ok(config)
    }
    
    /// Load configuration from file
    async fn load_from_file(&self, path: &Path) -> Result<AirConfiguration> {
        log::info!("Loading configuration from: {}", path.display());
        
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| AirError::Configuration(format!("Failed to read config file: {}", e)))?;
        
        let config: AirConfiguration = toml::from_str(&content)
            .map_err(|e| AirError::Configuration(format!("Failed to parse config file: {}", e)))?;
        
        // Validate configuration
        self.validate_configuration(&config)?;
        
        log::info!("Configuration loaded successfully");
        Ok(config)
    }
    
    /// Save configuration to file
    pub async fn save_configuration(&self, config: &AirConfiguration) -> Result<()> {
        let path = self.config_path
            .as_ref()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| Self::get_default_config_path().unwrap());
        
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| AirError::Configuration(format!("Failed to create config directory: {}", e)))?;
        }
        
        let content = toml::to_string_pretty(config)
            .map_err(|e| AirError::Configuration(format!("Failed to serialize config: {}", e)))?;
        
        tokio::fs::write(&path, content)
            .await
            .map_err(|e| AirError::Configuration(format!("Failed to write config file: {}", e)))?;
        
        log::info!("Configuration saved to: {}", path.display());
        Ok(())
    }
    
    /// Validate configuration values
    fn validate_configuration(&self, config: &AirConfiguration) -> Result<()> {
        // Validate gRPC configuration
        if config.grpc.bind_address.is_empty() {
            return Err(AirError::Configuration("gRPC bind address cannot be empty".to_string()));
        }
        
        if config.grpc.max_connections == 0 {
            return Err(AirError::Configuration("gRPC max connections must be greater than 0".to_string()));
        }
        
        // Validate authentication configuration
        if config.authentication.enabled && config.authentication.credentials_path.is_empty() {
            return Err(AirError::Configuration("Authentication credentials path cannot be empty".to_string()));
        }
        
        // Validate update configuration
        if config.updates.enabled && config.updates.update_server_url.is_empty() {
            return Err(AirError::Configuration("Update server URL cannot be empty".to_string()));
        }
        
        // Validate download configuration
        if config.downloader.enabled && config.downloader.cache_directory.is_empty() {
            return Err(AirError::Configuration("Download cache directory cannot be empty".to_string()));
        }
        
        // Validate indexing configuration
        if config.indexing.enabled && config.indexing.index_directory.is_empty() {
            return Err(AirError::Configuration("Index directory cannot be empty".to_string()));
        }
        
        Ok(())
    }
    
    /// Get default configuration file path
    fn get_default_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| AirError::Configuration("Cannot determine config directory".to_string()))?;
        
        Ok(config_dir.join("Air").join(DEFAULT_CONFIG_FILE))
    }
    
    /// Expand paths with home directory
    pub fn expand_path(path: &str) -> Result<PathBuf> {
        if path.starts_with('~') {
            let home = dirs::home_dir()
                .ok_or_else(|| AirError::Configuration("Cannot determine home directory".to_string()))?;
            
            let rest = &path[1..]; // Remove ~
            if rest.starts_with('/') || rest.starts_with('\\') {
                Ok(home.join(&rest[1..]))
            } else {
                Ok(home.join(rest))
            }
        } else {
            Ok(PathBuf::from(path))
        }
    }
}
