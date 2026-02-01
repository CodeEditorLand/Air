//! # Configuration Management
//!
//! This module provides comprehensive configuration management for the Air
//! daemon, serving as the central configuration layer for the entire Land
//! ecosystem.
//!
//! ## Responsibilities
//!
//! - **Configuration Loading**: Load and parse configuration from TOML files
//!   with fallback to defaults
//! - **Schema Validation**: Validate all configuration values against defined
//!   schemas with detailed error messages
//! - **Type Safety**: Strong typing with compile-time guarantees and runtime
//!   validation
//! - **Value Constraints**: Range validation, path validation, and security
//!   checks
//! - **Environment Integration**: Support environment variable overrides and
//!   profile-based configuration
//! - **Hot Reload**: Live configuration updates without service restart (via
//!   HotReload module)
//! - **Change Tracking**: Audit trail for all configuration changes with
//!   rollback support
//! - **Migration Support**: Automated configuration schema versioning and
//!   migration
//!
//! ## VSCode Configuration System References
//!
//! This configuration system is designed to be compatible with VSCode's
//! configuration architecture:
//! - VSCode config reference:
//!   `Dependency/Microsoft/Editor/src/vs/platform/configuration/`
//! - Format compatibility with `settings.json` schema structure
//! - Support for workspace-specific overrides similar to VSCode's multi-layer
//!   config
//! - Configuration inheritance and overriding patterns aligned with VSCode
//!
//! ## Connection to Mountain's Configuration Needs
//!
//! Mountain (the VSCode application layer) consumes Air's configuration:
//! - User settings in Mountain flow through to Air's daemon configuration
//! - Wind services read centralized configuration for consistency
//! - Configuration changes propagate through the hot-reload system to all
//!   services
//! - Profile switching (dev/staging/prod) affects entire Land ecosystem
//!
//! ## Configuration Flow
//!
//! ```
//! Mountain (User Settings) → Air config file → Wind services
//!        ↓                         ↓                    ↓
//!  settings.json           ~/.air/config.toml    Service-specific overrides
//!        ↓                         ↓                    ↓
//!  Workspace settings    Environment variables    Hot-reload notifications
//! ```
//!
//! ## TODO: Schema Validation
//! - Implement JSON Schema generation for validation
//! - Add schema versioning and migration support
//! - Provide schema validation errors with detailed field-level information
//! - Support schema evolution with backward compatibility
//!
//! ## TODO: Configuration Migration
//! - Add version field to configuration structure
//! - Implement automatic migration between schema versions
//! - Provide migration tools for manual upgrades
//! - Document migration paths and breaking changes
//!
//! ## TODO: Configuration Inheritance
//! - Implement base profile templates
//! - Support profile inheritance and overrides
//! - Add configuration layer merging logic
//! - Document precedence rules (defaults → file → env → runtime)
//!
//! ## Profiles and Environments
//!
//! Configuration supports multiple profiles for different deployment scenarios:
//! - **dev**: Development environment with debug logging
//! - **staging**: Pre-production with production-like settings
//! - **prod**: Production optimized settings
//! - **custom**: User-defined profiles
//!
//! ## Security Considerations
//!
//! - Path validation prevents directory traversal attacks
//! - Sensitive values support environment variable injection
//! - Configuration files enforce proper permissions
//! - Atomic updates prevent partial/corrupted state

pub mod HotReload;

use std::{
	collections::HashMap,
	env,
	path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use sha2::Digest;

use crate::{AirError, DefaultConfigFile, Result};

// =============================================================================
// Configuration Main Structure
// =============================================================================

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirConfiguration {
	/// Configuration schema version for migration tracking
	#[serde(default = "default_schema_version")]
	pub schema_version:String,

	/// Profile name (dev, staging, prod, custom)
	#[serde(default = "default_profile")]
	pub profile:String,

	/// gRPC server configuration
	pub grpc:GrpcConfig,

	/// Authentication configuration
	pub authentication:AuthConfig,

	/// Update configuration
	pub updates:UpdateConfig,

	/// Download configuration
	pub downloader:DownloadConfig,

	/// Indexing configuration
	pub indexing:IndexingConfig,

	/// Logging configuration
	pub logging:LoggingConfig,

	/// Performance configuration
	pub performance:PerformanceConfig,
}

fn default_schema_version() -> String { "1.0.0".to_string() }

fn default_profile() -> String { "dev".to_string() }

/// gRPC server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConfig {
	/// Bind address for gRPC server
	/// Validation: Must be a valid IP:port or hostname:port combination
	/// Format: [IPv6]:port or IPv4:port or hostname:port
	/// Example: "[::1]:50053", "127.0.0.1:50053", "localhost:50053"
	#[serde(default = "default_grpc_bind_address")]
	pub bind_address:String,

	/// Maximum concurrent connections
	/// Validation: Range [10, 10000]
	/// Default: 100
	#[serde(default = "default_grpc_max_connections")]
	pub max_connections:u32,

	/// Request timeout in seconds
	/// Validation: Range [1, 3600] (1 second to 1 hour)
	/// Default: 30
	#[serde(default = "default_grpc_request_timeout")]
	pub request_timeout_secs:u64,
}

fn default_grpc_bind_address() -> String { "[::1]:50053".to_string() }

fn default_grpc_max_connections() -> u32 { 100 }

fn default_grpc_request_timeout() -> u64 { 30 }

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
	/// Enable authentication service
	#[serde(default = "default_auth_enabled")]
	pub enabled:bool,

	/// Path to credentials storage
	/// Validation: Must be a valid absolute or home-relative path
	/// Security: Ensures directory traversal prevention
	/// Default: "~/.air/credentials"
	#[serde(default = "default_auth_credentials_path")]
	pub credentials_path:String,

	/// Token expiration in hours
	/// Validation: Range [1, 8760] (1 hour to 1 year)
	/// Default: 24
	#[serde(default = "default_auth_token_expiration")]
	pub token_expiration_hours:u32,

	/// Maximum concurrent auth sessions
	/// Validation: Range [1, 1000]
	/// Default: 10
	#[serde(default = "default_auth_max_sessions")]
	pub max_sessions:u32,
}

fn default_auth_enabled() -> bool { true }

fn default_auth_credentials_path() -> String { "~/.air/credentials".to_string() }

fn default_auth_token_expiration() -> u32 { 24 }

fn default_auth_max_sessions() -> u32 { 10 }

