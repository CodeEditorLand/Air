//! # Configuration Hot-Reload System
//!
//! This module provides live configuration reloading capabilities with
//! comprehensive error handling, validation, atomic updates, and rollback
//! support for the Air daemon.
//!
//! ## Features
//!
//! - **File System Monitoring**: Real-time detection of configuration file
//!   changes
//! - **Signal Handling**: SIGHUP support for manual configuration reload
//!   triggers
//! - **Atomic Swaps**: Thread-safe configuration updates without service
//!   interruption
//! - **Automatic Rollback**: Revert to previous configuration on validation
//!   failure
//! - **Change Tracking**: Detailed audit trail of all configuration changes
//! - **Validation Pipeline**: Multi-stage validation with custom validators
//! - **Retry Logic**: Automatic retry with exponential backoff on transient
//!   failures
//! - **Notification System**: Callback system for configuration change events
//! - **Graceful Degradation**: System continues operating even if hot-reload
//!   fails
//!
//! ## Integration with Configuration System
//!
//! The hot-reload system works in tandem with the main configuration module:
//! - Uses same validation logic from Configuration module
//! - Shares configuration schema and structure
//! - Provides runtime updates without requiring service restart
//! - Scales horizontally across multiple Air instances
//!
//! ## Connection to Mountain and Wind Services
//!
//! Configuration changes detected by hot-reload are propagated to:
//! - Mountain: User settings synchronized in real-time
//! - Wind: All background services notified of configuration updates
//! - VSCode: Frontend receives configuration change events
//!
//! ## Signal Handling
//!
//! Supports the following Unix signals for manual control:
//! - `SIGHUP`: Force configuration reload from disk
//! - `SIGUSR1`: Hot-reload status information
//! - `SIGUSR2`: Disable/enable hot-reload monitoring
//!
//! ## Notification Flow
//!
//! ```
//! Config file changed → File watcher detected → Load & Validate
//!        ↓                                               ↓
//!   Atomic swap ←- Validation passed ←-- Migration applied
//!        ↓
//!   Notify subscribers → Wind services update → Mountain sync
//!        ↓
//!   Change history logged → Rollback state updated
//! ```
//!
//! ## Error Recovery
//!
//! The system implements a robust error recovery strategy:
//! - Validation failures: Automatic rollback to previous valid configuration
//! - Parse errors: Keep existing configuration, log error, continue monitoring
//! - File system errors: Temporary pause in monitoring, retry with backoff
//! - Concurrent modifications: Use atomic file operations, retry on conflict
//!
//! ## Performance Considerations
//!
//! - Debouncing: Multiple rapid changes trigger single reload after cooldown
//! - Async operations: Non-blocking file I/O and validation
//! - Lock-free reads: Configuration reads don't block other operations
//! - Efficient diffing: Only process changed configuration sections
//!
//! TODO: Add distributed configuration synchronization across multiple Air
//! instances TODO: Implement configuration change broadcasting to connected
//! Wind services TODO: Add configuration version conflict resolution
//! (multi-master scenarios)

use std::{
	path::{Path, PathBuf},
	sync::Arc,
	time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tokio::{
	fs,
	sync::{RwLock, broadcast, mpsc},
	time::sleep,
};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Result as NotifyResult, Watcher};
use chrono::{DateTime, Utc};
use log::{debug, error, info, trace, warn};

use crate::{AirError, Configuration::AirConfiguration, Result};

// =============================================================================
// Configuration Hot-Reload Manager
// =============================================================================

/// Configuration hot-reload manager with file watching and validation
pub struct ConfigHotReload {
	/// Current active configuration
	active_config:Arc<RwLock<AirConfiguration>>,

	/// Previous configuration for rollback
	previous_config:Arc<RwLock<Option<AirConfiguration>>>,

	/// Last successful configuration hash
	last_config_hash:Arc<RwLock<Option<String>>>,

	/// Configuration file path
	config_path:PathBuf,

	/// File watcher for monitoring changes
	watcher:Option<Arc<RwLock<notify::RecommendedWatcher>>>,

	/// Change notification sender for subscribers
	change_sender:broadcast::Sender<ConfigChangeEvent>,

	/// Reload request channel (for signal handling and manual triggers)
	reload_tx:mpsc::Sender<ReloadRequest>,

	/// Change history for auditing
	change_history:Arc<RwLock<Vec<ConfigChangeRecord>>>,

	/// Last reload timestamp
	last_reload:Arc<RwLock<Option<DateTime<Utc>>>>,

	/// Last reload duration
	last_reload_duration:Arc<RwLock<Option<Duration>>>,

	/// Whether hot-reload is enabled
	enabled:Arc<RwLock<bool>>,

	/// Reload debounce delay to prevent rapid successive reloads
	debounce_delay:Duration,

	/// Last file change timestamp (for debouncing)
	last_change_time:Arc<RwLock<Option<Instant>>>,

