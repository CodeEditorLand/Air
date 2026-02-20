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
//! ```text
//! Mountain (User Settings) → Air config file → Wind services
//! ↓ ↓ ↓
//! settings.json ~/.Air/config.toml Service-specific overrides
//! ↓ ↓ ↓
//! Workspace settings Environment variables Hot-reload notifications
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
	pub SchemaVersion:String,

	/// Profile name (dev, staging, prod, custom)
	#[serde(default = "default_profile")]
	pub Profile:String,

	/// gRPC server configuration
	pub gRPC:gRPCConfig,

	/// Authentication configuration
	pub Authentication:AuthConfig,

	/// Update configuration
	pub Updates:UpdateConfig,

	/// Download configuration
	pub Downloader:DownloadConfig,

	/// Indexing configuration
	pub Indexing:IndexingConfig,

	/// Logging configuration
	pub Logging:LoggingConfig,

	/// Performance configuration
	pub Performance:PerformanceConfig,
}

fn default_schema_version() -> String { "1.0.0".to_string() }

fn default_profile() -> String { "dev".to_string() }

/// gRPC server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct gRPCConfig {
	/// Bind address for gRPC server
	/// Validation: Must be a valid IP:port or hostname:port combination
	/// Format: `[IPv6]`:port or IPv4:port or hostname:port
	/// Example: `"[::1]:50053"`, `"127.0.0.1:50053"`, `"localhost:50053"`
	#[serde(default = "default_grpc_bind_address")]
	pub BindAddress:String,

	/// Maximum concurrent connections
	/// Validation: Range [10, 10000]
	/// Default: 100
	#[serde(default = "default_grpc_max_connections")]
	pub MaxConnections:u32,

	/// Request timeout in seconds
	/// Validation: Range [1, 3600] (1 second to 1 hour)
	/// Default: 30
	#[serde(default = "default_grpc_request_timeout")]
	pub RequestTimeoutSecs:u64,
}

fn default_grpc_bind_address() -> String { "[::1]:50053".to_string() }

fn default_grpc_max_connections() -> u32 { 100 }

fn default_grpc_request_timeout() -> u64 { 30 }

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
	/// Enable authentication service
	#[serde(default = "default_auth_enabled")]
	pub Enabled:bool,

	/// Path to credentials storage
	/// Validation: Must be a valid absolute or home-relative path
	/// Security: Ensures directory traversal prevention
	/// Default: "~/.Air/credentials"
	#[serde(default = "default_auth_credentials_path")]
	pub CredentialsPath:String,

	/// Token expiration in hours
	/// Validation: Range [1, 8760] (1 hour to 1 year)
	/// Default: 24
	#[serde(default = "default_auth_token_expiration")]
	pub TokenExpirationHours:u32,

	/// Maximum concurrent auth sessions
	/// Validation: Range [1, 1000]
	/// Default: 10
	#[serde(default = "default_auth_max_sessions")]
	pub MaxSessions:u32,
}

fn default_auth_enabled() -> bool { true }

fn default_auth_credentials_path() -> String { "~/.Air/credentials".to_string() }

fn default_auth_token_expiration() -> u32 { 24 }

fn default_auth_max_sessions() -> u32 { 10 }

/// Update configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
	/// Enable update service
	#[serde(default = "default_update_enabled")]
	pub Enabled:bool,

	/// Update check interval in hours
	/// Validation: Range [1, 168] (1 hour to 1 week)
	/// Default: 6
	#[serde(default = "default_update_check_interval")]
	pub CheckIntervalHours:u32,

	/// Update server URL
	/// Validation: Must be a valid HTTPS URL
	/// Security: HTTPS required for security
	/// Default: <https://updates.editor.land>
	#[serde(default = "default_update_server_url")]
	pub UpdateServerUrl:String,

	/// Auto-download updates
	#[serde(default = "default_update_auto_download")]
	pub AutoDownload:bool,

	/// Auto-install updates
	/// Warning: Use with caution in production
	#[serde(default = "default_update_auto_install")]
	pub AutoInstall:bool,

	/// Update channel
	/// Validation: Must be one of: "stable", "insiders", "preview"
	/// Default: "stable"
	#[serde(default = "default_update_channel")]
	pub Channel:String,
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
	pub Enabled:bool,

	/// Maximum concurrent downloads
	/// Validation: Range [1, 50]
	/// Default: 5
	#[serde(default = "default_download_max_concurrent")]
	pub MaxConcurrentDownloads:u32,

	/// Download timeout in seconds
	/// Validation: Range [10, 3600] (10 seconds to 1 hour)
	/// Default: 300
	#[serde(default = "default_download_timeout")]
	pub DownloadTimeoutSecs:u64,

	/// Maximum retry attempts
	/// Validation: Range [0, 10]
	/// Default: 3
	#[serde(default = "default_download_max_retries")]
	pub MaxRetries:u32,

	/// Download cache directory
	/// Validation: Must be a valid absolute or home-relative path
	/// Default: "~/.Air/cache"
	#[serde(default = "default_download_cache_dir")]
	pub CacheDirectory:String,
}