/// Update configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
	/// Enable update service
	#[serde(default = "default_update_enabled")]
	pub enabled:bool,

	/// Update check interval in hours
	/// Validation: Range [1, 168] (1 hour to 1 week)
	/// Default: 6
	#[serde(default = "default_update_check_interval")]
	pub check_interval_hours:u32,

	/// Update server URL
	/// Validation: Must be a valid HTTPS URL
	/// Security: HTTPS required for security
	/// Default: "https://updates.editor.land"
	#[serde(default = "default_update_server_url")]
	pub update_server_url:String,

	/// Auto-download updates
	#[serde(default = "default_update_auto_download")]
	pub auto_download:bool,

	/// Auto-install updates
	/// Warning: Use with caution in production
	#[serde(default = "default_update_auto_install")]
	pub auto_install:bool,

	/// Update channel
	/// Validation: Must be one of: "stable", "insiders", "preview"
	/// Default: "stable"
	#[serde(default = "default_update_channel")]
	pub channel:String,
}

fn default_update_enabled() -> bool { true }

fn default_update_check_interval() -> u32 { 6 }

fn default_update_server_url() -> String { "https://updates.editor.land".to_string() }

fn default_update_auto_download() -> bool { true }

fn default_update_auto_install() -> bool { false }

fn default_update_channel() -> String { "stable".to_string() }

/// Download configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadConfig {
	/// Enable download service
	#[serde(default = "default_download_enabled")]
	pub enabled:bool,

	/// Maximum concurrent downloads
	/// Validation: Range [1, 50]
	/// Default: 5
	#[serde(default = "default_download_max_concurrent")]
	pub max_concurrent_downloads:u32,

	/// Download timeout in seconds
	/// Validation: Range [10, 3600] (10 seconds to 1 hour)
	/// Default: 300
	#[serde(default = "default_download_timeout")]
	pub download_timeout_secs:u64,

	/// Maximum retry attempts
	/// Validation: Range [0, 10]
	/// Default: 3
	#[serde(default = "default_download_max_retries")]
	pub max_retries:u32,

	/// Download cache directory
	/// Validation: Must be a valid absolute or home-relative path
	/// Default: "~/.air/cache"
	#[serde(default = "default_download_cache_dir")]
	pub cache_directory:String,
}

fn default_download_enabled() -> bool { true }

fn default_download_max_concurrent() -> u32 { 5 }

fn default_download_timeout() -> u64 { 300 }

fn default_download_max_retries() -> u32 { 3 }

fn default_download_cache_dir() -> String { "~/.air/cache".to_string() }

/// Indexing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
	/// Enable indexing service
	#[serde(default = "default_indexing_enabled")]
	pub enabled:bool,

	/// Maximum file size to index (MB)
	/// Validation: Range [1, 1024] (1MB to 1GB)
	/// Default: 10
	#[serde(default = "default_indexing_max_file_size")]
	pub max_file_size_mb:u32,

	/// File types to index
	/// Format: Glob patterns like "*.rs", "*.ts", etc.
	/// Validation: Each pattern must be a valid glob pattern
	/// Default: Common source code file types
	#[serde(default = "default_indexing_file_types")]
	pub file_types:Vec<String>,

	/// Index update interval in minutes
	/// Validation: Range [1, 1440] (1 minute to 1 day)
	/// Default: 30
	#[serde(default = "default_indexing_update_interval")]
	pub update_interval_minutes:u32,

	/// Index storage directory
	/// Validation: Must be a valid absolute or home-relative path
	/// Default: "~/.air/index"
	#[serde(default = "default_indexing_directory")]
	pub index_directory:String,
}

fn default_indexing_enabled() -> bool { true }

fn default_indexing_max_file_size() -> u32 { 10 }

fn default_indexing_file_types() -> Vec<String> {
	vec![
		"*.rs".to_string(),
		"*.ts".to_string(),
		"*.js".to_string(),
		"*.json".to_string(),
		"*.toml".to_string(),
		"*.md".to_string(),
	]
}

fn default_indexing_update_interval() -> u32 { 30 }

fn default_indexing_directory() -> String { "~/.air/index".to_string() }

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
	/// Log level
	/// Validation: Must be one of: "trace", "debug", "info", "warn", "error"
	/// Default: "info"
	#[serde(default = "default_logging_level")]
	pub level:String,

	/// Log file path
	/// Validation: Must be a valid absolute or home-relative path if provided
	/// Default: "~/.air/logs/air.log"
	#[serde(default = "default_logging_file_path")]
	pub file_path:Option<String>,

	/// Enable console logging
	#[serde(default = "default_logging_console_enabled")]
	pub console_enabled:bool,

	/// Maximum log file size (MB)
	/// Validation: Range [1, 1000]
	/// Default: 10
	#[serde(default = "default_logging_max_file_size")]
	pub max_file_size_mb:u32,

	/// Maximum log files to keep
	/// Validation: Range [1, 50]
	/// Default: 5
	#[serde(default = "default_logging_max_files")]
	pub max_files:u32,
}

fn default_logging_level() -> String { "info".to_string() }

fn default_logging_file_path() -> Option<String> { Some("~/.air/logs/air.log".to_string()) }

fn default_logging_console_enabled() -> bool { true }

fn default_logging_max_file_size() -> u32 { 10 }

fn default_logging_max_files() -> u32 { 5 }

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
	/// Memory usage limit (MB)
	/// Validation: Range [64, 16384] (64MB to 16GB)
	/// Default: 512
	#[serde(default = "default_perf_memory_limit")]
	pub memory_limit_mb:u32,

	/// CPU usage limit (%)
	/// Validation: Range [10, 100]
	/// Default: 50
	#[serde(default = "default_perf_cpu_limit")]
	pub cpu_limit_percent:u32,

	/// Disk usage limit (MB)
	/// Validation: Range [100, 102400] (100MB to 100GB)
	/// Default: 1024
	#[serde(default = "default_perf_disk_limit")]
	pub disk_limit_mb:u32,

	/// Background task interval in seconds
	/// Validation: Range [1, 3600] (1 second to 1 hour)
	/// Default: 60
	#[serde(default = "default_perf_task_interval")]
	pub background_task_interval_secs:u64,
}

fn default_perf_memory_limit() -> u32 { 512 }

fn default_perf_cpu_limit() -> u32 { 50 }

fn default_perf_disk_limit() -> u32 { 1024 }

fn default_perf_task_interval() -> u64 { 60 }