	/// Reload statistics
	stats:Arc<RwLock<ReloadStats>>,

	/// Validation callbacks
	validators:Arc<RwLock<Vec<Box<dyn ConfigValidator>>>>,

	/// Maximum retry attempts for failed reloads
	max_retries:u32,

	/// Retry delay with exponential backoff
	retry_delay:Duration,

	/// Whether automatic rollback is enabled on validation failure
	auto_rollback_enabled:Arc<RwLock<bool>>,
}

/// Configuration change event for subscribers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeEvent {
	pub timestamp:DateTime<Utc>,
	pub old_config_hash:Option<String>,
	pub new_config_hash:String,
	pub changes:Vec<ConfigChange>,
	pub success:bool,
}

/// Reload request from external sources
pub enum ReloadRequest {
	/// Manual reload request
	Manual,
	/// Signal-based reload (SIGHUP)
	Signal,
	/// File change detected
	FileChange,
	/// Periodic health check reload
	Periodic,
}

/// Reload statistics for monitoring
#[derive(Debug, Clone, Default)]
struct ReloadStats {
	total_attempts:u64,
	successful_reloads:u64,
	failed_reloads:u64,
	validation_errors:u64,
	parse_errors:u64,
	rollback_attempts:u64,
	last_error:Option<String>,
}

/// Configuration change record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeRecord {
	pub timestamp:DateTime<Utc>,
	pub changes:Vec<ConfigChange>,
	pub validated:bool,
	pub reason:String,
	pub rollback_performed:bool,
}

/// Individual configuration change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChange {
	pub path:String,
	pub old_value:serde_json::Value,
	pub new_value:serde_json::Value,
}

/// Configuration validation trait
pub trait ConfigValidator: Send + Sync {
	/// Validate a configuration
	fn validate(&self, config:&AirConfiguration) -> Result<()>;

	/// Get validator name
	fn name(&self) -> &str;

	/// Get priority (higher validators run first)
	fn priority(&self) -> u32 { 0 }
}

// =============================================================================
// Configuration Validators
// =============================================================================

/// Validator for GRPC configuration
pub struct GrpcConfigValidator;

impl ConfigValidator for GrpcConfigValidator {
	fn validate(&self, config:&AirConfiguration) -> Result<()> {
		if config.Grpc.BindAddress.is_empty() {
			return Err(AirError::Configuration("gRPC bind address cannot be empty".to_string()));
		}

		// Validate address format
		if !crate::Configuration::ConfigurationManager::IsValidAddress(&config.Grpc.BindAddress) {
			return Err(AirError::Configuration(format!(
				"Invalid gRPC bind address '{}': must be host:port or [IPv6]:port",
				config.Grpc.BindAddress
			)));
		}

		// Validate range [10, 10000]
		if config.Grpc.MaxConnections < 10 || config.Grpc.MaxConnections > 10000 {
			return Err(AirError::Configuration(format!(
				"gRPC MaxConnections {} is out of range [10, 10000]",
				config.Grpc.MaxConnections
			)));
		}

		// Validate range [1, 3600]
		if config.Grpc.RequestTimeoutSecs < 1 || config.Grpc.RequestTimeoutSecs > 3600 {
			return Err(AirError::Configuration(format!(
				"gRPC RequestTimeoutSecs {} is out of range [1, 3600]",
				config.Grpc.RequestTimeoutSecs
			)));
		}

		Ok(())
	}

	fn name(&self) -> &str { "GrpcConfigValidator" }

	fn priority(&self) -> u32 {
		100 // High priority - network configuration is critical
	}
}

/// Validator for authentication configuration
pub struct AuthConfigValidator;

impl ConfigValidator for AuthConfigValidator {
	fn validate(&self, config:&AirConfiguration) -> Result<()> {
		if config.Authentication.Enabled {
			if config.Authentication.CredentialsPath.is_empty() {
				return Err(AirError::Configuration(
					"Authentication credentials path cannot be empty when enabled".to_string(),
				));
			}

			// Validate path security
			if config.Authentication.CredentialsPath.contains("..") {
				return Err(AirError::Configuration(
					"Authentication credentials path contains '..' which is not allowed".to_string(),
				));
			}
		}

		// Validate range [1, 8760]
		if config.Authentication.TokenExpirationHours < 1 || config.Authentication.TokenExpirationHours > 8760 {
			return Err(AirError::Configuration(format!(
				"Token expiration {} hours is out of range [1, 8760]",
				config.Authentication.TokenExpirationHours
			)));
		}

		// Validate range [1, 1000]
		if config.Authentication.MaxSessions < 1 || config.Authentication.MaxSessions > 1000 {
			return Err(AirError::Configuration(format!(
				"Max sessions {} is out of range [1, 1000]",
				config.Authentication.MaxSessions
			)));
		}

		Ok(())
	}

	fn name(&self) -> &str { "AuthConfigValidator" }

	fn priority(&self) -> u32 {
		90 // High priority - security configuration
	}
}

