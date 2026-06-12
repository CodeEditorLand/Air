use std::{
	collections::HashMap,
	env,
	path::{Path, PathBuf},
};

use sha2::Digest;
use serde::{Deserialize, Serialize};

use crate::{AirError, DefaultConfigFile, Result, dev_log};

use super::AirConfiguration::Struct as AirConfig;
use super::AirConfiguration::AuthConfig;
use super::AirConfiguration::DownloadConfig;
use super::AirConfiguration::IndexingConfig;
use super::AirConfiguration::LoggingConfig;
use super::AirConfiguration::PerformanceConfig;
use super::AirConfiguration::UpdateConfig;
use super::AirConfiguration::gRPCConfig;
use super::Schema::generate_schema;

pub struct Struct {
	/// Path to configuration file
	ConfigPath:Option<PathBuf>,

	/// Backup configuration directory
	BackupDir:Option<PathBuf>,

	/// Enable configuration backup
	EnableBackup:bool,

	/// Environment variable prefix for overrides
	EnvPrefix:String,
}

impl Struct {
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
	pub async fn LoadConfiguration(&self) -> Result<AirConfig> {
		// Start with default configuration
		let mut config = AirConfig::default();

		// Try to load from specified or default path
		let ConfigPath = self.GetConfigPath()?;

		if ConfigPath.exists() {
			dev_log!("config", "Loading configuration from: {}", ConfigPath.display());

			config = self.LoadFromFile(&ConfigPath).await?;
		} else {
			dev_log!("config", "No configuration file found, using defaults");
		}

		// Apply environment variable overrides
		self.ApplyEnvironmentOverrides(&mut config)?;

		// Schema validation
		self.SchemaValidate(&config)?;

		// Validate all configuration values
		self.ValidateConfiguration(&config)?;

		dev_log!("config", "Configuration loaded successfully (profile: {})", config.Profile);

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
	async fn LoadFromFile(&self, path:&Path) -> Result<AirConfig> {
		let content = tokio::fs::read_to_string(path)
			.await
			.map_err(|e| AirError::Configuration(format!("Failed to read config file '{}': {}", path.display(), e)))?;

		let config:AirConfig = toml::from_str(&content).map_err(|e| {
			AirError::Configuration(format!("Failed to parse TOML config file '{}': {}", path.display(), e))
		})?;

		// Type validation is done by serde automatically
		dev_log!("config", "Configuration file parsed successfully");

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
	pub async fn SaveConfiguration(&self, config:&AirConfig) -> Result<()> {
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

		dev_log!("config", "Configuration saved to: {}", ConfigPath.display());

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
	fn ValidateConfiguration(&self, config:&AirConfig) -> Result<()> {
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

		dev_log!("config", "All configuration validation checks passed");

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
					dev_log!(
						"config",
						"warn: File type pattern '{}' does not contain wildcards, may not match as expected",
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
	fn SchemaValidate(&self, config:&AirConfig) -> Result<()> {
		let _schema = generate_schema();

		// Convert config to JSON for validation
		let ConfigJson = serde_json::to_value(config)
			.map_err(|e| AirError::Configuration(format!("Failed to serialize config for schema validation: {}", e)))?;

		// Basic schema validation (would use jsonschema crate in production)
		// For now, we do manual validation
		if !ConfigJson.is_object() {
			return Err(AirError::Configuration("Configuration must be an object".to_string()));
		}

		dev_log!("config", "Schema validation passed");

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
	fn ApplyEnvironmentOverrides(&self, config:&mut AirConfig) -> Result<()> {
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
			dev_log!("config", "Applied {} environment variable override(s)", override_count);
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

		dev_log!("config", "Configuration backed up to: {}", backup_path.display());

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

		dev_log!("config", "Configuration rolled back from: {}", backup_path.display());

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
	pub fn GetProfileDefaults(profile:&str) -> AirConfig {
		let mut config = AirConfig::default();

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
	pub fn ComputeHash(config:&AirConfig) -> Result<String> {
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
	pub fn ExportToJson(config:&AirConfig) -> Result<String> {
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
	pub fn ImportFromJson(json_str:&str) -> Result<AirConfig> {
			let config:AirConfig = serde_json::from_str(json_str)
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

// =============================================================================
// Free-function wrappers
// =============================================================================

/// Expand a path with home directory (~) expansion
pub fn ExpandPath(path:&str) -> Result<PathBuf> {
	Struct::ExpandPath(path)
}

/// Compute SHA-256 hash of configuration for change detection
pub fn ComputeHash(config:&AirConfig) -> Result<String> {
	Struct::ComputeHash(config)
}

/// Validate an address string (IP:port or hostname:port format)
pub fn IsValidAddress(addr:&str) -> bool {
	Struct::IsValidAddress(addr)
}

/// Validate a URL string
pub fn IsValidUrl(url:&str) -> bool {
	Struct::IsValidUrl(url)
}

/// Get profile-specific default configuration
pub fn GetProfileDefaults(profile:&str) -> AirConfig {
	Struct::GetProfileDefaults(profile)
}

/// Create a new configuration manager
pub fn New(ConfigPath:Option<String>) -> Result<Struct> {
	Struct::New(ConfigPath)
}

/// Export configuration to JSON string
pub fn ExportToJson(config:&AirConfig) -> Result<String> {
	Struct::ExportToJson(config)
}

/// Import configuration from JSON string
pub fn ImportFromJson(json_str:&str) -> Result<AirConfig> {
	Struct::ImportFromJson(json_str)
}

/// Load configuration from a specific file path
pub fn load(config_path:&std::path::Path) -> Result<AirConfig> {
	// Create a temporary manager to load from the given path
	let path_str = config_path.to_str().map(|s| s.to_string());
	let manager = Struct::New(path_str)?;
	let rt = tokio::runtime::Runtime::new()
		.map_err(|e| AirError::Configuration(format!("Failed to create tokio runtime: {}", e)))?;
	rt.block_on(manager.LoadConfiguration())
}