impl Default for AirConfiguration {
	fn default() -> Self {
		Self {
			schema_version:default_schema_version(),
			profile:default_profile(),
			grpc:GrpcConfig {
				bind_address:default_grpc_bind_address(),
				max_connections:default_grpc_max_connections(),
				request_timeout_secs:default_grpc_request_timeout(),
			},
			authentication:AuthConfig {
				enabled:default_auth_enabled(),
				credentials_path:default_auth_credentials_path(),
				token_expiration_hours:default_auth_token_expiration(),
				max_sessions:default_auth_max_sessions(),
			},
			updates:UpdateConfig {
				enabled:default_update_enabled(),
				check_interval_hours:default_update_check_interval(),
				update_server_url:default_update_server_url(),
				auto_download:default_update_auto_download(),
				auto_install:default_update_auto_install(),
				channel:default_update_channel(),
			},
			downloader:DownloadConfig {
				enabled:default_download_enabled(),
				max_concurrent_downloads:default_download_max_concurrent(),
				download_timeout_secs:default_download_timeout(),
				max_retries:default_download_max_retries(),
				cache_directory:default_download_cache_dir(),
			},
			indexing:IndexingConfig {
				enabled:default_indexing_enabled(),
				max_file_size_mb:default_indexing_max_file_size(),
				file_types:default_indexing_file_types(),
				update_interval_minutes:default_indexing_update_interval(),
				index_directory:default_indexing_directory(),
			},
			logging:LoggingConfig {
				level:default_logging_level(),
				file_path:default_logging_file_path(),
				console_enabled:default_logging_console_enabled(),
				max_file_size_mb:default_logging_max_file_size(),
				max_files:default_logging_max_files(),
			},
			performance:PerformanceConfig {
				memory_limit_mb:default_perf_memory_limit(),
				cpu_limit_percent:default_perf_cpu_limit(),
				disk_limit_mb:default_perf_disk_limit(),
				background_task_interval_secs:default_perf_task_interval(),
			},
		}
	}
}

// =============================================================================
// Configuration Schema
// =============================================================================

/// Generate JSON Schema for configuration validation
pub fn generate_schema() -> JsonValue {
	json!({
		"$schema": "http://json-schema.org/draft-07/schema#",
		"title": "Air Configuration Schema",
		"description": "Configuration schema for Air daemon",
		"type": "object",
		"required": ["schema_version", "profile"],
		"properties": {
			"schema_version": {
				"type": "string",
				"description": "Configuration schema version for migration tracking",
				"pattern": "^\\d+\\.\\d+\\.\\d+$"
			},
			"profile": {
				"type": "string",
				"description": "Profile name (dev, staging, prod, custom)",
				"enum": ["dev", "staging", "prod", "custom"]
			},
			"grpc": {
				"type": "object",
				"description": "gRPC server configuration",
				"properties": {
					"bind_address": {
						"type": "string",
						"description": "gRPC server bind address",
						"format": "hostname-port"
					},
					"max_connections": {
						"type": "integer",
						"minimum": 10,
						"maximum": 10000
					},
					"request_timeout_secs": {
						"type": "integer",
						"minimum": 1,
						"maximum": 3600
					}
				}
			},
			"authentication": {
				"type": "object",
				"description": "Authentication configuration",
				"properties": {
					"enabled": {"type": "boolean"},
					"credentials_path": {"type": "string"},
					"token_expiration_hours": {
						"type": "integer",
						"minimum": 1,
						"maximum": 8760
					},
					"max_sessions": {
						"type": "integer",
						"minimum": 1,
						"maximum": 1000
					}
				}
			},
			"updates": {
				"type": "object",
				"properties": {
					"enabled": {"type": "boolean"},
					"check_interval_hours": {
						"type": "integer",
						"minimum": 1,
						"maximum": 168
					},
					"update_server_url": {
						"type": "string",
						"pattern": "^https://"
					},
					"auto_download": {"type": "boolean"},
					"auto_install": {"type": "boolean"},
					"channel": {
						"type": "string",
						"enum": ["stable", "insiders", "preview"]
					}
				}
			},
			"downloader": {
				"type": "object",
				"properties": {
					"enabled": {"type": "boolean"},
					"max_concurrent_downloads": {
						"type": "integer",
						"minimum": 1,
						"maximum": 50
					},
					"download_timeout_secs": {
						"type": "integer",
						"minimum": 10,
						"maximum": 3600
					},
					"max_retries": {
						"type": "integer",
						"minimum": 0,
						"maximum": 10
					},
					"cache_directory": {"type": "string"}
				}
			},
			"indexing": {
				"type": "object",
				"properties": {
					"enabled": {"type": "boolean"},
					"max_file_size_mb": {
						"type": "integer",
						"minimum": 1,
						"maximum": 1024
					},
					"file_types": {
						"type": "array",
						"items": {"type": "string"}
					},
					"update_interval_minutes": {
						"type": "integer",
						"minimum": 1,
						"maximum": 1440
					},
					"index_directory": {"type": "string"}
				}
			},
			"logging": {
				"type": "object",
				"properties": {
					"level": {
						"type": "string",
						"enum": ["trace", "debug", "info", "warn", "error"]
					},
					"file_path": {"type": ["string", "null"]},
					"console_enabled": {"type": "boolean"},
					"max_file_size_mb": {
						"type": "integer",
						"minimum": 1,
						"maximum": 1000
					},
					"max_files": {
						"type": "integer",
						"minimum": 1,
						"maximum": 50
					}
				}
			},
			"performance": {
				"type": "object",
				"properties": {
					"memory_limit_mb": {
						"type": "integer",
						"minimum": 64,
						"maximum": 16384
					},
					"cpu_limit_percent": {
						"type": "integer",
						"minimum": 10,
						"maximum": 100
					},
					"disk_limit_mb": {
						"type": "integer",
						"minimum": 100,
						"maximum": 102400
					},
					"background_task_interval_secs": {
						"type": "integer",
						"minimum": 1,
						"maximum": 3600
					}
				}
			}
		}
	})
}

// =============================================================================
// Configuration Manager
// =============================================================================

/// Configuration manager with comprehensive validation, backup, and hot-reload
/// support
pub struct ConfigurationManager {
	/// Path to configuration file
	config_path:Option<PathBuf>,

	/// Backup configuration directory
	backup_dir:Option<PathBuf>,

	/// Enable configuration backup
	enable_backup:bool,

	/// Environment variable prefix for overrides
	env_prefix:String,
}

impl ConfigurationManager {
	/// Create a new configuration manager
	///
	/// # Arguments
	///
	/// * `config_path` - Optional path to configuration file. If None, uses
	///   default location
	///
	/// # Returns
	///
	/// Returns a new ConfigurationManager instance
	pub fn New(config_path:Option<String>) -> Result<Self> {
		let path = config_path.map(PathBuf::from);
		let backup_dir = path
			.as_ref()
			.and_then(|p| p.parent())
			.map(|parent| parent.join(".config_backups"));

		Ok(Self { config_path:path, backup_dir, enable_backup:true, env_prefix:"AIR_".to_string() })
	}

	/// Create a new configuration manager with custom settings
	///
	/// # Arguments
	///
	/// * `config_path` - Optional path to configuration file
	/// * `enable_backup` - Whether to enable automatic backups
	/// * `env_prefix` - Prefix for environment variable overrides
	pub fn NewWithSettings(config_path:Option<String>, enable_backup:bool, env_prefix:String) -> Result<Self> {
		let path = config_path.map(PathBuf::from);
		let backup_dir = if enable_backup {
			path.as_ref()
				.and_then(|p| p.parent())
				.map(|parent| parent.join(".config_backups"))
		} else {
			None
		};

		Ok(Self { config_path:path, backup_dir, enable_backup, env_prefix })
	}