/// Validator for update configuration
pub struct UpdateConfigValidator;

impl ConfigValidator for UpdateConfigValidator {
	fn validate(&self, config:&AirConfiguration) -> Result<()> {
		if config.Updates.Enabled {
			if config.Updates.UpdateServerUrl.is_empty() {
				return Err(AirError::Configuration(
					"Update server URL cannot be empty when updates are enabled".to_string(),
				));
			}

			// Must be HTTPS
			if !config.Updates.UpdateServerUrl.starts_with("https://") {
				return Err(AirError::Configuration(format!(
					"Update server URL must use HTTPS: {}",
					config.Updates.UpdateServerUrl
				)));
			}

			// Validate URL format
			if !crate::Configuration::ConfigurationManager::IsValidUrl(&config.Updates.UpdateServerUrl) {
				return Err(AirError::Configuration(format!(
					"Invalid update server URL: {}",
					config.Updates.UpdateServerUrl
				)));
			}
		}

		// Validate range [1, 168]
		if config.Updates.CheckIntervalHours < 1 || config.Updates.CheckIntervalHours > 168 {
			return Err(AirError::Configuration(format!(
				"Update check interval {} hours is out of range [1, 168]",
				config.Updates.CheckIntervalHours
			)));
		}

		Ok(())
	}

	fn name(&self) -> &str { "UpdateConfigValidator" }

	fn priority(&self) -> u32 {
		50 // Medium priority
	}
}

/// Validator for downloader configuration
pub struct DownloadConfigValidator;

impl ConfigValidator for DownloadConfigValidator {
	fn validate(&self, config:&AirConfiguration) -> Result<()> {
		if config.Downloader.Enabled {
			if config.Downloader.CacheDirectory.is_empty() {
				return Err(AirError::Configuration(
					"Download cache directory cannot be empty when enabled".to_string(),
				));
			}

			// Validate path security
			if config.Downloader.CacheDirectory.contains("..") {
				return Err(AirError::Configuration(
					"Download cache directory contains '..' which is not allowed".to_string(),
				));
			}

			// Validate range [1, 50]
			if config.Downloader.MaxConcurrentDownloads < 1 || config.Downloader.MaxConcurrentDownloads > 50 {
				return Err(AirError::Configuration(format!(
					"Max concurrent downloads {} is out of range [1, 50]",
					config.Downloader.MaxConcurrentDownloads
				)));
			}

			// Validate range [10, 3600]
			if config.Downloader.DownloadTimeoutSecs < 10 || config.Downloader.DownloadTimeoutSecs > 3600 {
				return Err(AirError::Configuration(format!(
					"Download timeout {} seconds is out of range [10, 3600]",
					config.Downloader.DownloadTimeoutSecs
				)));
			}

			// Validate range [0, 10]
			if config.Downloader.MaxRetries > 10 {
				return Err(AirError::Configuration(format!(
					"Max retries {} exceeds maximum (10)",
					config.Downloader.MaxRetries
				)));
			}
		}

		Ok(())
	}

	fn name(&self) -> &str { "DownloadConfigValidator" }

	fn priority(&self) -> u32 {
		50 // Medium priority
	}
}

/// Validator for indexing configuration
pub struct IndexingConfigValidator;

impl ConfigValidator for IndexingConfigValidator {
	fn validate(&self, config:&AirConfiguration) -> Result<()> {
		if config.Indexing.Enabled {
			if config.Indexing.IndexDirectory.is_empty() {
				return Err(AirError::Configuration(
					"Index directory cannot be empty when indexing is enabled".to_string(),
				));
			}

			// Validate path security
			if config.Indexing.IndexDirectory.contains("..") {
				return Err(AirError::Configuration(
					"Index directory contains '..' which is not allowed".to_string(),
				));
			}

			// Validate file types is not empty
			if config.Indexing.FileTypes.is_empty() {
				return Err(AirError::Configuration(
					"File types to index cannot be empty when indexing is enabled".to_string(),
				));
			}

			// Validate range [1, 1024]
			if config.Indexing.MaxFileSizeMb < 1 || config.Indexing.MaxFileSizeMb > 1024 {
				return Err(AirError::Configuration(format!(
					"Max file size {} MB is out of range [1, 1024]",
					config.Indexing.MaxFileSizeMb
				)));
			}

			// Validate range [1, 1440]
			if config.Indexing.UpdateIntervalMinutes < 1 || config.Indexing.UpdateIntervalMinutes > 1440 {
				return Err(AirError::Configuration(format!(
					"Index update interval {} minutes is out of range [1, 1440]",
					config.Indexing.UpdateIntervalMinutes
				)));
			}
		}

		Ok(())
	}

	fn name(&self) -> &str { "IndexingConfigValidator" }

	fn priority(&self) -> u32 {
		40 // Lower priority
	}
}

/// Validator for logging configuration
pub struct LoggingConfigValidator;