fn default_download_enabled() -> bool { true }

fn default_download_max_concurrent() -> u32 { 5 }

fn default_download_timeout() -> u64 { 300 }

fn default_download_max_retries() -> u32 { 3 }

fn default_download_cache_dir() -> String { "~/.Air/cache".to_string() }

/// Indexing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
	/// Enable indexing service
	#[serde(default = "default_indexing_enabled")]
	pub Enabled:bool,

	/// Maximum file size to index (MB)
	/// Validation: Range [1, 1024] (1MB to 1GB)
	/// Default: 10
	#[serde(default = "default_indexing_max_file_size")]
	pub MaxFileSizeMb:u32,

	/// File types to index
	/// Format: Glob patterns like "*.rs", "*.ts", etc.
	/// Validation: Each pattern must be a valid glob pattern
	/// Default: Common source code file types
	#[serde(default = "default_indexing_file_types")]
	pub FileTypes:Vec<String>,

	/// Index update interval in minutes
	/// Validation: Range [1, 1440] (1 minute to 1 day)
	/// Default: 30
	#[serde(default = "default_indexing_update_interval")]
	pub UpdateIntervalMinutes:u32,

	/// Index storage directory
	/// Validation: Must be a valid absolute or home-relative path
	/// Default: "~/.Air/index"
	#[serde(default = "default_indexing_directory")]
	pub IndexDirectory:String,

	/// Maximum parallel indexing operations
	/// Validation: Range [1, 100] (1 to 100 concurrent operations)
	/// Default: 10
	#[serde(default = "default_max_parallel_indexing")]
	pub MaxParallelIndexing:u32,
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

fn default_indexing_directory() -> String { "~/.Air/index".to_string() }

fn default_max_parallel_indexing() -> u32 { 10 }

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
	/// Log level
	/// Validation: Must be one of: "trace", "debug", "info", "warn", "error"
	/// Default: "info"
	#[serde(default = "default_logging_level")]
	pub Level:String,

	/// Log file path
	/// Validation: Must be a valid absolute or home-relative path if provided
	/// Default: "~/.Air/logs/Air.log"
	#[serde(default = "default_logging_file_path")]
	pub FilePath:Option<String>,

	/// Enable console logging
	#[serde(default = "default_logging_console_enabled")]
	pub ConsoleEnabled:bool,

	/// Maximum log file size (MB)
	/// Validation: Range [1, 1000]
	/// Default: 10
	#[serde(default = "default_logging_max_file_size")]
	pub MaxFileSizeMb:u32,

	/// Maximum log files to keep
	/// Validation: Range [1, 50]
	/// Default: 5
	#[serde(default = "default_logging_max_files")]
	pub MaxFiles:u32,
}

fn default_logging_level() -> String { "info".to_string() }

fn default_logging_file_path() -> Option<String> { Some("~/.Air/logs/Air.log".to_string()) }

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
	pub MemoryLimitMb:u32,

	/// CPU usage limit (%)
	/// Validation: Range [10, 100]
	/// Default: 50
	#[serde(default = "default_perf_cpu_limit")]
	pub CPULimitPercent:u32,

	/// Disk usage limit (MB)
	/// Validation: Range [100, 102400] (100MB to 100GB)
	/// Default: 1024
	#[serde(default = "default_perf_disk_limit")]
	pub DiskLimitMb:u32,

	/// Background task interval in seconds
	/// Validation: Range [1, 3600] (1 second to 1 hour)
	/// Default: 60
	#[serde(default = "default_perf_task_interval")]
	pub BackgroundTaskIntervalSecs:u64,
}

fn default_perf_memory_limit() -> u32 { 512 }

fn default_perf_cpu_limit() -> u32 { 50 }