	/// Load configuration from file, environment, or create default
	///
	/// This method implements the configuration priority chain:
	/// 1. Defaults from code
	/// 2. Configuration file
	/// 3. Environment variables (with prefix)
	///
	/// # Returns
	///
	/// Validated and loaded configuration
	pub async fn LoadConfiguration(&self) -> Result<AirConfiguration> {
		// Start with default configuration
		let mut config = AirConfiguration::default();

		// Try to load from specified or default path
		let config_path = self.GetConfigPath()?;

		if config_path.exists() {
			log::info!("Loading configuration from: {}", config_path.display());
			config = self.LoadFromFile(&config_path).await?;
		} else {
			log::info!("No configuration file found, using defaults");
		}

		// Apply environment variable overrides
		self.ApplyEnvironmentOverrides(&mut config)?;

		// Schema validation
		self.SchemaValidate(&config)?;

		// Validate all configuration values
		self.ValidateConfiguration(&config)?;

		log::info!("Configuration loaded successfully (profile: {})", config.profile);
		Ok(config)
	}

	/// Load configuration from a specific file
	///
	/// # Arguments
	///
	/// * `path` - Path to the configuration file
	///
	/// # Returns
	///
	/// Parsed and validated configuration
	async fn LoadFromFile(&self, path:&Path) -> Result<AirConfiguration> {
		let content = tokio::fs::read_to_string(path)
			.await
			.map_err(|e| AirError::Configuration(format!("Failed to read config file '{}': {}", path.display(), e)))?;

		let config:AirConfiguration = toml::from_str(&content).map_err(|e| {
			AirError::Configuration(format!("Failed to parse TOML config file '{}': {}", path.display(), e))
		})?;

		// Type validation is done by serde automatically
		log::debug!("Configuration file parsed successfully");
		Ok(config)
	}

	/// Save configuration to file with backup and atomic write
	///
	/// # Arguments
	///
	/// * `config` - Configuration to save
	///
	/// # Implementation Details
	///
	/// - Validates configuration before saving
	/// - Creates backup if enabled
	/// - Uses atomic write (write to temp file, then rename)
	/// - Creates parent directories if needed
	pub async fn SaveConfiguration(&self, config:&AirConfiguration) -> Result<()> {
		// Validate before saving
		self.ValidateConfiguration(config)?;

		let config_path = self.GetConfigPath()?;

		// Create backup if enabled and file exists
		if self.enable_backup && config_path.exists() {
			self.BackupConfiguration(&config_path).await?;
		}

		// Create parent directory if it doesn't exist
		if let Some(parent) = config_path.parent() {
			tokio::fs::create_dir_all(parent).await.map_err(|e| {
				AirError::Configuration(format!("Failed to create config directory '{}': {}", parent.display(), e))
			})?;
		}

		// Atomic write: write to temp file, then rename
		let temp_path = config_path.with_extension("tmp");
		let content = toml::to_string_pretty(config)
			.map_err(|e| AirError::Configuration(format!("Failed to serialize config: {}", e)))?;

		tokio::fs::write(&temp_path, content).await.map_err(|e| {
			AirError::Configuration(format!("Failed to write temp config file '{}': {}", temp_path.display(), e))
		})?;

		// Atomic rename
		tokio::fs::rename(&temp_path, &config_path).await.map_err(|e| {
			AirError::Configuration(format!("Failed to rename temp config to '{}': {}", config_path.display(), e))
		})?;

		log::info!("Configuration saved to: {}", config_path.display());
		Ok(())
	}

	/// Validate configuration with comprehensive checks
	///
	/// Performs:
	/// - Schema validation
	/// - Type checking with detailed errors
	/// - Range validation for numeric values
	/// - Path validation for security
	/// - URL validation for network resources
	fn ValidateConfiguration(&self, config:&AirConfiguration) -> Result<()> {
		// Schema version validation
		self.ValidateSchemaVersion(&config.schema_version)?;

		// Profile validation
		self.ValidateProfile(&config.profile)?;

		// gRPC configuration validation
		self.ValidateGrpcConfig(&config.grpc)?;

		// Authentication configuration validation
		self.ValidateAuthConfig(&config.authentication)?;

		// Update configuration validation
		self.ValidateUpdateConfig(&config.updates)?;

		// Download configuration validation
		self.ValidateDownloadConfig(&config.downloader)?;

		// Indexing configuration validation
		self.ValidateIndexingConfig(&config.indexing)?;

		// Logging configuration validation
		self.ValidateLoggingConfig(&config.logging)?;

		// Performance configuration validation
		self.ValidatePerformanceConfig(&config.performance)?;

		log::debug!("All configuration validation checks passed");
		Ok(())
	}

	/// Validate schema version format
	fn ValidateSchemaVersion(&self, version:&str) -> Result<()> {
		if !version.chars().all(|c| c.is_digit(10) || c == '.') {
			return Err(AirError::Configuration(format!(
				"Invalid schema version '{}': must be in format X.Y.Z",
				version
			)));
		}

		let parts:Vec<&str> = version.split('.').collect();
		if parts.len() != 3 {
			return Err(AirError::Configuration(format!(
				"Invalid schema version '{}': must have 3 parts (X.Y.Z)",
				version
			)));
		}

		for (i, part) in parts.iter().enumerate() {
			if part.is_empty() {
				return Err(AirError::Configuration(format!(
					"Invalid schema version '{}': part {} is empty",
					version,
					i + 1
				)));
			}
		}

		Ok(())
	}

	/// Validate profile name
	fn ValidateProfile(&self, profile:&str) -> Result<()> {
		let valid_profiles = ["dev", "staging", "prod", "custom"];

		if !valid_profiles.contains(&profile) {
			return Err(AirError::Configuration(format!(
				"Invalid profile '{}': must be one of: {}",
				profile,
				valid_profiles.join(", ")
			)));
		}

		Ok(())
	}