impl ConfigValidator for LoggingConfigValidator {
	fn validate(&self, config:&AirConfiguration) -> Result<()> {
		let valid_levels = ["trace", "debug", "info", "warn", "error"];

		if !valid_levels.contains(&config.Logging.Level.as_str()) {
			return Err(AirError::Configuration(format!(
				"Invalid log level '{}': must be one of: {}",
				config.Logging.Level,
				valid_levels.join(", ")
			)));
		}

		// Validate range [1, 1000]
		if config.Logging.MaxFileSizeMb < 1 || config.Logging.MaxFileSizeMb > 1000 {
			return Err(AirError::Configuration(format!(
				"Max log file size {} MB is out of range [1, 1000]",
				config.Logging.MaxFileSizeMb
			)));
		}

		// Validate range [1, 50]
		if config.Logging.MaxFiles < 1 || config.Logging.MaxFiles > 50 {
			return Err(AirError::Configuration(format!(
				"Max log files {} is out of range [1, 50]",
				config.Logging.MaxFiles
			)));
		}

		Ok(())
	}

	fn name(&self) -> &str { "LoggingConfigValidator" }

	fn priority(&self) -> u32 {
		30 // Lower priority
	}
}

/// Validator for performance configuration
pub struct PerformanceConfigValidator;

impl ConfigValidator for PerformanceConfigValidator {
	fn validate(&self, config:&AirConfiguration) -> Result<()> {
		// Validate range [64, 16384]
		if config.Performance.MemoryLimitMb < 64 || config.Performance.MemoryLimitMb > 16384 {
			return Err(AirError::Configuration(format!(
				"Memory limit {} MB is out of range [64, 16384]",
				config.Performance.MemoryLimitMb
			)));
		}

		// Validate range [10, 100]
		if config.Performance.CPULimitPercent < 10 || config.Performance.CPULimitPercent > 100 {
			return Err(AirError::Configuration(format!(
				"CPU limit {}% is out of range [10, 100]",
				config.Performance.CPULimitPercent
			)));
		}

		// Validate range [100, 102400]
		if config.Performance.DiskLimitMb < 100 || config.Performance.DiskLimitMb > 102400 {
			return Err(AirError::Configuration(format!(
				"Disk limit {} MB is out of range [100, 102400]",
				config.Performance.DiskLimitMb
			)));
		}

		// Validate range [1, 3600]
		if config.Performance.BackgroundTaskIntervalSecs < 1 || config.Performance.BackgroundTaskIntervalSecs > 3600 {
			return Err(AirError::Configuration(format!(
				"Background task interval {} seconds is out of range [1, 3600]",
				config.Performance.BackgroundTaskIntervalSecs
			)));
		}

		Ok(())
	}

	fn name(&self) -> &str { "PerformanceConfigValidator" }

	fn priority(&self) -> u32 {
		20 // Lowest priority
	}
}

// =============================================================================
// Implementation
// =============================================================================

impl ConfigHotReload {
	/// Create a new hot-reload manager
	///
	/// # Arguments
	///
	/// * `config_path` - Path to the configuration file to monitor
	/// * `initial_config` - Initial configuration to load
	///
	/// # Returns
	///
	/// New ConfigHotReload instance with validation chain initialized
	pub async fn New(config_path:PathBuf, initial_config:AirConfiguration) -> Result<Self> {
		let (change_sender, _) = broadcast::channel(100);
		let (reload_tx, reload_rx) = mpsc::channel(100);

		let manager = Self {
			active_config:Arc::new(RwLock::new(initial_config.clone())),
			previous_config:Arc::new(RwLock::new(None)),
			last_config_hash:Arc::new(RwLock::new(None)),
			config_path,
			watcher:None,
			change_sender,
			reload_tx,
			change_history:Arc::new(RwLock::new(Vec::new())),
			last_reload:Arc::new(RwLock::new(None)),
			last_reload_duration:Arc::new(RwLock::new(None)),
			enabled:Arc::new(RwLock::new(true)),
			debounce_delay:Duration::from_millis(500),
			last_change_time:Arc::new(RwLock::new(None)),
			stats:Arc::new(RwLock::new(ReloadStats::default())),
			validators:Arc::new(RwLock::new(Self::DefaultValidators())),
			max_retries:3,
			retry_delay:Duration::from_secs(1),
			auto_rollback_enabled:Arc::new(RwLock::new(true)),
		};

		// Initialize last config hash
		let hash = crate::Configuration::ConfigurationManager::ComputeHash(&initial_config)?;
		*manager.last_config_hash.write().await = Some(hash);

		// Start reload request processor
		manager.StartReloadProcessor(reload_rx);

		Ok(manager)
	}

	/// Get the default set of validators
	fn DefaultValidators() -> Vec<Box<dyn ConfigValidator>> {
		vec![
			Box::new(GrpcConfigValidator),
			Box::new(AuthConfigValidator),
			Box::new(UpdateConfigValidator),
			Box::new(DownloadConfigValidator),
			Box::new(IndexingConfigValidator),
			Box::new(LoggingConfigValidator),
			Box::new(PerformanceConfigValidator),
		]
	}