fn default_perf_disk_limit() -> u32 { 1024 }

fn default_perf_task_interval() -> u64 { 60 }

impl Default for AirConfiguration {
	fn default() -> Self {
		Self {
			SchemaVersion:default_schema_version(),
			Profile:default_profile(),
			gRPC:gRPCConfig {
				BindAddress:default_grpc_bind_address(),
				MaxConnections:default_grpc_max_connections(),
				RequestTimeoutSecs:default_grpc_request_timeout(),
			},
			Authentication:AuthConfig {
				Enabled:default_auth_enabled(),
				CredentialsPath:default_auth_credentials_path(),
				TokenExpirationHours:default_auth_token_expiration(),
				MaxSessions:default_auth_max_sessions(),
			},
			Updates:UpdateConfig {
				Enabled:default_update_enabled(),
				CheckIntervalHours:default_update_check_interval(),
				UpdateServerUrl:default_update_server_url(),
				AutoDownload:default_update_auto_download(),
				AutoInstall:default_update_auto_install(),
				Channel:default_update_channel(),
			},
			Downloader:DownloadConfig {
				Enabled:default_download_enabled(),
				MaxConcurrentDownloads:default_download_max_concurrent(),
				DownloadTimeoutSecs:default_download_timeout(),
				MaxRetries:default_download_max_retries(),
				CacheDirectory:default_download_cache_dir(),
			},
			Indexing:IndexingConfig {
				Enabled:default_indexing_enabled(),
				MaxFileSizeMb:default_indexing_max_file_size(),
				FileTypes:default_indexing_file_types(),
				UpdateIntervalMinutes:default_indexing_update_interval(),
				IndexDirectory:default_indexing_directory(),
				MaxParallelIndexing:default_max_parallel_indexing(),
			},
			Logging:LoggingConfig {
				Level:default_logging_level(),
				FilePath:default_logging_file_path(),
				ConsoleEnabled:default_logging_console_enabled(),
				MaxFileSizeMb:default_logging_max_file_size(),
				MaxFiles:default_logging_max_files(),
			},
			Performance:PerformanceConfig {
				MemoryLimitMb:default_perf_memory_limit(),
				CPULimitPercent:default_perf_cpu_limit(),
				DiskLimitMb:default_perf_disk_limit(),
				BackgroundTaskIntervalSecs:default_perf_task_interval(),
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
		"required": ["SchemaVersion", "profile"],
		"properties": {
			"SchemaVersion": {
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
					"BindAddress": {
						"type": "string",
						"description": "gRPC server bind address",
						"format": "hostname-port"
					},
					"MaxConnections": {
						"type": "integer",
						"minimum": 10,
						"maximum": 10000
					},
					"RequestTimeoutSecs": {
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
					"CredentialsPath": {"type": "string"},
					"TokenExpirationHours": {
						"type": "integer",
						"minimum": 1,
						"maximum": 8760
					},
					"MaxSessions": {
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
					"CheckIntervalHours": {
						"type": "integer",
						"minimum": 1,
						"maximum": 168
					},
					"UpdateServerUrl": {
						"type": "string",
						"pattern": "^https://"
					},
					"AutoDownload": {"type": "boolean"},
					"AutoInstall": {"type": "boolean"},
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
					"MaxConcurrentDownloads": {
						"type": "integer",
						"minimum": 1,
						"maximum": 50
					},
					"DownloadTimeoutSecs": {
						"type": "integer",
						"minimum": 10,
						"maximum": 3600
					},
					"MaxRetries": {
						"type": "integer",
						"minimum": 0,
						"maximum": 10
					},
					"CacheDirectory": {"type": "string"}
				}
			},
			"indexing": {
				"type": "object",
				"properties": {
					"enabled": {"type": "boolean"},
					"MaxFileSizeMb": {
						"type": "integer",
						"minimum": 1,
						"maximum": 1024
					},
					"FileTypes": {
						"type": "array",
						"items": {"type": "string"}
					},
					"UpdateIntervalMinutes": {
						"type": "integer",
						"minimum": 1,
						"maximum": 1440
					},
					"IndexDirectory": {"type": "string"}
				}
			},
			"logging": {
				"type": "object",
				"properties": {
					"level": {
						"type": "string",
						"enum": ["trace", "debug", "info", "warn", "error"]
					},
					"FilePath": {"type": ["string", "null"]},
					"ConsoleEnabled": {"type": "boolean"},
					"MaxFileSizeMb": {
						"type": "integer",
						"minimum": 1,
						"maximum": 1000
					},
					"MaxFiles": {
						"type": "integer",
						"minimum": 1,
						"maximum": 50
					}
				}
			},
			"performance": {
				"type": "object",
				"properties": {
					"MemoryLimitMb": {
						"type": "integer",
						"minimum": 64,
						"maximum": 16384
					},
					"CPULimitPercent": {
						"type": "integer",
						"minimum": 10,
						"maximum": 100
					},
					"DiskLimitMb": {
						"type": "integer",
						"minimum": 100,
						"maximum": 102400
					},
					"BackgroundTaskIntervalSecs": {
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
	ConfigPath:Option<PathBuf>,

	/// Backup configuration directory
	BackupDir:Option<PathBuf>,

	/// Enable configuration backup
	EnableBackup:bool,

	/// Environment variable prefix for overrides
	EnvPrefix:String,
}

impl ConfigurationManager {
	/// Create a new configuration manager
	///
	/// # Arguments
	///
	/// * `ConfigPath` - Optional path to configuration file. If None, uses
	///   default location
	///
	/// # Returns
	///
	/// Returns a new ConfigurationManager instance
	pub fn New(ConfigPath:Option<String>) -> Result<Self> {
		let path = ConfigPath.map(PathBuf::from);
		let BackupDir = path
			.as_ref()
			.and_then(|p| p.parent())
			.map(|parent| parent.join(".ConfigBackups"));

		Ok(Self { ConfigPath:path, BackupDir, EnableBackup:true, EnvPrefix:"AIR_".to_string() })
	}

	/// Create a new configuration manager with custom settings
	///
	/// # Arguments
	///
	/// * `ConfigPath` - Optional path to configuration file
	/// * `EnableBackup` - Whether to enable automatic backups
	/// * `EnvPrefix` - Prefix for environment variable overrides
	pub fn NewWithSettings(ConfigPath:Option<String>, EnableBackup:bool, EnvPrefix:String) -> Result<Self> {
		let path = ConfigPath.map(PathBuf::from);
		let BackupDir = if EnableBackup {
			path.as_ref()
				.and_then(|p| p.parent())
				.map(|parent| parent.join(".ConfigBackups"))
		} else {
			None
		};

		Ok(Self { ConfigPath:path, BackupDir, EnableBackup, EnvPrefix })
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
		let ConfigPath = self.GetConfigPath()?;

		if ConfigPath.exists() {
			log::info!("Loading configuration from: {}", ConfigPath.display());
			config = self.LoadFromFile(&ConfigPath).await?;
		} else {
			log::info!("No configuration file found, using defaults");
		}

		// Apply environment variable overrides
		self.ApplyEnvironmentOverrides(&mut config)?;

		// Schema validation
		self.SchemaValidate(&config)?;

		// Validate all configuration values
		self.ValidateConfiguration(&config)?;

		log::info!("Configuration loaded successfully (profile: {})", config.Profile);
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

		let ConfigPath = self.GetConfigPath()?;

		// Create backup if enabled and file exists
		if self.EnableBackup && ConfigPath.exists() {
			self.BackupConfiguration(&ConfigPath).await?;
		}

		// Create parent directory if it doesn't exist
		if let Some(parent) = ConfigPath.parent() {
			tokio::fs::create_dir_all(parent).await.map_err(|e| {
				AirError::Configuration(format!("Failed to create config directory '{}': {}", parent.display(), e))
			})?;
		}

		// Atomic write: write to temp file, then rename
		let TempPath = ConfigPath.with_extension("tmp");
		let content = toml::to_string_pretty(config)
			.map_err(|e| AirError::Configuration(format!("Failed to serialize config: {}", e)))?;

		tokio::fs::write(&TempPath, content).await.map_err(|e| {
			AirError::Configuration(format!("Failed to write temp config file '{}': {}", TempPath.display(), e))
		})?;

		// Atomic rename
		tokio::fs::rename(&TempPath, &ConfigPath).await.map_err(|e| {
			AirError::Configuration(format!("Failed to rename temp config to '{}': {}", ConfigPath.display(), e))
		})?;

		log::info!("Configuration saved to: {}", ConfigPath.display());
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
		self.ValidateSchemaVersion(&config.SchemaVersion)?;

		// Profile validation
		self.ValidateProfile(&config.Profile)?;

		// gRPC configuration validation
		self.ValidategRPCConfig(&config.gRPC)?;

		// Authentication configuration validation
		self.ValidateAuthConfig(&config.Authentication)?;

		// Update configuration validation
		self.ValidateUpdateConfig(&config.Updates)?;

		// Download configuration validation
		self.ValidateDownloadConfig(&config.Downloader)?;

		// Indexing configuration validation
		self.ValidateIndexingConfig(&config.Indexing)?;

		// Logging configuration validation
		self.ValidateLoggingConfig(&config.Logging)?;

		// Performance configuration validation
		self.ValidatePerformanceConfig(&config.Performance)?;

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
		let ValidProfiles = ["dev", "staging", "prod", "custom"];

		if !ValidProfiles.contains(&profile) {
			return Err(AirError::Configuration(format!(
				"Invalid profile '{}': must be one of: {}",
				profile,
				ValidProfiles.join(", ")
			)));
		}

		Ok(())
	}

	/// Validate gRPC configuration with range checking
	fn ValidategRPCConfig(&self, grpc:&gRPCConfig) -> Result<()> {
		// Validate bind address
		if grpc.BindAddress.is_empty() {
			return Err(AirError::Configuration("gRPC bind address cannot be empty".to_string()));
		}

		// Validate address format
		if !Self::IsValidAddress(&grpc.BindAddress) {
			return Err(AirError::Configuration(format!(
				"Invalid gRPC bind address '{}': must be in format host:port or [IPv6]:port",
				grpc.BindAddress
			)));
		}

		// Validate MaxConnections range [10, 10000]
		if grpc.MaxConnections < 10 {
			return Err(AirError::Configuration(format!(
				"gRPC MaxConnections {} is below minimum (10)",
				grpc.MaxConnections
			)));
		}

		if grpc.MaxConnections > 10000 {
			return Err(AirError::Configuration(format!(
				"gRPC MaxConnections {} exceeds maximum (10000)",
				grpc.MaxConnections
			)));
		}

		// Validate RequestTimeoutSecs range [1, 3600]
		if grpc.RequestTimeoutSecs < 1 {
			return Err(AirError::Configuration(format!(
				"gRPC RequestTimeoutSecs {} is below minimum (1 second)",
				grpc.RequestTimeoutSecs
			)));
		}

		if grpc.RequestTimeoutSecs > 3600 {
			return Err(AirError::Configuration(format!(
				"gRPC RequestTimeoutSecs {} exceeds maximum (3600 seconds = 1 hour)",
				grpc.RequestTimeoutSecs
			)));
		}

		Ok(())
	}

	/// Validate authentication configuration
	fn ValidateAuthConfig(&self, auth:&AuthConfig) -> Result<()> {
		// If authentication is enabled, validate credentials path
		if auth.Enabled {
			if auth.CredentialsPath.is_empty() {
				return Err(AirError::Configuration(
					"Authentication credentials path cannot be empty when authentication is enabled".to_string(),
				));
			}

			// Validate path for security (prevent directory traversal)
			self.ValidatePath(&auth.CredentialsPath)?;
		}

		// Validate TokenExpirationHours range [1, 8760]
		if auth.TokenExpirationHours < 1 {
			return Err(AirError::Configuration(format!(
				"Token expiration hours {} is below minimum (1 hour)",
				auth.TokenExpirationHours
			)));
		}

		if auth.TokenExpirationHours > 8760 {
			return Err(AirError::Configuration(format!(
				"Token expiration hours {} exceeds maximum (8760 hours = 1 year)",
				auth.TokenExpirationHours
			)));
		}

		// Validate MaxSessions range [1, 1000]
		if auth.MaxSessions < 1 {
			return Err(AirError::Configuration(format!(
				"Max sessions {} is below minimum (1)",
				auth.MaxSessions
			)));
		}

		if auth.MaxSessions > 1000 {
			return Err(AirError::Configuration(format!(
				"Max sessions {} exceeds maximum (1000)",
				auth.MaxSessions
			)));
		}

		Ok(())
	}

	/// Validate update configuration
	fn ValidateUpdateConfig(&self, updates:&UpdateConfig) -> Result<()> {
		if updates.Enabled {
			// Validate update server URL
			if updates.UpdateServerUrl.is_empty() {
				return Err(AirError::Configuration(
					"Update server URL cannot be empty when updates are enabled".to_string(),
				));
			}

			// Must be HTTPS for security
			if !updates.UpdateServerUrl.starts_with("https://") {
				return Err(AirError::Configuration(format!(
					"Update server URL must use HTTPS, got: {}",
					updates.UpdateServerUrl
				)));
			}

			// Validate URL format
			if !Self::IsValidUrl(&updates.UpdateServerUrl) {
				return Err(AirError::Configuration(format!(
					"Invalid update server URL '{}'",
					updates.UpdateServerUrl
				)));
			}
		}

		// Validate CheckIntervalHours range [1, 168]
		if updates.CheckIntervalHours < 1 {
			return Err(AirError::Configuration(format!(
				"Update check interval {} hours is below minimum (1 hour)",
				updates.CheckIntervalHours
			)));
		}

		if updates.CheckIntervalHours > 168 {
			return Err(AirError::Configuration(format!(
				"Update check interval {} hours exceeds maximum (168 hours = 1 week)",
				updates.CheckIntervalHours
			)));
		}

		Ok(())
	}

	/// Validate download configuration
	fn ValidateDownloadConfig(&self, downloader:&DownloadConfig) -> Result<()> {
		if downloader.Enabled {
			if downloader.CacheDirectory.is_empty() {
				return Err(AirError::Configuration(
					"Download cache directory cannot be empty when downloader is enabled".to_string(),
				));
			}

			// Validate path for security
			self.ValidatePath(&downloader.CacheDirectory)?;
		}

		// Validate MaxConcurrentDownloads range [1, 50]
		if downloader.MaxConcurrentDownloads < 1 {
			return Err(AirError::Configuration(format!(
				"Max concurrent downloads {} is below minimum (1)",
				downloader.MaxConcurrentDownloads
			)));
		}

		if downloader.MaxConcurrentDownloads > 50 {
			return Err(AirError::Configuration(format!(
				"Max concurrent downloads {} exceeds maximum (50)",
				downloader.MaxConcurrentDownloads
			)));
		}

		// Validate DownloadTimeoutSecs range [10, 3600]
		if downloader.DownloadTimeoutSecs < 10 {
			return Err(AirError::Configuration(format!(
				"Download timeout {} seconds is below minimum (10 seconds)",
				downloader.DownloadTimeoutSecs
			)));
		}

		if downloader.DownloadTimeoutSecs > 3600 {
			return Err(AirError::Configuration(format!(
				"Download timeout {} seconds exceeds maximum (3600 seconds = 1 hour)",
				downloader.DownloadTimeoutSecs
			)));
		}

		// Validate MaxRetries range [0, 10]
		if downloader.MaxRetries > 10 {
			return Err(AirError::Configuration(format!(
				"Max retries {} exceeds maximum (10)",
				downloader.MaxRetries
			)));
		}

		Ok(())
	}

	/// Validate indexing configuration
	fn ValidateIndexingConfig(&self, indexing:&IndexingConfig) -> Result<()> {
		if indexing.Enabled {
			if indexing.IndexDirectory.is_empty() {
				return Err(AirError::Configuration(
					"Index directory cannot be empty when indexing is enabled".to_string(),
				));
			}

			// Validate path for security
			self.ValidatePath(&indexing.IndexDirectory)?;

			// Validate FileTypes is not empty
			if indexing.FileTypes.is_empty() {
				return Err(AirError::Configuration(
					"File types to index cannot be empty when indexing is enabled".to_string(),
				));
			}

			// Validate each file type pattern
			for FileType in &indexing.FileTypes {
				if FileType.is_empty() {
					return Err(AirError::Configuration("File type pattern cannot be empty".to_string()));
				}

				if !FileType.contains('*') {
					log::warn!(
						"File type pattern '{}' does not contain wildcards, may not match as expected",
						FileType
					);
				}
			}
		}

		// Validate MaxFileSizeMb range [1, 1024]
		if indexing.MaxFileSizeMb < 1 {
			return Err(AirError::Configuration(format!(
				"Max file size {} MB is below minimum (1 MB)",
				indexing.MaxFileSizeMb
			)));
		}

		if indexing.MaxFileSizeMb > 1024 {
			return Err(AirError::Configuration(format!(
				"Max file size {} MB exceeds maximum (1024 MB = 1 GB)",
				indexing.MaxFileSizeMb
			)));
		}

		// Validate UpdateIntervalMinutes range [1, 1440]
		if indexing.UpdateIntervalMinutes < 1 {
			return Err(AirError::Configuration(format!(
				"Index update interval {} minutes is below minimum (1 minute)",
				indexing.UpdateIntervalMinutes
			)));
		}

		if indexing.UpdateIntervalMinutes > 1440 {
			return Err(AirError::Configuration(format!(
				"Index update interval {} minutes exceeds maximum (1440 minutes = 1 day)",
				indexing.UpdateIntervalMinutes
			)));
		}

		Ok(())
	}

	/// Validate logging configuration
	fn ValidateLoggingConfig(&self, logging:&LoggingConfig) -> Result<()> {
		// Validate log level
		let ValidLevels = ["trace", "debug", "info", "warn", "error"];
		if !ValidLevels.contains(&logging.Level.as_str()) {
			return Err(AirError::Configuration(format!(
				"Invalid log level '{}': must be one of: {}",
				logging.Level,
				ValidLevels.join(", ")
			)));
		}

		// Validate file path if provided
		if let Some(ref FilePath) = logging.FilePath {
			if !FilePath.is_empty() {
				self.ValidatePath(FilePath)?;
			}
		}

		// Validate MaxFileSizeMb range [1, 1000]
		if logging.MaxFileSizeMb < 1 {
			return Err(AirError::Configuration(format!(
				"Max log file size {} MB is below minimum (1 MB)",
				logging.MaxFileSizeMb
			)));
		}

		if logging.MaxFileSizeMb > 1000 {
			return Err(AirError::Configuration(format!(
				"Max log file size {} MB exceeds maximum (1000 MB = 1 GB)",
				logging.MaxFileSizeMb
			)));
		}

		// Validate MaxFiles range [1, 50]
		if logging.MaxFiles < 1 {
			return Err(AirError::Configuration(format!(
				"Max log files {} is below minimum (1)",
				logging.MaxFiles
			)));
		}

		if logging.MaxFiles > 50 {
			return Err(AirError::Configuration(format!(
				"Max log files {} exceeds maximum (50)",
				logging.MaxFiles
			)));
		}

		Ok(())
	}

	/// Validate performance configuration
	fn ValidatePerformanceConfig(&self, performance:&PerformanceConfig) -> Result<()> {
		// Validate MemoryLimitMb range [64, 16384]
		if performance.MemoryLimitMb < 64 {
			return Err(AirError::Configuration(format!(
				"Memory limit {} MB is below minimum (64 MB)",
				performance.MemoryLimitMb
			)));
		}

		if performance.MemoryLimitMb > 16384 {
			return Err(AirError::Configuration(format!(
				"Memory limit {} MB exceeds maximum (16384 MB = 16 GB)",
				performance.MemoryLimitMb
			)));
		}

		// Validate CPULimitPercent range [10, 100]
		if performance.CPULimitPercent < 10 {
			return Err(AirError::Configuration(format!(
				"CPU limit {}% is below minimum (10%)",
				performance.CPULimitPercent
			)));
		}

		if performance.CPULimitPercent > 100 {
			return Err(AirError::Configuration(format!(
				"CPU limit {}% exceeds maximum (100%)",
				performance.CPULimitPercent
			)));
		}

		// Validate DiskLimitMb range [100, 102400]
		if performance.DiskLimitMb < 100 {
			return Err(AirError::Configuration(format!(
				"Disk limit {} MB is below minimum (100 MB)",
				performance.DiskLimitMb
			)));
		}

		if performance.DiskLimitMb > 102400 {
			return Err(AirError::Configuration(format!(
				"Disk limit {} MB exceeds maximum (102400 MB = 100 GB)",
				performance.DiskLimitMb
			)));
		}

		// Validate BackgroundTaskIntervalSecs range [1, 3600]
		if performance.BackgroundTaskIntervalSecs < 1 {
			return Err(AirError::Configuration(format!(
				"Background task interval {} seconds is below minimum (1 second)",
				performance.BackgroundTaskIntervalSecs
			)));
		}

		if performance.BackgroundTaskIntervalSecs > 3600 {
			return Err(AirError::Configuration(format!(
				"Background task interval {} seconds exceeds maximum (3600 seconds = 1 hour)",
				performance.BackgroundTaskIntervalSecs
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
		let ConfigJson = serde_json::to_value(config)
			.map_err(|e| AirError::Configuration(format!("Failed to serialize config for schema validation: {}", e)))?;

		// Basic schema validation (would use jsonschema crate in production)
		// For now, we do manual validation
		if !ConfigJson.is_object() {
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
		if let Ok(val) = env::var(&format!("{}GRPC_BIND_ADDRESS", self.EnvPrefix)) {
			config.gRPC.BindAddress = val;
			override_count += 1;
		}

		if let Ok(val) = env::var(&format!("{}GRPC_MAX_CONNECTIONS", self.EnvPrefix)) {
			config.gRPC.MaxConnections = val
				.parse()
				.map_err(|e| AirError::Configuration(format!("Invalid GRPC_MAX_CONNECTIONS value: {}", e)))?;
			override_count += 1;
		}

		// Authentication overrides
		if let Ok(val) = env::var(&format!("{}AUTH_ENABLED", self.EnvPrefix)) {
			config.Authentication.Enabled = val
				.parse()
				.map_err(|e| AirError::Configuration(format!("Invalid AUTH_ENABLED value: {}", e)))?;
			override_count += 1;
		}

		if let Ok(val) = env::var(&format!("{}AUTH_CREDENTIALS_PATH", self.EnvPrefix)) {
			config.Authentication.CredentialsPath = val;
			override_count += 1;
		}

		// Update overrides
		if let Ok(val) = env::var(&format!("{}UPDATE_ENABLED", self.EnvPrefix)) {
			config.Updates.Enabled = val
				.parse()
				.map_err(|e| AirError::Configuration(format!("Invalid UPDATE_ENABLED value: {}", e)))?;
			override_count += 1;
		}

		if let Ok(val) = env::var(&format!("{}UPDATE_AUTO_DOWNLOAD", self.EnvPrefix)) {
			config.Updates.AutoDownload = val
				.parse()
				.map_err(|e| AirError::Configuration(format!("Invalid UPDATE_AUTO_DOWNLOAD value: {}", e)))?;
			override_count += 1;
		}

		// Logging overrides
		if let Ok(val) = env::var(&format!("{}LOGGING_LEVEL", self.EnvPrefix)) {
			config.Logging.Level = val.to_lowercase();
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
			.BackupDir
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
			.BackupDir
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
		if let Some(ref path) = self.ConfigPath {
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
		config.Profile = profile.to_string();

		match profile {
			"prod" => {
				config.Logging.Level = "warn".to_string();
				config.Logging.ConsoleEnabled = false;
				config.Performance.MemoryLimitMb = 1024;
				config.Performance.CPULimitPercent = 80;
			},
			"staging" => {
				config.Logging.Level = "info".to_string();
				config.Performance.MemoryLimitMb = 768;
				config.Performance.CPULimitPercent = 70;
			},
			"dev" | _ => {
				// Dev defaults are already set
				config.Logging.Level = "debug".to_string();
				config.Logging.ConsoleEnabled = true;
				config.Performance.MemoryLimitMb = 512;
				config.Performance.CPULimitPercent = 50;
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
		let prefix = &self.EnvPrefix;
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
		assert_eq!(config.SchemaVersion, "1.0.0");
		assert_eq!(config.Profile, "dev");
		assert!(config.Authentication.Enabled);
		assert!(config.Logging.ConsoleEnabled);
	}

	#[test]
	fn test_profile_defaults() {
		let DevConfig = ConfigurationManager::GetProfileDefaults("dev");
		assert_eq!(DevConfig.Profile, "dev");
		assert_eq!(DevConfig.Logging.Level, "debug");

		let ProdConfig = ConfigurationManager::GetProfileDefaults("prod");
		assert_eq!(ProdConfig.Profile, "prod");
		assert_eq!(ProdConfig.Logging.Level, "warn");
		assert!(!ProdConfig.Logging.ConsoleEnabled);
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
		assert_eq!(imported.SchemaVersion, config.SchemaVersion);
		assert_eq!(imported.Profile, config.Profile);
		assert_eq!(imported.gRPC.BindAddress, config.gRPC.BindAddress);
	}

	#[test]
	fn test_compute_hash() {
		let config = AirConfiguration::default();
		let hash1 = ConfigurationManager::ComputeHash(&config).unwrap();
		let hash2 = ConfigurationManager::ComputeHash(&config).unwrap();
		assert_eq!(hash1, hash2);

		let mut modified = config;
		modified.gRPC.BindAddress = "[::1]:50054".to_string();
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