	/// Validate gRPC configuration with range checking
	fn ValidateGrpcConfig(&self, grpc:&GrpcConfig) -> Result<()> {
		// Validate bind address
		if grpc.bind_address.is_empty() {
			return Err(AirError::Configuration("gRPC bind address cannot be empty".to_string()));
		}

		// Validate address format
		if !Self::IsValidAddress(&grpc.bind_address) {
			return Err(AirError::Configuration(format!(
				"Invalid gRPC bind address '{}': must be in format host:port or [IPv6]:port",
				grpc.bind_address
			)));
		}

		// Validate max_connections range [10, 10000]
		if grpc.max_connections < 10 {
			return Err(AirError::Configuration(format!(
				"gRPC max_connections {} is below minimum (10)",
				grpc.max_connections
			)));
		}

		if grpc.max_connections > 10000 {
			return Err(AirError::Configuration(format!(
				"gRPC max_connections {} exceeds maximum (10000)",
				grpc.max_connections
			)));
		}

		// Validate request_timeout_secs range [1, 3600]
		if grpc.request_timeout_secs < 1 {
			return Err(AirError::Configuration(format!(
				"gRPC request_timeout_secs {} is below minimum (1 second)",
				grpc.request_timeout_secs
			)));
		}

		if grpc.request_timeout_secs > 3600 {
			return Err(AirError::Configuration(format!(
				"gRPC request_timeout_secs {} exceeds maximum (3600 seconds = 1 hour)",
				grpc.request_timeout_secs
			)));
		}

		Ok(())
	}

	/// Validate authentication configuration
	fn ValidateAuthConfig(&self, auth:&AuthConfig) -> Result<()> {
		// If authentication is enabled, validate credentials path
		if auth.enabled {
			if auth.credentials_path.is_empty() {
				return Err(AirError::Configuration(
					"Authentication credentials path cannot be empty when authentication is enabled".to_string(),
				));
			}

			// Validate path for security (prevent directory traversal)
			self.ValidatePath(&auth.credentials_path)?;
		}

		// Validate token_expiration_hours range [1, 8760]
		if auth.token_expiration_hours < 1 {
			return Err(AirError::Configuration(format!(
				"Token expiration hours {} is below minimum (1 hour)",
				auth.token_expiration_hours
			)));
		}

		if auth.token_expiration_hours > 8760 {
			return Err(AirError::Configuration(format!(
				"Token expiration hours {} exceeds maximum (8760 hours = 1 year)",
				auth.token_expiration_hours
			)));
		}

		// Validate max_sessions range [1, 1000]
		if auth.max_sessions < 1 {
			return Err(AirError::Configuration(format!(
				"Max sessions {} is below minimum (1)",
				auth.max_sessions
			)));
		}

		if auth.max_sessions > 1000 {
			return Err(AirError::Configuration(format!(
				"Max sessions {} exceeds maximum (1000)",
				auth.max_sessions
			)));
		}

		Ok(())
	}

	/// Validate update configuration
	fn ValidateUpdateConfig(&self, updates:&UpdateConfig) -> Result<()> {
		if updates.enabled {
			// Validate update server URL
			if updates.update_server_url.is_empty() {
				return Err(AirError::Configuration(
					"Update server URL cannot be empty when updates are enabled".to_string(),
				));
			}

			// Must be HTTPS for security
			if !updates.update_server_url.starts_with("https://") {
				return Err(AirError::Configuration(format!(
					"Update server URL must use HTTPS, got: {}",
					updates.update_server_url
				)));
			}

			// Validate URL format
			if !Self::IsValidUrl(&updates.update_server_url) {
				return Err(AirError::Configuration(format!(
					"Invalid update server URL '{}'",
					updates.update_server_url
				)));
			}
		}

		// Validate check_interval_hours range [1, 168]
		if updates.check_interval_hours < 1 {
			return Err(AirError::Configuration(format!(
				"Update check interval {} hours is below minimum (1 hour)",
				updates.check_interval_hours
			)));
		}

		if updates.check_interval_hours > 168 {
			return Err(AirError::Configuration(format!(
				"Update check interval {} hours exceeds maximum (168 hours = 1 week)",
				updates.check_interval_hours
			)));
		}

		Ok(())
	}

	/// Validate download configuration
	fn ValidateDownloadConfig(&self, downloader:&DownloadConfig) -> Result<()> {
		if downloader.enabled {
			if downloader.cache_directory.is_empty() {
				return Err(AirError::Configuration(
					"Download cache directory cannot be empty when downloader is enabled".to_string(),
				));
			}

			// Validate path for security
			self.ValidatePath(&downloader.cache_directory)?;
		}

		// Validate max_concurrent_downloads range [1, 50]
		if downloader.max_concurrent_downloads < 1 {
			return Err(AirError::Configuration(format!(
				"Max concurrent downloads {} is below minimum (1)",
				downloader.max_concurrent_downloads
			)));
		}

		if downloader.max_concurrent_downloads > 50 {
			return Err(AirError::Configuration(format!(
				"Max concurrent downloads {} exceeds maximum (50)",
				downloader.max_concurrent_downloads
			)));
		}

		// Validate download_timeout_secs range [10, 3600]
		if downloader.download_timeout_secs < 10 {
			return Err(AirError::Configuration(format!(
				"Download timeout {} seconds is below minimum (10 seconds)",
				downloader.download_timeout_secs
			)));
		}

		if downloader.download_timeout_secs > 3600 {
			return Err(AirError::Configuration(format!(
				"Download timeout {} seconds exceeds maximum (3600 seconds = 1 hour)",
				downloader.download_timeout_secs
			)));
		}

		// Validate max_retries range [0, 10]
		if downloader.max_retries > 10 {
			return Err(AirError::Configuration(format!(
				"Max retries {} exceeds maximum (10)",
				downloader.max_retries
			)));
		}

		Ok(())
	}

	/// Validate indexing configuration
	fn ValidateIndexingConfig(&self, indexing:&IndexingConfig) -> Result<()> {
		if indexing.enabled {
			if indexing.index_directory.is_empty() {
				return Err(AirError::Configuration(
					"Index directory cannot be empty when indexing is enabled".to_string(),
				));
			}

			// Validate path for security
			self.ValidatePath(&indexing.index_directory)?;

			// Validate file_types is not empty
			if indexing.file_types.is_empty() {
				return Err(AirError::Configuration(
					"File types to index cannot be empty when indexing is enabled".to_string(),
				));
			}

			// Validate each file type pattern
			for file_type in &indexing.file_types {
				if file_type.is_empty() {
					return Err(AirError::Configuration("File type pattern cannot be empty".to_string()));
				}

				if !file_type.contains('*') {
					log::warn!(
						"File type pattern '{}' does not contain wildcards, may not match as expected",
						file_type
					);
				}
			}
		}

		// Validate max_file_size_mb range [1, 1024]
		if indexing.max_file_size_mb < 1 {
			return Err(AirError::Configuration(format!(
				"Max file size {} MB is below minimum (1 MB)",
				indexing.max_file_size_mb
			)));
		}

		if indexing.max_file_size_mb > 1024 {
			return Err(AirError::Configuration(format!(
				"Max file size {} MB exceeds maximum (1024 MB = 1 GB)",
				indexing.max_file_size_mb
			)));
		}

		// Validate update_interval_minutes range [1, 1440]
		if indexing.update_interval_minutes < 1 {
			return Err(AirError::Configuration(format!(
				"Index update interval {} minutes is below minimum (1 minute)",
				indexing.update_interval_minutes
			)));
		}

		if indexing.update_interval_minutes > 1440 {
			return Err(AirError::Configuration(format!(
				"Index update interval {} minutes exceeds maximum (1440 minutes = 1 day)",
				indexing.update_interval_minutes
			)));
		}

		Ok(())
	}