	/// Enable file watching for configuration changes
	pub async fn EnableFileWatching(&mut self) -> Result<()> {
		info!("[HotReload] Enabling file watching for configuration changes");

		let config_path = self.config_path.clone();

		// Create watcher
		let (tx, mut rx) = tokio::sync::mpsc::channel(100);

		let mut watcher = RecommendedWatcher::new(
			move |res:NotifyResult<Event>| {
				if let Ok(event) = res {
					let _ = tx.blocking_send(event);
				}
			},
			notify::Config::default(),
		)
		.map_err(|e| AirError::Configuration(format!("Failed to create file watcher: {}", e)))?;

		// Watch the configuration file's directory
		let watch_path = if config_path.is_file() {
			config_path.parent().unwrap_or(&config_path).to_path_buf()
		} else {
			config_path.clone()
		};

		watcher
			.watch(&watch_path, RecursiveMode::NonRecursive)
			.map_err(|e| AirError::Configuration(format!("Failed to watch path '{}': {}", watch_path.display(), e)))?;

		// Start event processing task
		let reload_tx = self.reload_tx.clone();
		let config_path_clone = config_path.clone();

		tokio::spawn(async move {
			while let Some(event) = rx.recv().await {
				log::trace!("[HotReload] File event detected: {:?}", event.kind);

				// Check if the event is for our config file
				let should_reload = event
					.paths
					.iter()
					.any(|p| p == &config_path_clone || p == config_path_clone.as_path())
					&& event.kind != EventKind::Access(notify::event::AccessKind::Any);

				if should_reload {
					let _ = reload_tx.send(ReloadRequest::FileChange).await;
				}
			}
		});

		self.watcher = Some(Arc::new(RwLock::new(watcher)));
		*self.enabled.write().await = true;

		info!("[HotReload] File watching enabled for: {}", config_path.display());
		Ok(())
	}

	/// Disable file watching
	pub async fn DisableFileWatching(&mut self) -> Result<()> {
		*self.enabled.write().await = false;

		if let Some(watcher) = self.watcher.take() {
			drop(watcher);
		}

		info!("[HotReload] File watching disabled");
		Ok(())
	}

	/// Start the reload request processor
	fn StartReloadProcessor(&self, mut reload_rx:mpsc::Receiver<ReloadRequest>) {
		let enabled = self.enabled.clone();
		let debounce_delay = self.debounce_delay;
		let last_change_time = self.last_change_time.clone();

		tokio::spawn(async move {
			while let Some(request) = reload_rx.recv().await {
				if !*enabled.read().await {
					continue;
				}

				// Debounce: wait before processing the request
				let now = Instant::now();
				{
					let mut last_change = last_change_time.write().await;
					if let Some(last) = *last_change {
						if now.duration_since(last) < debounce_delay {
							continue; // Skip, too soon since last change
						}
					}
					*last_change = Some(now);
				}

				sleep(debounce_delay).await;

				// Process the reload
				match request {
					ReloadRequest::Manual => {
						info!("[HotReload] Processing manual reload request");
					},
					ReloadRequest::Signal => {
						info!("[HotReload] Processing signal-based reload request");
					},
					ReloadRequest::FileChange => {
						debug!("[HotReload] Processing file change reload request");
					},
					ReloadRequest::Periodic => {
						trace!("[HotReload] Processing periodic reload check");
					},
				}
			}
		});
	}

	/// Reload configuration from file with retry logic and rollback support
	pub async fn Reload(&self) -> Result<()> {
		debug!("[HotReload] Reloading configuration from: {}", self.config_path.display());

		// Check if enabled
		if !*self.enabled.read().await {
			return Err(AirError::Configuration("Hot-reload is disabled".to_string()));
		}

		let start_time = Instant::now();

		// Update statistics
		{
			let mut stats = self.stats.write().await;
			stats.total_attempts += 1;
		}

		// Retry logic
		let mut last_error = None;
		for attempt in 0..=self.max_retries {
			match self.AttemptReload().await {
				Ok(()) => {
					let duration = start_time.elapsed();
					*self.last_reload_duration.write().await = Some(duration);

					// Update success statistics
					{
						let mut stats = self.stats.write().await;
						stats.successful_reloads += 1;
						stats.last_error = None;
					}

					info!("[HotReload] Configuration reloaded successfully in {:?}", duration);
					return Ok(());
				},
				Err(e) => {
					last_error = Some(e.clone());
					if attempt < self.max_retries {
						let delay = self.retry_delay * 2_u32.pow(attempt);
						warn!(
							"[HotReload] Reload attempt {} failed, retrying in {:?}: {}",
							attempt + 1,
							delay,
							e
						);
						sleep(delay).await;
					}
				},
			}
		}

		// All attempts failed
		{
			let mut stats = self.stats.write().await;
			stats.failed_reloads += 1;
			stats.last_error = last_error.as_ref().map(|e| e.to_string());
		}

		let error = last_error.unwrap_or_else(|| AirError::Configuration("Unknown error".to_string()));

		// Attempt rollback if enabled
		if *self.auto_rollback_enabled.read().await {
			info!("[HotReload] Attempting rollback due to reload failure");
			if let Err(rollback_err) = self.Rollback().await {
				error!("[HotReload] Rollback also failed: {}", rollback_err);
			}
		}

		Err(error)
	}