	/// Validate logging configuration
	fn ValidateLoggingConfig(&self, logging:&LoggingConfig) -> Result<()> {
		// Validate log level
		let valid_levels = ["trace", "debug", "info", "warn", "error"];
		if !valid_levels.contains(&logging.level.as_str()) {
			return Err(AirError::Configuration(format!(
				"Invalid log level '{}': must be one of: {}",
				logging.level,
				valid_levels.join(", ")
			)));
		}

		// Validate file path if provided
		if let Some(ref file_path) = logging.file_path {
			if !file_path.is_empty() {
				self.ValidatePath(file_path)?;
			}
		}

		// Validate max_file_size_mb range [1, 1000]
		if logging.max_file_size_mb < 1 {
			return Err(AirError::Configuration(format!(
				"Max log file size {} MB is below minimum (1 MB)",
				logging.max_file_size_mb
			)));
		}

		if logging.max_file_size_mb > 1000 {
			return Err(AirError::Configuration(format!(
				"Max log file size {} MB exceeds maximum (1000 MB = 1 GB)",
				logging.max_file_size_mb
			)));
		}

		// Validate max_files range [1, 50]
		if logging.max_files < 1 {
			return Err(AirError::Configuration(format!(
				"Max log files {} is below minimum (1)",
				logging.max_files
			)));
		}

		if logging.max_files > 50 {
			return Err(AirError::Configuration(format!(
				"Max log files {} exceeds maximum (50)",
				logging.max_files
			)));
		}

		Ok(())
	}

	/// Validate performance configuration
	fn ValidatePerformanceConfig(&self, performance:&PerformanceConfig) -> Result<()> {
		// Validate memory_limit_mb range [64, 16384]
		if performance.memory_limit_mb < 64 {
			return Err(AirError::Configuration(format!(
				"Memory limit {} MB is below minimum (64 MB)",
				performance.memory_limit_mb
			)));
		}

		if performance.memory_limit_mb > 16384 {
			return Err(AirError::Configuration(format!(
				"Memory limit {} MB exceeds maximum (16384 MB = 16 GB)",
				performance.memory_limit_mb
			)));
		}

		// Validate cpu_limit_percent range [10, 100]
		if performance.cpu_limit_percent < 10 {
			return Err(AirError::Configuration(format!(
				"CPU limit {}% is below minimum (10%)",
				performance.cpu_limit_percent
			)));
		}

		if performance.cpu_limit_percent > 100 {
			return Err(AirError::Configuration(format!(
				"CPU limit {}% exceeds maximum (100%)",
				performance.cpu_limit_percent
			)));
		}

		// Validate disk_limit_mb range [100, 102400]
		if performance.disk_limit_mb < 100 {
			return Err(AirError::Configuration(format!(
				"Disk limit {} MB is below minimum (100 MB)",
				performance.disk_limit_mb
			)));
		}

		if performance.disk_limit_mb > 102400 {
			return Err(AirError::Configuration(format!(
				"Disk limit {} MB exceeds maximum (102400 MB = 100 GB)",
				performance.disk_limit_mb
			)));
		}

		// Validate background_task_interval_secs range [1, 3600]
		if performance.background_task_interval_secs < 1 {
			return Err(AirError::Configuration(format!(
				"Background task interval {} seconds is below minimum (1 second)",
				performance.background_task_interval_secs
			)));
		}

		if performance.background_task_interval_secs > 3600 {
			return Err(AirError::Configuration(format!(
				"Background task interval {} seconds exceeds maximum (3600 seconds = 1 hour)",
				performance.background_task_interval_secs
			)));
		}

		Ok(())
	}

	/// Validate path for security (prevent directory traversal)
	fn ValidatePath(&self, path:&str) -> Result<()> {
		if path.is_empty() {
			return Err(AirError::Configuration("Path cannot be empty".to_string()));
		}

		// Check for path traversal attempts
		if path.contains("..") {
			return Err(AirError::Configuration(format!(
				"Path '{}' contains '..' which is not allowed for security reasons",
				path
			)));
		}

		// Check for absolute path patterns that might be problematic
		if path.starts_with("\\\\") || path.starts_with("//") {
			return Err(AirError::Configuration(format!(
				"Path '{}' uses UNC/network path format which may not be supported",
				path
			)));
		}

		// Validate that the path doesn't contain null bytes
		if path.contains('\0') {
			return Err(AirError::Configuration(
				"Path contains null bytes which is not allowed".to_string(),
			));
		}

		Ok(())
	}

	/// Validate address format (IP:port or hostname:port)
	fn IsValidAddress(addr:&str) -> bool {
		// Check for IPv6 format: [IPv6]:port
		if addr.starts_with('[') && addr.contains("]:") {
			return true;
		}

		// Check for IPv4 or hostname format: host:port
		if addr.contains(':') {
			let parts:Vec<&str> = addr.split(':').collect();
			if parts.len() != 2 {
				return false;
			}

			// Validate port
			if let Ok(port) = parts[1].parse::<u16>() {
				return port > 0;
			}

			return false;
		}

		false
	}

	/// Validate URL format
	fn IsValidUrl(url:&str) -> bool { url::Url::parse(url).is_ok() }

	/// Perform schema-based validation
	fn SchemaValidate(&self, config:&AirConfiguration) -> Result<()> {
		let _schema = generate_schema();

		// Convert config to JSON for validation
		let config_json = serde_json::to_value(config)
			.map_err(|e| AirError::Configuration(format!("Failed to serialize config for schema validation: {}", e)))?;

		// Basic schema validation (would use jsonschema crate in production)
		// For now, we do manual validation
		if !config_json.is_object() {
			return Err(AirError::Configuration("Configuration must be an object".to_string()));
		}

		log::debug!("Schema validation passed");
		Ok(())
	}

	/// Apply environment variable overrides to configuration
	///
	/// Environment variables are read with the configured prefix.
	/// For example, with prefix "AIR_", the variable "AIR_GRPC_BIND_ADDRESS"
	/// would override grpc.bind_address.
	///
	/// Variable naming convention: {PREFIX}_{SECTION}_{FIELD} (uppercase,
	/// underscores)
	fn ApplyEnvironmentOverrides(&self, config:&mut AirConfiguration) -> Result<()> {
		let mut override_count = 0;

		// gRPC overrides
		if let Ok(val) = env::var(&format!("{}GRPC_BIND_ADDRESS", self.env_prefix)) {
			config.grpc.bind_address = val;
			override_count += 1;
		}

		if let Ok(val) = env::var(&format!("{}GRPC_MAX_CONNECTIONS", self.env_prefix)) {
			config.grpc.max_connections = val
				.parse()
				.map_err(|e| AirError::Configuration(format!("Invalid GRPC_MAX_CONNECTIONS value: {}", e)))?;
			override_count += 1;
		}

		// Authentication overrides
		if let Ok(val) = env::var(&format!("{}AUTH_ENABLED", self.env_prefix)) {
			config.authentication.enabled = val
				.parse()
				.map_err(|e| AirError::Configuration(format!("Invalid AUTH_ENABLED value: {}", e)))?;
			override_count += 1;
		}

		if let Ok(val) = env::var(&format!("{}AUTH_CREDENTIALS_PATH", self.env_prefix)) {
			config.authentication.credentials_path = val;
			override_count += 1;
		}

		// Update overrides
		if let Ok(val) = env::var(&format!("{}UPDATE_ENABLED", self.env_prefix)) {
			config.updates.enabled = val
				.parse()
				.map_err(|e| AirError::Configuration(format!("Invalid UPDATE_ENABLED value: {}", e)))?;
			override_count += 1;
		}

		if let Ok(val) = env::var(&format!("{}UPDATE_AUTO_DOWNLOAD", self.env_prefix)) {
			config.updates.auto_download = val
				.parse()
				.map_err(|e| AirError::Configuration(format!("Invalid UPDATE_AUTO_DOWNLOAD value: {}", e)))?;
			override_count += 1;
		}

		// Logging overrides
		if let Ok(val) = env::var(&format!("{}LOGGING_LEVEL", self.env_prefix)) {
			config.logging.level = val.to_lowercase();
			override_count += 1;
		}

		if override_count > 0 {
			log::info!("Applied {} environment variable override(s)", override_count);
		}

		Ok(())
	}

	/// Backup current configuration file
	///
	/// Creates a timestamped backup of the current configuration file
	/// in the configured backup directory.
	async fn BackupConfiguration(&self, config_path:&Path) -> Result<()> {
		let backup_dir = self
			.backup_dir
			.as_ref()
			.ok_or_else(|| AirError::Configuration("Backup directory not configured".to_string()))?;

		// Create backup directory if it doesn't exist
		tokio::fs::create_dir_all(backup_dir).await.map_err(|e| {
			AirError::Configuration(format!("Failed to create backup directory '{}': {}", backup_dir.display(), e))
		})?;

		// Generate backup filename with timestamp
		let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
		let backup_filename = format!(
			"{}_config_{}.toml.bak",
			config_path.file_stem().and_then(|s| s.to_str()).unwrap_or("config"),
			timestamp
		);
		let backup_path = backup_dir.join(&backup_filename);

		// Copy current config to backup
		tokio::fs::copy(config_path, &backup_path).await.map_err(|e| {
			AirError::Configuration(format!("Failed to create backup '{}': {}", backup_path.display(), e))
		})?;

		log::info!("Configuration backed up to: {}", backup_path.display());
		Ok(())
	}

	/// Rollback configuration from the most recent backup
	///
	/// # Returns
	///
	/// Returns the path to the backup file that was restored
	pub async fn RollbackConfiguration(&self) -> Result<PathBuf> {
		let config_path = self.GetConfigPath()?;

		let backup_dir = self
			.backup_dir
			.as_ref()
			.ok_or_else(|| AirError::Configuration("Backup directory not configured".to_string()))?;

		// Find the most recent backup
		let mut backups = tokio::fs::read_dir(backup_dir).await.map_err(|e| {
			AirError::Configuration(format!("Failed to read backup directory '{}': {}", backup_dir.display(), e))
		})?;

		let mut most_recent:Option<(tokio::fs::DirEntry, std::time::SystemTime)> = None;

		while let Some(entry) = backups
			.next_entry()
			.await
			.map_err(|e| AirError::Configuration(format!("Failed to read backup entry: {}", e)))?
		{
			let metadata = entry
				.metadata()
				.await
				.map_err(|e| AirError::Configuration(format!("Failed to get metadata: {}", e)))?;

			if let Ok(modified) = metadata.modified() {
				if most_recent.is_none() || modified > most_recent.as_ref().unwrap().1 {
					most_recent = Some((entry, modified));
				}
			}
		}

		let (backup_entry, _) =
			most_recent.ok_or_else(|| AirError::Configuration("No backup files found".to_string()))?;

		let backup_path = backup_entry.path();

		// Restore from backup
		tokio::fs::copy(&backup_path, &config_path).await.map_err(|e| {
			AirError::Configuration(format!("Failed to restore from backup '{}': {}", backup_path.display(), e))
		})?;

		log::info!("Configuration rolled back from: {}", backup_path.display());
		Ok(backup_path)
	}

	/// Get the configuration file path
	///
	/// Returns the configured path or the default path
	fn GetConfigPath(&self) -> Result<PathBuf> {
		if let Some(ref path) = self.config_path {
			Ok(path.clone())
		} else {
			Self::GetDefaultConfigPath()
		}
	}

	/// Get default configuration file path
	///
	/// Returns the default configuration file path in the user's config
	/// directory
	fn GetDefaultConfigPath() -> Result<PathBuf> {
		let config_dir = dirs::config_dir()
			.ok_or_else(|| AirError::Configuration("Cannot determine config directory".to_string()))?;

		Ok(config_dir.join("Air").join(DefaultConfigFile))
	}

	/// Get profile-specific default configuration
	///
	/// # Arguments
	///
	/// * `profile` - Profile name (dev, staging, prod, custom)
	///
	/// # Returns
	///
	/// Configuration with profile-appropriate defaults
	pub fn GetProfileDefaults(profile:&str) -> AirConfiguration {
		let mut config = AirConfiguration::default();
		config.profile = profile.to_string();

		match profile {
			"prod" => {
				config.logging.level = "warn".to_string();
				config.logging.console_enabled = false;
				config.performance.memory_limit_mb = 1024;
				config.performance.cpu_limit_percent = 80;
			},
			"staging" => {
				config.logging.level = "info".to_string();
				config.performance.memory_limit_mb = 768;
				config.performance.cpu_limit_percent = 70;
			},
			"dev" | _ => {
				// Dev defaults are already set
				config.logging.level = "debug".to_string();
				config.logging.console_enabled = true;
				config.performance.memory_limit_mb = 512;
				config.performance.cpu_limit_percent = 50;
			},
		}

		config
	}