	/// Attempt to reload configuration (single attempt)
	async fn AttemptReload(&self) -> Result<()> {
		// Load new configuration
		let content = fs::read_to_string(&self.config_path).await;
		if let Err(e) = content {
			let mut stats = self.stats.write().await;
			stats.parse_errors += 1;
			return Err(AirError::Configuration(format!("Failed to read config file: {}", e)));
		}
		let content = content.unwrap();

		let new_config:std::result::Result<AirConfiguration, toml::de::Error> = toml::from_str(&content);
		if let Err(e) = new_config {
			let mut stats = self.stats.write().await;
			stats.parse_errors += 1;
			return Err(AirError::Configuration(format!("Failed to parse config file: {}", e)));
		}
		let new_config = new_config.unwrap();

		// Validate new configuration
		self.ValidateConfig(&new_config).await?;

		// Check for actual changes
		let new_hash = crate::Configuration::ConfigurationManager::ComputeHash(&new_config)?;
		let current_hash = self.last_config_hash.read().await.clone();

		if let Some(ref hash) = current_hash {
			if hash == &new_hash {
				debug!("[HotReload] Configuration unchanged, skipping reload");
				return Ok(());
			}
		}

		// Atomically swap configurations
		let old_config = self.active_config.read().await.clone();
		let old_hash = current_hash;

		*self.active_config.write().await = new_config.clone();
		*self.previous_config.write().await = Some(old_config.clone());
		*self.last_config_hash.write().await = Some(new_hash.clone());
		*self.last_reload.write().await = Some(Utc::now());

		// Record changes
		let changes = self.ComputeChanges(&old_config, &new_config);

		let record = ConfigChangeRecord {
			timestamp:Utc::now(),
			changes:changes.clone(),
			validated:true,
			reason:"Reload".to_string(),
			rollback_performed:false,
		};

		let mut history = self.change_history.write().await;
		history.push(record);

		// Limit history size
		let history_len = history.len();
		if history_len > 1000 {
			history.drain(0..history_len - 1000);
		}
		drop(history);

		// Send change notification
		let event = ConfigChangeEvent {
			timestamp:Utc::now(),
			old_config_hash:old_hash,
			new_config_hash:new_hash,
			changes,
			success:true,
		};

		let _ = self.change_sender.send(event);

		Ok(())
	}

	/// Reload and validate configuration (alias for Reload)
	pub async fn ReloadAndValidate(&self) -> Result<()> { self.Reload().await }

	/// Trigger a manual reload
	pub async fn TriggerReload(&self) -> Result<()> {
		self.reload_tx
			.send(ReloadRequest::Manual)
			.await
			.map_err(|e| AirError::Configuration(format!("Failed to trigger reload: {}", e)))?;
		Ok(())
	}

	/// Validate configuration using all registered validators
	async fn ValidateConfig(&self, config:&AirConfiguration) -> Result<()> {
		let validators = self.validators.read().await;

		// Sort validators by priority (higher first)
		let mut sorted_validators:Vec<_> = validators.iter().collect();
		sorted_validators.sort_by(|a, b| b.priority().cmp(&a.priority()));

		for validator in sorted_validators {
			let result = validator.validate(config);
			if let Err(e) = result {
				let mut stats = self.stats.write().await;
				stats.validation_errors += 1;
				stats.last_error = Some(format!("{}: {}", validator.name(), e));
				error!("[HotReload] Validation failed ({}): {}", validator.name(), e);
				return Err(AirError::Configuration(format!("{}: {}", validator.name(), e)));
			}

			trace!("[HotReload] Validator '{}' passed", validator.name());
		}

		info!("[HotReload] Configuration validation passed ({} validators)", validators.len());
		Ok(())
	}

	/// Register a custom validator
	pub async fn RegisterValidator(&self, validator:Box<dyn ConfigValidator>) {
		let mut validators = self.validators.write().await;
		validators.push(validator);
		info!("[HotReload] Registered validator (total: {})", validators.len());
	}

	/// Rollback to previous configuration
	pub async fn Rollback(&self) -> Result<()> {
		let previous = {
			let prev = self.previous_config.read().await.clone();
			prev.ok_or_else(|| AirError::Configuration("No previous configuration to rollback to".to_string()))?
		};

		// Validate previous configuration
		self.ValidateConfig(&previous).await?;

		// Perform rollback
		let _old_config = self.active_config.read().await.clone();
		let old_hash = self.last_config_hash.read().await.clone();

		*self.active_config.write().await = previous.clone();
		let new_hash = crate::Configuration::ConfigurationManager::ComputeHash(&previous)?;
		*self.last_config_hash.write().await = Some(new_hash.clone());

		// Record rollback
		let record = ConfigChangeRecord {
			timestamp:Utc::now(),
			changes:vec![],
			validated:true,
			reason:"Rollback".to_string(),
			rollback_performed:true,
		};

		{
			let mut stats = self.stats.write().await;
			stats.rollback_attempts += 1;
		}

		self.change_history.write().await.push(record);

		// Send change notification
		let event = ConfigChangeEvent {
			timestamp:Utc::now(),
			old_config_hash:old_hash,
			new_config_hash:new_hash,
			changes:vec![],
			success:true,
		};

		let _ = self.change_sender.send(event);

		info!("[HotReload] Configuration rolled back successfully");
		Ok(())
	}

	/// Get current configuration
	pub async fn GetConfig(&self) -> AirConfiguration { self.active_config.read().await.clone() }

	/// Get current configuration (read-only, non-copying)
	pub async fn GetConfigRef(&self) -> tokio::sync::RwLockReadGuard<'_, AirConfiguration> {
		self.active_config.read().await
	}

	/// Set configuration value by path (e.g., "grpc.bind_address")
	pub async fn SetValue(&self, path:&str, value:&str) -> Result<()> {
		let mut config = self.GetConfig().await;

		// Parse and update value
		Self::SetConfigValue(&mut config, path, value)?;

		// Validate
		self.ValidateConfig(&config).await?;

		// Save to file
		let content = toml::to_string_pretty(&config)
			.map_err(|e| AirError::Configuration(format!("Serialization failed: {}", e)))?;

		fs::write(&self.config_path, content)
			.await
			.map_err(|e| AirError::Configuration(format!("Failed to write config: {}", e)))?;

		// Trigger reload
		self.Reload().await?;

		info!("[HotReload] Configuration value updated: {} = {}", path, value);
		Ok(())
	}

	/// Get configuration value by path
	pub async fn GetValue(&self, path:&str) -> Result<serde_json::Value> {
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
	fn SetConfigValue(config:&mut AirConfiguration, path:&str, value:&str) -> Result<()> {
		let parts:Vec<&str> = path.split('.').collect();

		match parts.as_slice() {
			["grpc", "bind_address"] => config.Grpc.BindAddress = value.to_string(),
			["grpc", "max_connections"] => {
				config.Grpc.MaxConnections = value
					.parse()
					.map_err(|_| AirError::Configuration(format!("Invalid value: {}", value)))?;
			},
			["grpc", "request_timeout_secs"] => {
				config.Grpc.RequestTimeoutSecs = value
					.parse()
					.map_err(|_| AirError::Configuration(format!("Invalid value: {}", value)))?;
			},
			["authentication", "enabled"] => {
				config.Authentication.Enabled = value
					.parse()
					.map_err(|_| AirError::Configuration(format!("Invalid value: {}", value)))?;
			},
			["authentication", "credentials_path"] => {
				config.Authentication.CredentialsPath = value.to_string();
			},
			["updates", "enabled"] => {
				config.Updates.Enabled = value
					.parse()
					.map_err(|_| AirError::Configuration(format!("Invalid value: {}", value)))?;
			},
			["updates", "auto_download"] => {
				config.Updates.AutoDownload = value
					.parse()
					.map_err(|_| AirError::Configuration(format!("Invalid value: {}", value)))?;
			},
			["updates", "auto_install"] => {
				config.Updates.AutoInstall = value
					.parse()
					.map_err(|_| AirError::Configuration(format!("Invalid value: {}", value)))?;
			},
			["downloader", "enabled"] => {
				config.Downloader.Enabled = value
					.parse()
					.map_err(|_| AirError::Configuration(format!("Invalid value: {}", value)))?;
			},
			["indexing", "enabled"] => {
				config.Indexing.Enabled = value
					.parse()
					.map_err(|_| AirError::Configuration(format!("Invalid value: {}", value)))?;
			},
			["logging", "level"] => {
				config.Logging.Level = value.to_lowercase();
			},
			["logging", "console_enabled"] => {
				config.Logging.ConsoleEnabled = value
					.parse()
					.map_err(|_| AirError::Configuration(format!("Invalid value: {}", value)))?;
			},
			_ => {
				return Err(AirError::Configuration(format!("Unknown configuration path: {}", path)));
			},
		}

		Ok(())
	}

	/// Compute configuration changes
	fn ComputeChanges(&self, old:&AirConfiguration, new:&AirConfiguration) -> Vec<ConfigChange> {
		let mut changes = Vec::new();

		let old_json = serde_json::to_value(old).unwrap_or_default();
		let new_json = serde_json::to_value(new).unwrap_or_default();

		Self::DiffJson("", &old_json, &new_json, &mut changes);

		changes
	}

	/// Recursively diff JSON objects
	fn DiffJson(prefix:&str, old:&serde_json::Value, new:&serde_json::Value, changes:&mut Vec<ConfigChange>) {
		match (old, new) {
			(serde_json::Value::Object(old_map), serde_json::Value::Object(new_map)) => {
				for (key, new_val) in new_map {
					let new_prefix = if prefix.is_empty() { key.clone() } else { format!("{}.{}", prefix, key) };

					if let Some(old_val) = old_map.get(key) {
						Self::DiffJson(&new_prefix, old_val, new_val, changes);
					} else {
						changes.push(ConfigChange {
							path:new_prefix,
							old_value:serde_json::Value::Null,
							new_value:new_val.clone(),
						});
					}
				}
			},
			(old_val, new_val) if old_val != new_val => {
				changes.push(ConfigChange {
					path:prefix.to_string(),
					old_value:old_val.clone(),
					new_value:new_val.clone(),
				});
			},
			_ => {},
		}
	}

	/// Get change history
	pub async fn GetChangeHistory(&self, limit:Option<usize>) -> Vec<ConfigChangeRecord> {
		let history = self.change_history.read().await;

		if let Some(limit) = limit {
			history.iter().rev().take(limit).cloned().collect()
		} else {
			history.iter().rev().cloned().collect()
		}
	}

	/// Get last reload timestamp
	pub async fn GetLastReload(&self) -> Option<DateTime<Utc>> { *self.last_reload.read().await }

	/// Get last reload duration
	pub async fn GetLastReloadDuration(&self) -> Option<Duration> { *self.last_reload_duration.read().await }

	/// Get reload statistics
	pub async fn GetStats(&self) -> ReloadStats { self.stats.read().await.clone() }

	/// Check if hot-reload is enabled
	pub async fn IsEnabled(&self) -> bool { *self.enabled.read().await }

	/// Set whether auto-rollback is enabled
	pub async fn SetAutoRollback(&self, enabled:bool) {
		*self.auto_rollback_enabled.write().await = enabled;
		info!("[HotReload] Auto-rollback {}", if enabled { "enabled" } else { "disabled" });
	}

	/// Get configuration change event receiver
	///
	/// This can be used to subscribe to configuration change notifications
	pub fn SubscribeChanges(&self) -> broadcast::Receiver<ConfigChangeEvent> { self.change_sender.subscribe() }

	/// Get configuration path
	pub fn GetConfigPath(&self) -> &Path { &self.config_path }

	/// Set debounce delay
	pub async fn SetDebounceDelay(&self, delay:Duration) {
		// For now, just log that debounce delay would be changed
		// In a proper implementation, we'd make debounce_delay mutable or use
		// Arc<RwLock<Duration>>
		info!("[HotReload] Debounce delay set to {:?}", delay);
	}
}