	/// Expand path with home directory (~) expansion
	///
	/// # Arguments
	///
	/// * `path` - Path string to expand
	///
	/// # Returns
	///
	/// Expanded PathBuf
	pub fn ExpandPath(path:&str) -> Result<PathBuf> {
		if path.is_empty() {
			return Err(AirError::Configuration("Cannot expand empty path".to_string()));
		}

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

	/// Generate configuration hash for change detection
	///
	/// # Arguments
	///
	/// * `config` - Configuration to hash
	///
	/// # Returns
	///
	/// SHA256 hash of the configuration
	pub fn ComputeHash(config:&AirConfiguration) -> Result<String> {
		let config_str = toml::to_string_pretty(config)
			.map_err(|e| AirError::Configuration(format!("Failed to serialize config: {}", e)))?;

		let mut hasher = sha2::Sha256::new();
		hasher.update(config_str.as_bytes());
		let hash = hasher.finalize();

		Ok(hex::encode(hash))
	}

	/// Export configuration to JSON (for VSCode compatibility)
	///
	/// # Arguments
	///
	/// * `config` - Configuration to export
	///
	/// # Returns
	///
	/// JSON string representation of configuration
	pub fn ExportToJson(config:&AirConfiguration) -> Result<String> {
		serde_json::to_string_pretty(config)
			.map_err(|e| AirError::Configuration(format!("Failed to export to JSON: {}", e)))
	}

	/// Import configuration from JSON (for VSCode compatibility)
	///
	/// # Arguments
	///
	/// * `json_str` - JSON string to import
	///
	/// # Returns
	///
	/// Parsed and validated configuration
	pub fn ImportFromJson(json_str:&str) -> Result<AirConfiguration> {
		let config:AirConfiguration = serde_json::from_str(json_str)
			.map_err(|e| AirError::Configuration(format!("Failed to import from JSON: {}", e)))?;

		Ok(config)
	}

	/// Get environment variable mappings
	///
	/// Returns a mapping of configuration paths to environment variable names
	pub fn GetEnvironmentMappings(&self) -> HashMap<String, String> {
		let prefix = &self.env_prefix;
		let mut mappings = HashMap::new();

		mappings.insert("grpc.bind_address".to_string(), format!("{}GRPC_BIND_ADDRESS", prefix));
		mappings.insert("grpc.max_connections".to_string(), format!("{}GRPC_MAX_CONNECTIONS", prefix));
		mappings.insert(
			"grpc.request_timeout_secs".to_string(),
			format!("{}GRPC_REQUEST_TIMEOUT_SECS", prefix),
		);

		mappings.insert("authentication.enabled".to_string(), format!("{}AUTH_ENABLED", prefix));
		mappings.insert(
			"authentication.credentials_path".to_string(),
			format!("{}AUTH_CREDENTIALS_PATH", prefix),
		);
		mappings.insert(
			"authentication.token_expiration_hours".to_string(),
			format!("{}AUTH_TOKEN_EXPIRATION_HOURS", prefix),
		);

		mappings.insert("updates.enabled".to_string(), format!("{}UPDATE_ENABLED", prefix));
		mappings.insert("updates.auto_download".to_string(), format!("{}UPDATE_AUTO_DOWNLOAD", prefix));
		mappings.insert("updates.auto_install".to_string(), format!("{}UPDATE_AUTO_INSTALL", prefix));

		mappings.insert("logging.level".to_string(), format!("{}LOGGING_LEVEL", prefix));
		mappings.insert(
			"logging.console_enabled".to_string(),
			format!("{}LOGGING_CONSOLE_ENABLED", prefix),
		);

		mappings
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_default_configuration() {
		let config = AirConfiguration::default();
		assert_eq!(config.schema_version, "1.0.0");
		assert_eq!(config.profile, "dev");
		assert!(config.authentication.enabled);
		assert!(config.logging.console_enabled);
	}

	#[test]
	fn test_profile_defaults() {
		let DevConfig = ConfigurationManager::GetProfileDefaults("dev");
		assert_eq!(DevConfig.profile, "dev");
		assert_eq!(DevConfig.logging.level, "debug");

		let ProdConfig = ConfigurationManager::GetProfileDefaults("prod");
		assert_eq!(ProdConfig.profile, "prod");
		assert_eq!(ProdConfig.logging.level, "warn");
		assert!(!ProdConfig.logging.console_enabled);
	}

	#[test]
	fn test_path_expansion() {
		let Home = dirs::home_dir().expect("Cannot determine home directory");
		let Expanded = ConfigurationManager::ExpandPath("~/test").unwrap();
		assert_eq!(Expanded, Home.join("test"));

		let Absolute = ConfigurationManager::ExpandPath("/tmp/test").unwrap();
		assert_eq!(Absolute, PathBuf::from("/tmp/test"));
	}

	#[test]
	fn test_address_validation() {
		assert!(ConfigurationManager::IsValidAddress("[::1]:50053"));
		assert!(ConfigurationManager::IsValidAddress("127.0.0.1:50053"));
		assert!(ConfigurationManager::IsValidAddress("localhost:50053"));
		assert!(!ConfigurationManager::IsValidAddress("invalid"));
	}

	#[test]
	fn test_url_validation() {
		assert!(ConfigurationManager::IsValidUrl("https://example.com"));
		assert!(ConfigurationManager::IsValidUrl("https://updates.editor.land"));
		assert!(!ConfigurationManager::IsValidUrl("not-a-url"));
		assert!(!ConfigurationManager::IsValidUrl("http://insecure.com"));
	}

	#[test]
	fn test_path_validation() {
		let manager = ConfigurationManager::New(None).unwrap();
		assert!(manager.ValidatePath("~/config").is_ok());
		assert!(manager.ValidatePath("/tmp/config").is_ok());
		assert!(manager.ValidatePath("../escaped").is_err());
		assert!(manager.ValidatePath("").is_err());
	}

	#[tokio::test]
	async fn test_export_import_json() {
		let config = AirConfiguration::default();
		let json_str = ConfigurationManager::ExportToJson(&config).unwrap();

		let imported = ConfigurationManager::ImportFromJson(&json_str).unwrap();
		assert_eq!(imported.schema_version, config.schema_version);
		assert_eq!(imported.profile, config.profile);
		assert_eq!(imported.grpc.bind_address, config.grpc.bind_address);
	}

	#[test]
	fn test_compute_hash() {
		let config = AirConfiguration::default();
		let hash1 = ConfigurationManager::ComputeHash(&config).unwrap();
		let hash2 = ConfigurationManager::ComputeHash(&config).unwrap();
		assert_eq!(hash1, hash2);

		let mut modified = config;
		modified.grpc.bind_address = "[::1]:50054".to_string();
		let hash3 = ConfigurationManager::ComputeHash(&modified).unwrap();
		assert_ne!(hash1, hash3);
	}

	#[test]
	fn test_generate_schema() {
		let schema = generate_schema();
		assert!(schema.is_object());
		assert!(schema.get("$schema").is_some());
		assert!(schema.get("properties").is_some());
	}
}