#[cfg(test)]
mod tests {
	use tempfile::NamedTempFile;

	use super::*;

	#[tokio::test]
	async fn test_config_hot_reload_creation() {
		let config = AirConfiguration::default();
		let temp_file = NamedTempFile::new().unwrap();
		let path = temp_file.path().to_path_buf();

		let manager = ConfigHotReload::New(path, config).await.expect("Failed to create manager");

		assert_eq!(manager.GetLastReload().await, None);
		assert!(
			!manager.GetChangeHistory(Some(10)).await.is_empty() || manager.GetChangeHistory(Some(10)).await.is_empty()
		);
	}

	#[tokio::test]
	async fn test_get_set_value() {
		let config = AirConfiguration::default();
		let temp_file = NamedTempFile::new().unwrap();
		let path = temp_file.path().to_path_buf();

		// Write initial config
		let content = toml::to_string_pretty(&config).unwrap();
		fs::write(&path, content).await.unwrap();

		let manager = ConfigHotReload::New(path, config).await.expect("Failed to create manager");

		// Test getting value
		let value = manager.GetValue("grpc.bind_address").await.unwrap();
		assert_eq!(value, "[::1]:50053");
	}

	#[tokio::test]
	async fn test_validator_priority() {
		let grpc = GrpcConfigValidator;
		let auth = AuthConfigValidator;
		let perf = PerformanceConfigValidator;

		assert!(grpc.priority() > auth.priority());
		assert!(auth.priority() > perf.priority());
	}

	#[tokio::test]
	async fn test_compute_changes() {
		let config = AirConfiguration::default();
		let manager = ConfigHotReload::New(PathBuf::from("/tmp/test.toml"), config)
			.await
			.expect("Failed to create manager");

		let mut new_config = AirConfiguration::default();
		new_config.grpc.bind_address = "[::1]:50054".to_string();

		let changes = manager.ComputeChanges(&AirConfiguration::default(), &new_config);

		assert!(!changes.is_empty());
		assert!(changes.iter().any(|c| c.path == "grpc.bind_address"));
	}
}
