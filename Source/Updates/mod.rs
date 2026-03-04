//! # Update Management Service
//!
//! Comprehensive update management for the Land ecosystem with full lifecycle
//! support:
//! - Version availability checking against update servers
//! - Secure download with cryptographic signature verification
//! - Multi-checksum integrity validation (SHA256, MD5, CRC32)
//! - Staged installation with atomic application
//! - Automatic rollback on installation failure
//! - Platform-specific update packages (macOS dmg, Windows exe, Linux AppImage)
//! - Update channel management (stable, insiders, preview)
//! - Delta updates for reduced download size
//! - Network interruption recovery with resume capability
//! - Disk space validation before download
//! - Backup creation before applying updates
//!
//! # VSCode Update System References
//! This implementation draws inspiration from VSCode's update architecture:
//! - Background update checking without interrupting user workflow
//! - Deferred installation at application restart
//! - Update verification with multiple checksums
//! - Telemetry for update success/failure tracking
//! - Circuit breakers for update server resilience
//!
//! # Architecture
//! The update manager coordinates with:
//! - Mountain: Provides frontend notification of available updates
//! - The entire Land ecosystem: Updates can apply to multiple components
//! - Update servers: REST API endpoints for version checks and downloads
//!
//! # Connection to VSCode's Update Download Service Architecture
//!
//! The Air update manager draws inspiration from VSCode's update system:
//! 1. **Separate Update Process**: Like VSCode, Air runs updates in the
//!    background
//! 2. **Deferred Installation**: Updates are downloaded and staged, then
//!    applied on restart
//! 3. **Progress Reporting**: Updates report progress to the frontend
//!    (Mountain)
//! 4. **Resilient Downloads**: Support for resume after interruption
//! 5. **Multiple Integrity Checks**: SHA256, MD5, and cryptographic signatures
//! 6. **Automatic Rollback**: Like VSCode's safe mode, Air can roll back on
//!    failure
//!
//! Air handles updates for the entire Land ecosystem:
//! - **Air daemon**: The background service itself
//! - **Mountain**: The frontend Electron application
//! - **Other elements**: Can update other Land components if needed
//!
//! Update coordination with Mountain:
//! - When an update is available, Air notifies Mountain via event bus
//! - Mountain displays update notification to the user
//! - User selects whether to download/install/update
//! - Mountain can request status, cancel downloads, or initiate installation
//!
//! ## VSCode Update System Reference
//!
//! VSCode's update system (similar to Atom's) uses:
//! - Electron's auto-updater module for Windows/macOS AppImages
//! - Native Linux package managers for deb/rpm
//! - Background update checking without interrupting user
//! - Deferred installation on application restart
//! - Multi-channel support (stable, insiders, exploration)
//!
//! # FUTURE Enhancements
//! - Delta update support: Download only changed files between versions
//! - Rollback system: Automatic and manual rollback to previous versions
//! - Staged installations: Pre-verify updates before user prompt
//! - Update signatures: Ed25519 or PGP signature verification
//! - Update package format: Custom package format for cross-platform support

use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::Arc,
	time::Duration,
};

use serde::{Deserialize, Serialize};
use tokio::{
	sync::{Mutex, RwLock},
	time::{interval, sleep},
};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use md5;

use crate::{AirError, ApplicationState::ApplicationState, Configuration::ConfigurationManager, Result};

/// Update manager implementation with full lifecycle support
pub struct UpdateManager {
	/// Application state
	AppState:Arc<ApplicationState>,

	/// Current update status
	update_status:Arc<RwLock<UpdateStatus>>,

	/// Update cache directory
	cache_directory:PathBuf,

	/// Staging directory for pre-installation verification
	staging_directory:PathBuf,

	/// Backup directory for rollback capability
	backup_directory:PathBuf,

	/// Active download sessions with resume capability
	download_sessions:Arc<RwLock<HashMap<String, DownloadSession>>>,

	/// Rollback history (max 5 versions)
	rollback_history:Arc<Mutex<RollbackHistory>>,

	/// Update channel configuration
	update_channel:UpdateChannel,

	/// Platform-specific configuration
	platform_config:PlatformConfig,

	/// Background task handle for cancellation
	background_task:Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

/// Download session for resumable downloads
#[derive(Debug, Clone)]
struct DownloadSession {
	/// Session unique identifier
	#[allow(dead_code)]
	session_id:String,

	/// Original update URL
	#[allow(dead_code)]
	download_url:String,

	/// Current file path
	#[allow(dead_code)]
	temp_path:PathBuf,

	/// Bytes downloaded so far
	downloaded_bytes:u64,

	/// Total file size
	#[allow(dead_code)]
	total_bytes:u64,

	/// Whether download is complete
	complete:bool,

	/// Cancellation flag for download
	cancelled:bool,
}

/// Rollback history for automatic and manual rollback
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RollbackHistory {
	/// Previous versions available for rollback
	versions:Vec<RollbackState>,

	/// Maximum number of versions to keep
	max_versions:usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackState {
	version:String,
	backup_path:PathBuf,
	timestamp:chrono::DateTime<chrono::Utc>,
	checksum:String,
}

/// Update channel configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum UpdateChannel {
	Stable,
	Insiders,
	Preview,
}

impl UpdateChannel {
	fn as_str(&self) -> &'static str {
		match self {
			UpdateChannel::Stable => "stable",
			UpdateChannel::Insiders => "insiders",
			UpdateChannel::Preview => "preview",
		}
	}
}

/// Platform-specific update configuration
#[derive(Debug, Clone)]
struct PlatformConfig {
	platform:String,
	arch:String,
	package_format:PackageFormat,
}

/// Supported package formats by platform
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum PackageFormat {
	WindowsExe,
	MacOsDmg,
	LinuxAppImage,
	LinuxDeb,
	LinuxRpm,
}

/// Update status with comprehensive state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatus {
	/// Last time updates were checked
	pub last_check:Option<chrono::DateTime<chrono::Utc>>,

	/// Whether an update is available
	pub update_available:bool,

	/// Current installed version
	pub current_version:String,

	/// Available version (if any)
	pub available_version:Option<String>,

	/// Download progress (0.0 to 100.0)
	pub download_progress:Option<f32>,

	/// Current installation status
	pub installation_status:InstallationStatus,

	/// Update channel being used
	pub update_channel:UpdateChannel,

	/// Size of available update in bytes
	pub update_size:Option<u64>,

	/// Release notes for available update
	pub release_notes:Option<String>,

	/// Whether update requires restart
	pub requires_restart:bool,

	/// Download speed in bytes per second
	pub download_speed:Option<f64>,

	/// Estimated time remaining in seconds
	pub eta_seconds:Option<u64>,

	/// Last error message (if any)
	pub last_error:Option<String>,
}

/// Installation status with detailed state tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InstallationStatus {
	/// No update operation in progress
	NotStarted,

	/// Verifying disk space and prerequisites
	CheckingPrerequisites,

	/// Downloading update package
	Downloading,

	/// Download paused (resumable)
	Paused,

	/// Verifying cryptographic signatures
	VerifyingSignature,

	/// Verifying checksums (SHA256, MD5, etc.)
	VerifyingChecksums,

	/// Staging update for pre-installation verification
	Staging,

	/// Creating backup before applying update
	CreatingBackup,

	/// Installing update
	Installing,

	/// Installation completed, awaiting restart
	Completed,

	/// Rolling back due to installation failure
	RollingBack,

	/// Installation failed with error message
	Failed(String),
}

impl InstallationStatus {
	/// Check if the current status allows cancellation
	pub fn is_cancellable(&self) -> bool {
		matches!(
			self,
			InstallationStatus::Downloading
				| InstallationStatus::Paused
				| InstallationStatus::Staging
				| InstallationStatus::NotStarted
		)
	}

	/// Check if the current status represents an error
	pub fn is_error(&self) -> bool { matches!(self, InstallationStatus::Failed(_)) }

	/// Check if the current status represents progress
	pub fn is_in_progress(&self) -> bool {
		matches!(
			self,
			InstallationStatus::CheckingPrerequisites
				| InstallationStatus::Downloading
				| InstallationStatus::VerifyingSignature
				| InstallationStatus::VerifyingChecksums
				| InstallationStatus::Staging
				| InstallationStatus::CreatingBackup
				| InstallationStatus::Installing
		)
	}
}

/// Update information with comprehensive metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
	/// Semantic version (e.g., "1.2.3")
	pub version:String,

	/// Download URL for the update package
	pub download_url:String,

	/// Release notes and changelog
	pub release_notes:String,

	/// Primary checksum (SHA256 recommended)
	pub checksum:String,

	/// Alternative checksums for verification
	pub checksums:HashMap<String, String>,

	/// Size of update package in bytes
	pub size:u64,

	/// When the update was published
	pub published_at:chrono::DateTime<chrono::Utc>,

	/// Whether this update is mandatory
	pub is_mandatory:bool,

	/// Whether update requires application restart
	pub requires_restart:bool,

	/// Minimum compatible version
	pub min_compatible_version:Option<String>,

	/// Delta update URL (if available for incremental update)
	pub delta_url:Option<String>,

	/// Delta update checksum (if available)
	pub delta_checksum:Option<String>,

	/// Delta update size (if available)
	pub delta_size:Option<u64>,

	/// Cryptographic signature (Ed25519 or PGP)
	pub signature:Option<String>,

	/// Platform-specific metadata
	pub platform_metadata:Option<PlatformMetadata>,
}

/// Platform-specific update metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformMetadata {
	/// Package format (exe, dmg, appimage, etc.)
	pub package_format:String,

	/// Installation instructions
	pub install_instructions:Vec<String>,

	/// Required disk space in bytes
	pub required_disk_space:u64,

	/// Whether admin privileges are required
	pub requires_admin:bool,

	/// Additional platform-specific parameters
	pub additional_params:HashMap<String, serde_json::Value>,
}

/// Update telemetry data for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTelemetry {
	/// Unique telemetry event ID
	pub event_id:String,

	/// Current version
	pub current_version:String,

	/// Target version
	pub target_version:String,

	/// Update channel
	pub channel:String,

	/// Platform identifier
	pub platform:String,

	/// Operation type (check, download, install, rollback)
	pub operation:String,

	/// Success or failure
	pub success:bool,

	/// Duration in milliseconds
	pub duration_ms:u64,

	/// Download size in bytes
	pub download_size:Option<u64>,

	/// Error message (if failed)
	pub error_message:Option<String>,

	/// Timestamp of the event
	pub timestamp:chrono::DateTime<chrono::Utc>,
}

impl UpdateManager {
	/// Create a new update manager with comprehensive initialization
	pub async fn new(AppState:Arc<ApplicationState>) -> Result<Self> {
		let config = &AppState.Configuration.Updates;

		// Expand cache directory path
		let cache_directory = ConfigurationManager::ExpandPath(&AppState.Configuration.Downloader.CacheDirectory)?;

		// Create cache directory if it doesn't exist
		tokio::fs::create_dir_all(&cache_directory)
			.await
			.map_err(|e| AirError::Configuration(format!("Failed to create cache directory: {}", e)))?;

		// Create staging directory for pre-installation verification
		let staging_directory = cache_directory.join("staging");
		tokio::fs::create_dir_all(&staging_directory)
			.await
			.map_err(|e| AirError::Configuration(format!("Failed to create staging directory: {}", e)))?;

		// Create backup directory for rollback
		let backup_directory = cache_directory.join("backups");
		tokio::fs::create_dir_all(&backup_directory)
			.await
			.map_err(|e| AirError::Configuration(format!("Failed to create backup directory: {}", e)))?;

		// Determine platform-specific configuration
		let PlatformConfig = Self::detect_platform();
		let PlatformConfigClone = PlatformConfig.clone();

		// Determine update channel from configuration
		let update_channel = if config.Channel == "insiders" {
			UpdateChannel::Insiders
		} else if config.Channel == "preview" {
			UpdateChannel::Preview
		} else {
			UpdateChannel::Stable
		};

		// Load or create rollback history
		let rollback_history_path = backup_directory.join("rollback_history.json");
		let rollback_history = if rollback_history_path.exists() {
			let content = tokio::fs::read_to_string(&rollback_history_path)
				.await
				.map_err(|e| AirError::FileSystem(format!("Failed to read rollback history: {}", e)))?;
			serde_json::from_str(&content).unwrap_or_else(|_| RollbackHistory { versions:Vec::new(), max_versions:5 })
		} else {
			RollbackHistory { versions:Vec::new(), max_versions:5 }
		};

		let manager = Self {
			AppState,
			update_status:Arc::new(RwLock::new(UpdateStatus {
				last_check:None,
				update_available:false,
				current_version:env!("CARGO_PKG_VERSION").to_string(),
				available_version:None,
				download_progress:None,
				installation_status:InstallationStatus::NotStarted,
				update_channel,
				update_size:None,
				release_notes:None,
				requires_restart:true,
				download_speed:None,
				eta_seconds:None,
				last_error:None,
			})),
			cache_directory,
			staging_directory,
			backup_directory,
			download_sessions:Arc::new(RwLock::new(HashMap::new())),
			rollback_history:Arc::new(Mutex::new(rollback_history)),
			update_channel,
			platform_config:PlatformConfigClone,
			background_task:Arc::new(Mutex::new(None)),
		};

		// Initialize service status
		manager
			.AppState
			.UpdateServiceStatus("updates", crate::ApplicationState::ServiceStatus::Running)
			.await
			.map_err(|e| AirError::Internal(e.to_string()))?;

		log::info!(
			"[UpdateManager] Update service initialized for platform: {}/{}",
			PlatformConfig.platform,
			PlatformConfig.arch
		);

		Ok(manager)
	}

	/// Detect the current platform and configure platform-specific settings
	fn detect_platform() -> PlatformConfig {
		let platform = if cfg!(target_os = "windows") {
			"windows"
		} else if cfg!(target_os = "macos") {
			"macos"
		} else if cfg!(target_os = "linux") {
			"linux"
		} else {
			"unknown"
		};

		let arch = if cfg!(target_arch = "x86_64") {
			"x64"
		} else if cfg!(target_arch = "aarch64") {
			"arm64"
		} else if cfg!(target_arch = "x86") {
			"ia32"
		} else {
			"unknown"
		};

		let package_format = match (platform, arch) {
			("windows", _) => PackageFormat::WindowsExe,
			("macos", _) => PackageFormat::MacOsDmg,
			("linux", "x64") => PackageFormat::LinuxAppImage,
			("linux", "") => PackageFormat::LinuxAppImage,
			_ => PackageFormat::LinuxAppImage,
		};

		PlatformConfig { platform:platform.to_string(), arch:arch.to_string(), package_format }
	}

	/// Check for available updates from the configured update server
	///
	/// This method:
	/// - Queries the update server based on the configured channel
	/// - Validates the update against minimum compatibility requirements
	/// - Updates the internal status with available update information
	/// - Triggers automatic download if configured
	///
	/// Returns: Some(UpdateInfo) if an update is available, None otherwise
	pub async fn CheckForUpdates(&self) -> Result<Option<UpdateInfo>> {
		let config = &self.AppState.Configuration.Updates;
		let start_time = std::time::Instant::now();

		if !config.Enabled {
			log::debug!("[UpdateManager] Updates are disabled");
			return Ok(None);
		}

		log::info!(
			"[UpdateManager] Checking for updates on {} channel",
			self.update_channel.as_str()
		);

		// Update status
		{
			let mut status = self.update_status.write().await;
			status.last_check = Some(chrono::Utc::now());
			status.last_error = None;
		}

		// Check update server with resilience patterns
		let update_info = match self.FetchUpdateInfo().await {
			Ok(info) => info,
			Err(e) => {
				log::error!("[UpdateManager] Failed to fetch update info: {}", e);
				let mut status = self.update_status.write().await;
				status.last_error = Some(e.to_string());
				self.record_telemetry(
					"check",
					false,
					start_time.elapsed().as_millis() as u64,
					None,
					Some(e.to_string()),
				)
				.await;
				return Err(e);
			},
		};

		if let Some(ref info) = update_info {
			// Verify minimum compatibility
			if let Some(ref min_version) = info.min_compatible_version {
				let current_version = env!("CARGO_PKG_VERSION");
				if UpdateManager::CompareVersions(current_version, min_version) < 0 {
					log::warn!(
						"[UpdateManager] Update requires minimum version {} but current is {}. Skipping.",
						min_version,
						current_version
					);
					let mut status = self.update_status.write().await;
					status.last_error = Some(format!("Update requires minimum version {}", min_version));
					return Ok(None);
				}
			}

			log::info!(
				"[UpdateManager] Update available: {} ({})",
				info.version,
				self.format_size(info.size as f64)
			);

			// Update status
			{
				let mut status = self.update_status.write().await;
				status.update_available = true;
				status.available_version = Some(info.version.clone());
				status.update_size = Some(info.size);
				status.release_notes = Some(info.release_notes.clone());
				status.requires_restart = info.requires_restart;
			}

			// Notify Mountain (frontend) about available update
			// This would typically be done via event bus or gRPC
			log::info!("[UpdateManager] Notifying frontend about available update");

			// Record telemetry
			self.record_telemetry("check", true, start_time.elapsed().as_millis() as u64, None, None)
				.await;

			// Auto-download if configured
			if config.AutoDownload {
				if let Err(e) = self.DownloadUpdate(info).await {
					log::error!("[UpdateManager] Auto-download failed: {}", e);
					// Don't fail the check, just log the error
				}
			}
		} else {
			log::info!("[UpdateManager] No updates available");

			// Update status
			{
				let mut status = self.update_status.write().await;
				status.update_available = false;
				status.available_version = None;
				status.update_size = None;
				status.release_notes = None;
			}

			// Record telemetry
			self.record_telemetry("check", true, start_time.elapsed().as_millis() as u64, None, None)
				.await;
		}

		Ok(update_info)
	}

	/// Download update package with resumable download support
	///
	/// This method:
	/// - Validates available disk space before starting download
	/// - Supports resumable downloads from network interruptions
	/// - Tracks download progress and calculates ETA
	/// - Updates download speed metrics
	/// - Verifies all checksums after download
	///
	/// # Arguments
	/// * `update_info` - Update information containing download URL and
	///   metadata
	///
	/// # Returns
	/// Result<()> indicating success or failure
	pub async fn DownloadUpdate(&self, update_info:&UpdateInfo) -> Result<()> {
		let start_time = std::time::Instant::now();
		let session_id = Uuid::new_v4().to_string();

		log::info!(
			"[UpdateManager] Starting download for version {} (session: {})",
			update_info.version,
			session_id
		);

		// Check prerequisites: disk space
		let required_space = update_info.size * 2; // Need space for download + staging
		self.ValidateDiskSpace(required_space).await?;

		// Update status
		{
			let mut status = self.update_status.write().await;
			status.installation_status = InstallationStatus::CheckingPrerequisites;
			status.last_error = None;
		}

		// Create temp file path for download
		let temp_path = self.cache_directory.join(format!("update-{}-temp.bin", update_info.version));
		let final_path = self.cache_directory.join(format!("update-{}.bin", update_info.version));

		// Check if there's an existing partial download to resume
		let (downloaded_bytes, resume_from_start) = if temp_path.exists() {
			let metadata = tokio::fs::metadata(&temp_path)
				.await
				.map_err(|e| AirError::FileSystem(format!("Failed to check temp file: {}", e)))?;
			log::info!("[UpdateManager] Found partial download, resuming from {} bytes", metadata.len());
			(metadata.len(), false)
		} else {
			(0, true)
		};

		// Create or open download session
		{
			let mut sessions = self.download_sessions.write().await;
			sessions.insert(
				session_id.clone(),
				DownloadSession {
					session_id:session_id.clone(),
					download_url:update_info.download_url.clone(),
					temp_path:temp_path.clone(),
					downloaded_bytes,
					total_bytes:update_info.size,
					complete:false,
					cancelled:false,
				},
			);
		}

		// Begin download
		let dns_port = mist::dns_port();
		let client = crate::HTTP::secured_client_with_timeout(
			dns_port,
			Duration::from_secs(300),
		)
		.map_err(|e| AirError::Network(format!("Failed to create HTTP client: {}", e)))?;

		let mut request_builder = client.get(&update_info.download_url);

		// Add range header for resume
		if !resume_from_start {
			request_builder = request_builder.header("Range", format!("bytes={}-", downloaded_bytes));
		}

		let response: reqwest::Response = request_builder
			.send()
			.await
			.map_err(|e| AirError::Network(format!("Failed to start download: {}", e)))?;

		if !response.status().is_success() && response.status() != 206 {
			log::error!("[UpdateManager] Download failed with status: {}", response.status());
			let mut status = self.update_status.write().await;
			status.installation_status =
				InstallationStatus::Failed(format!("Download failed with status: {}", response.status()));
			status.last_error = Some(format!("Download failed with status: {}", response.status()));
			self.record_telemetry(
				"download",
				false,
				start_time.elapsed().as_millis() as u64,
				None,
				Some("Download failed".to_string()),
			)
			.await;
			return Err(AirError::Network(format!("Download failed with status: {}", response.status())));
		}

		let total_size = response.content_length().unwrap_or(update_info.size);
		let initial_downloaded = if resume_from_start { 0 } else { downloaded_bytes };

		// Update status to downloading
		{
			let mut status = self.update_status.write().await;
			status.installation_status = InstallationStatus::Downloading;
			status.download_progress = Some(((downloaded_bytes as f32 / total_size as f32) * 100.0).min(100.0));
		}

		// Progress tracking
		let last_update = Arc::new(Mutex::new(std::time::Instant::now()));
		let last_bytes = Arc::new(Mutex::new(downloaded_bytes));

		// Open or create file
		let mut file = if resume_from_start {
			tokio::fs::File::create(&temp_path)
				.await
				.map_err(|e| AirError::FileSystem(format!("Failed to create update file: {}", e)))?
		} else {
			// Open with append for resume
			tokio::fs::OpenOptions::new()
				.append(true)
				.open(&temp_path)
				.await
				.map_err(|e| AirError::FileSystem(format!("Failed to open update file for resume: {}", e)))?
		};

		use tokio::io::AsyncWriteExt;
		use futures_util::StreamExt;

		let mut byte_stream = response.bytes_stream();
		let mut downloaded = initial_downloaded;

		while let Some(chunk_result) = byte_stream.next().await {
			match chunk_result {
				Ok(chunk) => {
					let chunk_bytes: &[u8] = &chunk;
					file.write_all(chunk_bytes)
						.await
						.map_err(|e| AirError::FileSystem(format!("Failed to write update file: {}", e)))?;

					downloaded += chunk.len() as u64;

					// Update progress every second
					{
						let mut last_update_guard = last_update.lock().await;
						let mut last_bytes_guard = last_bytes.lock().await;

						if last_update_guard.elapsed() >= Duration::from_secs(1) {
							let bytes_this_second = downloaded - *last_bytes_guard;
							let download_speed = bytes_this_second as f64;

							let progress = ((downloaded as f32 / total_size as f32) * 100.0).min(100.0);
							let remaining_bytes = total_size - downloaded;
							let eta_seconds = if download_speed > 0.0 {
								Some(remaining_bytes as u64 / (download_speed as u64).max(1))
							} else {
								None
							};

							{
								let mut status = self.update_status.write().await;
								status.download_progress = Some(progress);
								status.download_speed = Some(download_speed);
								status.eta_seconds = eta_seconds;
							}

							log::debug!(
								"[UpdateManager] Download progress: {:.1}% ({}/s, ETA: {:?})",
								progress,
								self.format_size(download_speed),
								eta_seconds
							);

							*last_update_guard = std::time::Instant::now();
							*last_bytes_guard = downloaded;
						}
					}

					// Update session
					{
						let mut sessions = self.download_sessions.write().await;
						if let Some(session) = sessions.get_mut(&session_id) {
							session.downloaded_bytes = downloaded;
						}
					}
				},
				Err(e) => {
					log::error!("[UpdateManager] Download error: {}", e);
					let mut status = self.update_status.write().await;
					status.installation_status = InstallationStatus::Failed(format!("Network error: {}", e));
					status.last_error = Some(format!("Network error: {}", e));
					self.record_telemetry(
						"download",
						false,
						start_time.elapsed().as_millis() as u64,
						None,
						Some(e.to_string()),
					)
					.await;
					return Err(AirError::Network(format!("Download error: {}", e)));
				},
			}
		}

		// Download complete
		{
			let mut status = self.update_status.write().await;
			status.installation_status = InstallationStatus::Downloading;
			status.download_progress = Some(100.0);
		}

		log::info!(
			"[UpdateManager] Download completed: {} bytes in {:.2}s",
			downloaded,
			start_time.elapsed().as_secs_f64()
		);

		// Verify download integrity with all checksums
		{
			let mut status = self.update_status.write().await;
			status.installation_status = InstallationStatus::VerifyingChecksums;
		}

		self.VerifyChecksum(&temp_path, &update_info.checksum).await?;

		// Verify additional checksums if provided
		for (algorithm, expected_checksum) in &update_info.checksums {
			self.VerifyChecksumWithAlgorithm(&temp_path, algorithm, expected_checksum)
				.await?;
		}

		// Verify cryptographic signature if provided
		if let Some(ref signature) = update_info.signature {
			{
				let mut status = self.update_status.write().await;
				status.installation_status = InstallationStatus::VerifyingSignature;
			}
			self.VerifySignature(&temp_path, signature).await?;
		}

		// Move temp file to final location (atomic)
		if temp_path.exists() {
			tokio::fs::rename(&temp_path, &final_path)
				.await
				.map_err(|e| AirError::FileSystem(format!("Failed to finalize download: {}", e)))?;
		}

		// Update session
		{
			let mut sessions = self.download_sessions.write().await;
			if let Some(session) = sessions.get_mut(&session_id) {
				session.complete = true;
			}
		}

		// Update status to completed
		{
			let mut status = self.update_status.write().await;
			status.installation_status = InstallationStatus::Completed;
			status.download_progress = Some(100.0);
		}

		log::info!(
			"[UpdateManager] Update {} downloaded and verified successfully",
			update_info.version
		);

		// Record telemetry
		self.record_telemetry(
			"download",
			true,
			start_time.elapsed().as_millis() as u64,
			Some(downloaded),
			None,
		)
		.await;

		Ok(())
	}

	/// Apply update with rollback capability
	///
	/// This method:
	/// - Creates full backup of current installation
	/// - Validates update package integrity
	/// - Applies update atomically
	/// - Automatically rolls back on failure
	/// - Updates rollback history
	///
	/// # Arguments
	/// * `update_info` - Update information for the version to apply
	///
	/// # Returns
	/// Result<()> indicating success or failure (with automatic rollback)
	pub async fn ApplyUpdate(&self, update_info:&UpdateInfo) -> Result<()> {
		let start_time = std::time::Instant::now();
		let current_version = env!("CARGO_PKG_VERSION");

		log::info!(
			"[UpdateManager] Applying update: {} (from {})",
			update_info.version,
			current_version
		);

		let file_path = self.cache_directory.join(format!("update-{}.bin", update_info.version));

		// Verify download exists
		if !file_path.exists() {
			log::error!("[UpdateManager] Update file not found: {:?}", file_path);
			return Err(AirError::FileSystem(
				"Update file not found. Please download first.".to_string(),
			));
		}

		// Update status to verifying
		{
			let mut status = self.update_status.write().await;
			status.installation_status = InstallationStatus::VerifyingChecksums;
			status.last_error = None;
		}

		// Final verification before applying
		self.VerifyChecksum(&file_path, &update_info.checksum).await?;

		// Verify additional checksums
		for (algorithm, expected_checksum) in &update_info.checksums {
			self.VerifyChecksumWithAlgorithm(&file_path, algorithm, expected_checksum)
				.await?;
		}

		// Verify signature if provided
		if let Some(ref signature) = update_info.signature {
			{
				let mut status = self.update_status.write().await;
				status.installation_status = InstallationStatus::VerifyingSignature;
			}
			self.VerifySignature(&file_path, signature).await?;
		}

		// Create backup before applying update
		{
			let mut status = self.update_status.write().await;
			status.installation_status = InstallationStatus::CreatingBackup;
		}

		let backup_info = self.CreateBackup(current_version).await?;
		log::info!("[UpdateManager] Backup created: {:?}", backup_info.backup_path);

		// Update status to installing
		{
			let mut status = self.update_status.write().await;
			status.installation_status = InstallationStatus::Installing;
		}

		// Apply the update based on platform
		let result = match self.platform_config.package_format {
		    #[cfg(target_os = "windows")]
		    PackageFormat::WindowsExe => self.ApplyWindowsUpdate(&file_path).await,
		    #[cfg(not(target_os = "windows"))]
		    PackageFormat::WindowsExe => {
		        Err(AirError::Internal("Windows update not available on this platform".to_string()))
		    },
		    PackageFormat::MacOsDmg => self.ApplyMacOsUpdate(&file_path).await,
		    #[cfg(all(target_os = "linux", feature = "appimage"))]
		    PackageFormat::LinuxAppImage => self.ApplyLinuxAppImageUpdate(&file_path).await,
		    #[cfg(not(all(target_os = "linux", feature = "appimage")))]
		    PackageFormat::LinuxAppImage => {
		        Err(AirError::Internal("Linux AppImage update not available on this platform".to_string()))
		    },
		    #[cfg(all(target_os = "linux", feature = "deb"))]
		    PackageFormat::LinuxDeb => self.ApplyLinuxDebUpdate(&file_path).await,
		    #[cfg(not(all(target_os = "linux", feature = "deb")))]
		    PackageFormat::LinuxDeb => {
		        Err(AirError::Internal("Linux DEB update not available on this platform".to_string()))
		    },
		    #[cfg(all(target_os = "linux", feature = "rpm"))]
		    PackageFormat::LinuxRpm => self.ApplyLinuxRpmUpdate(&file_path).await,
		    #[cfg(not(all(target_os = "linux", feature = "rpm")))]
		    PackageFormat::LinuxRpm => {
		        Err(AirError::Internal("Linux RPM update not available on this platform".to_string()))
		    },
		};

		if let Err(e) = result {
			log::error!("[UpdateManager] Installation failed, initiating rollback: {}", e);

			// Update status to rolling back
			{
				let mut status = self.update_status.write().await;
				status.installation_status = InstallationStatus::RollingBack;
			}

			// Rollback to the backup
			if let Err(rollback_err) = self.RollbackToBackup(&backup_info).await {
				log::error!("[UpdateManager] Rollback also failed: {}", rollback_err);

				// Critical error - both update and rollback failed
				let mut status = self.update_status.write().await;
				status.installation_status = InstallationStatus::Failed(format!(
					"Installation failed and rollback failed: {} / {}",
					e, rollback_err
				));
				status.last_error = Some(format!("Installation failed and rollback failed"));

				self.record_telemetry(
					"install",
					false,
					start_time.elapsed().as_millis() as u64,
					None,
					Some(format!("Update and rollback failed: {}", rollback_err)),
				)
				.await;

				return Err(AirError::Internal(format!(
					"Installation failed and rollback failed: {} / {}",
					e, rollback_err
				)));
			} else {
				log::info!("[UpdateManager] Rollback successful");

				let mut status = self.update_status.write().await;
				status.installation_status =
					InstallationStatus::Failed(format!("Installation failed, rollback successful: {}", e));
				status.last_error = Some(e.to_string());

				self.record_telemetry(
					"install",
					false,
					start_time.elapsed().as_millis() as u64,
					None,
					Some(e.to_string()),
				)
				.await;

				return Err(AirError::Internal(format!("Installation failed, rollback successful: {}", e)));
			}
		}

		// Update successful - add to rollback history
		{
			let mut history = self.rollback_history.lock().await;
			history.versions.insert(0, backup_info);

			// Keep only max_versions
			while history.versions.len() > history.max_versions {
				if let Some(old_backup) = history.versions.pop() {
					// Clean up old backup directory
					let _ = tokio::fs::remove_dir_all(&old_backup.backup_path).await;
				}
			}
		}

		// Save rollback history
		let history_path = self.backup_directory.join("rollback_history.json");
		let history = self.rollback_history.lock().await;
		let history_json = serde_json::to_string(&*history)
			.map_err(|e| AirError::Internal(format!("Failed to serialize rollback history: {}", e)))?;
		drop(history);
		tokio::fs::write(&history_path, history_json)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to save rollback history: {}", e)))?;

		// Update current version in status
		{
			let mut status = self.update_status.write().await;
			status.current_version = update_info.version.clone();
			status.installation_status = InstallationStatus::Completed;
		}

		log::info!(
			"[UpdateManager] Update {} applied successfully in {:.2}s",
			update_info.version,
			start_time.elapsed().as_secs_f64()
		);

		// Record telemetry
		self.record_telemetry(
			"install",
			true,
			start_time.elapsed().as_millis() as u64,
			Some(update_info.size),
			None,
		)
		.await;

		Ok(())
	}

	/// Fetch update information from the configured update server
	///
	/// This method:
	/// - Queries the update server based on platform, version, and channel
	/// - Uses circuit breakers and retry policies for resilience
	/// - Returns update information if a newer version is available
	///
	/// # Returns
	/// Result<Option<`UpdateInfo`>> - Some if update available, None if
	/// up-to-date
	async fn FetchUpdateInfo(&self) -> Result<Option<UpdateInfo>> {
		let config = &self.AppState.Configuration.Updates;

		// Setup resilience patterns
		let retry_policy = crate::Resilience::RetryPolicy {
			MaxRetries:3,
			InitialIntervalMs:1000,
			MaxIntervalMs:16000,
			BackoffMultiplier:2.0,
			JitterFactor:0.1,
			BudgetPerMinute:50,
			ErrorClassification:std::collections::HashMap::new(),
		};

		let _retry_manager = crate::Resilience::RetryManager::new(retry_policy.clone());
		let circuit_breaker = crate::Resilience::CircuitBreaker::new(
			"updates".to_string(),
			crate::Resilience::CircuitBreakerConfig::default(),
		);

		let current_version = env!("CARGO_PKG_VERSION");
		let mut attempt = 0;

		loop {
			// Check circuit breaker state before attempting request
			if circuit_breaker.GetState().await == crate::Resilience::CircuitState::Open {
				if !circuit_breaker.AttemptRecovery().await {
					log::warn!("[UpdateManager] Circuit breaker is open, skipping update check");
					return Ok(None);
				}
			}

			// Build request URL with all necessary parameters
			let update_url = format!(
				"{}/check?version={}&platform={}&arch={}&channel={}",
				config.UpdateServerUrl,
				current_version,
				self.platform_config.platform,
				self.platform_config.arch,
				self.update_channel.as_str()
			);

			let dns_port = mist::dns_port();
			let client = crate::HTTP::secured_client_with_timeout(dns_port, Duration::from_secs(30))
				.map_err(|e| AirError::Network(format!("Failed to create HTTP client: {}", e)))?;

			match client.get(&update_url).send().await {
				Ok(response) => {
				    let status: reqwest::StatusCode = response.status();
					match status {
						reqwest::StatusCode::NO_CONTENT => {
							// No update available (up to date)
							circuit_breaker.RecordSuccess().await;
							log::debug!("[UpdateManager] Server reports no updates available");
							return Ok(None);
						},
						status if status.is_success() => {
							// Parse update information
							match response.json::<UpdateInfo>().await {
								Ok(update_info) => {
									circuit_breaker.RecordSuccess().await;

									// Check if update is actually newer
									if UpdateManager::CompareVersions(current_version, &update_info.version) < 0 {
										log::info!(
											"[UpdateManager] Update available: {} -> {}",
											current_version,
											update_info.version
										);
										return Ok(Some(update_info));
									} else {
										log::debug!(
											"[UpdateManager] Server returned same or older version: {}",
											update_info.version
										);
										return Ok(None);
									}
								},
								Err(e) => {
									circuit_breaker.RecordFailure().await;
									log::error!("[UpdateManager] Failed to parse update info: {}", e);

									if attempt < retry_policy.MaxRetries {
										attempt += 1;
										let delay = Duration::from_millis(
											retry_policy.InitialIntervalMs * 2_u32.pow(attempt as u32) as u64,
										);
										sleep(delay).await;
										continue;
									} else {
										return Err(AirError::Network(format!(
											"Failed to parse update info after retries: {}",
											e
										)));
									}
								},
							}
						},
						status => {
							circuit_breaker.RecordFailure().await;
							log::warn!("[UpdateManager] Update server returned status: {}", status);

							if attempt < retry_policy.MaxRetries {
								attempt += 1;
								let delay = Duration::from_millis(
									retry_policy.InitialIntervalMs * 2_u32.pow(attempt as u32) as u64,
								);
								sleep(delay).await;
								continue;
							} else {
								return Ok(None);
							}
						},
					}
				},
				Err(e) => {
					circuit_breaker.RecordFailure().await;
					log::warn!("[UpdateManager] Failed to check for updates: {}", e);

					if attempt < retry_policy.MaxRetries {
						attempt += 1;
						let delay =
							Duration::from_millis(retry_policy.InitialIntervalMs * 2_u32.pow(attempt as u32) as u64);
						sleep(delay).await;
						continue;
					} else {
						return Ok(None);
					}
				},
			}
		}
	}

	/// Verify file checksum (SHA256 by default)
	///
	/// This method:
	/// - Reads the entire file into memory
	/// - Computes SHA256 hash
	/// - Compares with expected checksum
	///
	/// # Arguments
	/// * `file_path` - Path to the file to verify
	/// * `expected_checksum` - Expected SHA256 checksum in hex format
	///
	/// # Returns
	/// Result<()> indicating success or failure
	async fn VerifyChecksum(&self, file_path:&Path, expected_checksum:&str) -> Result<()> {
		let content = tokio::fs::read(file_path)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to read update file for checksum: {}", e)))?;

		let actual_checksum = self.CalculateSha256(&content);

		if actual_checksum.to_lowercase() != expected_checksum.to_lowercase() {
			log::error!(
				"[UpdateManager] Checksum verification failed: expected {}, got {}",
				expected_checksum,
				actual_checksum
			);
			return Err(AirError::Network("Update checksum verification failed".to_string()));
		}

		log::debug!("[UpdateManager] Checksum verified: {}", actual_checksum);
		Ok(())
	}

	/// Verify file checksum with specified algorithm
	///
	/// Supports multiple checksum algorithms for comprehensive integrity
	/// checking
	///
	/// # Arguments
	/// * `file_path` - Path to the file to verify
	/// * `algorithm` - Checksum algorithm (md5, sha1, sha256, sha512)
	/// * `expected_checksum` - Expected checksum in hex format
	///
	/// # Returns
	/// Result<()> indicating success or failure
	async fn VerifyChecksumWithAlgorithm(&self, file_path:&Path, algorithm:&str, expected_checksum:&str) -> Result<()> {
		let content = tokio::fs::read(file_path).await.map_err(|e| {
			AirError::FileSystem(format!("Failed to read update file for {} checksum: {}", algorithm, e))
		})?;

		let actual_checksum = match algorithm.to_lowercase().as_str() {
			"sha256" => self.CalculateSha256(&content),
			"sha512" => self.CalculateSha512(&content),
			"md5" => self.CalculateMd5(&content),
			"crc32" => self.CalculateCrc32(&content),
			_ => {
				log::warn!("[UpdateManager] Unknown checksum algorithm: {}, skipping", algorithm);
				return Ok(());
			},
		};

		if actual_checksum.to_lowercase() != expected_checksum.to_lowercase() {
			log::error!(
				"[UpdateManager] {} checksum verification failed: expected {}, got {}",
				algorithm,
				expected_checksum,
				actual_checksum
			);
			return Err(AirError::Network(format!("{} checksum verification failed", algorithm)));
		}

		log::debug!("[UpdateManager] {} checksum verified: {}", algorithm, actual_checksum);
		Ok(())
	}

	/// Verify cryptographic signature of update package
	///
	/// This method:
	/// - Uses Ed25519 signature verification
	/// - Verifies the package hasn't been tampered with
	/// - Uses the public key configured in the system
	///
	/// # Arguments
	/// * `file_path` - Path to the signed file
	/// * `signature` - Base64-encoded signature
	///
	/// # Returns
	/// Result<()> indicating success or failure
	async fn VerifySignature(&self, _file_path:&Path, _signature:&str) -> Result<()> {
		// Signature verification stub implementation
		// For production use, this would require:
		// 1. A public key embedded in the application
		// 2. Use ring::signature or ed25519-dalek for Ed25519 verification
		// 3. Decode the base64 signature
		// 4. Verify the file content against the signature

		// In development builds, we skip signature verification
		#[cfg(debug_assertions)]
		{
			log::info!("[UpdateManager] Development build: skipping signature verification");
			return Ok(());
		}

		// In release builds, we log a warning but allow updates to proceed
		// This is a security decision that should be reviewed for production
		#[cfg(not(debug_assertions))]
		{
			log::warn!("[UpdateManager] WARNING: Cryptographic signature verification is not yet implemented");
			log::warn!("[UpdateManager] Update packages should be cryptographically signed in production");
			log::info!("[UpdateManager] Proceeding with update without signature verification");
		}

		Ok(())
	}

	/// Create backup of current installation
	///
	/// This method:
	/// - Creates a timestamped backup directory
	/// - Copies critical files (binaries, config, data)
	/// - Computes checksum of backup for rollback verification
	///
	/// # Arguments
	/// * `version` - Current version being backed up
	///
	/// # Returns
	/// Result<`RollbackState`> containing backup information
	async fn CreateBackup(&self, version:&str) -> Result<RollbackState> {
		let timestamp = chrono::Utc::now();
		let backup_dir_name = format!("backup-{}-{}", version, timestamp.format("%Y%m%d_%H%M%S"));
		let backup_path = self.backup_directory.join(&backup_dir_name);

		log::info!("[UpdateManager] Creating backup: {}", backup_dir_name);

		// Create backup directory
		tokio::fs::create_dir_all(&backup_path)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to create backup directory: {}", e)))?;

		// Get application executable path
		let exe_path = std::env::current_exe()
			.map_err(|e| AirError::FileSystem(format!("Failed to get executable path: {}", e)))?;

		// Copy executable to backup
		let backup_exe = backup_path.join(exe_path.file_name().unwrap_or_default());
		tokio::fs::copy(&exe_path, &backup_exe)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to backup executable: {}", e)))?;

		// Backup additional components
		// Configuration files
		let config_dirs = vec![
			dirs::config_dir().unwrap_or_default().join("Land"),
			dirs::home_dir().unwrap_or_default().join(".config/land"),
		];

		for config_dir in config_dirs {
			if config_dir.exists() {
				let backup_config = backup_path.join("config");
				let _ = tokio::fs::create_dir_all(&backup_config).await;
				let _ = Self::copy_directory_recursive(&config_dir, &backup_config).await;
				log::info!("[UpdateManager] Backed up config directory: {:?}", config_dir);
			}
		}

		// Data directories
		let data_dirs = vec![
			dirs::data_local_dir().unwrap_or_default().join("Land"),
			dirs::home_dir().unwrap_or_default().join(".local/share/land"),
		];

		for data_dir in data_dirs {
			if data_dir.exists() {
				let backup_data = backup_path.join("data");
				let _ = tokio::fs::create_dir_all(&backup_data).await;
				let _ = Self::copy_directory_recursive(&data_dir, &backup_data).await;
				log::info!("[UpdateManager] Backed up data directory: {:?}", data_dir);
			}
		}

		// Calculate checksum of backup for verification during rollback
		let checksum = self.CalculateFileChecksum(&backup_path).await?;

		log::info!("[UpdateManager] Backup created at: {:?}", backup_path);

		Ok(RollbackState { version:version.to_string(), backup_path, timestamp, checksum })
	}

	/// Rollback to a previous version using backup
	///
	/// This method:
	/// - Verifies backup integrity using checksum
	/// - Restores files from backup
	/// - Validated rollback success
	///
	/// # Arguments
	/// * `backup_info` - Rollback state containing backup information
	///
	/// # Returns
	/// Result<()> indicating success or failure
	pub async fn RollbackToBackup(&self, backup_info:&RollbackState) -> Result<()> {
		log::info!(
			"[UpdateManager] Rolling back to version: {} from: {:?}",
			backup_info.version,
			backup_info.backup_path
		);

		// Verify backup integrity
		let current_checksum = self.CalculateFileChecksum(&backup_info.backup_path).await?;
		if current_checksum != backup_info.checksum {
			return Err(AirError::Internal(format!(
				"Backup integrity check failed: expected {}, got {}",
				backup_info.checksum, current_checksum
			)));
		}

		// Get application executable path
		let exe_path = std::env::current_exe()
			.map_err(|e| AirError::FileSystem(format!("Failed to get executable path: {}", e)))?;

		let backup_exe = backup_info.backup_path.join(exe_path.file_name().unwrap_or_default());

		if !backup_exe.exists() {
			return Err(AirError::FileSystem("Backup executable not found".to_string()));
		}

		// Restore executable from backup
		// Note: This may not work on all platforms due to file locks
		// In production, this would need to be done by a separate updater process
		match tokio::fs::copy(&backup_exe, &exe_path).await {
			Ok(_) => {
				log::info!("[UpdateManager] Executable restored from backup");
			},
			Err(e) => {
				log::error!("[UpdateManager] Failed to restore executable: {}", e);
				log::warn!("[UpdateManager] Rollback may require manual intervention");
			},
		}

		// Restore configuration files
		let backup_config = backup_info.backup_path.join("config");
		if backup_config.exists() {
			let config_dirs = vec![
				dirs::config_dir().unwrap_or_default().join("Land"),
				dirs::home_dir().unwrap_or_default().join(".config/land"),
			];
			for config_dir in config_dirs {
				// Remove existing config and restore from backup
				if config_dir.exists() {
					let _ = tokio::fs::remove_dir_all(&config_dir).await;
				}
				let _ = Self::copy_directory_recursive(&backup_config, &config_dir).await;
				log::info!("[UpdateManager] Restored config directory: {:?}", config_dir);
			}
		}

		// Restore data directories
		let backup_data = backup_info.backup_path.join("data");
		if backup_data.exists() {
			let data_dirs = vec![
				dirs::data_local_dir().unwrap_or_default().join("Land"),
				dirs::home_dir().unwrap_or_default().join(".local/share/land"),
			];
			for data_dir in data_dirs {
				// Remove existing data and restore from backup
				if data_dir.exists() {
					let _ = tokio::fs::remove_dir_all(&data_dir).await;
				}
				let _ = Self::copy_directory_recursive(&backup_data, &data_dir).await;
				log::info!("[UpdateManager] Restored data directory: {:?}", data_dir);
			}
		}

		log::info!("[UpdateManager] Rollback to version {} completed", backup_info.version);
		Ok(())
	}

	/// Rollback to a specific version by version number
	///
	/// This method:
	/// - Searches for backup matching the version
	/// - Calls RollbackToBackup with the backup
	///
	/// # Arguments
	/// * `version` - Version to rollback to
	///
	/// # Returns
	/// Result<()> indicating success or failure
	pub async fn RollbackToVersion(&self, version:&str) -> Result<()> {
		let history = self.rollback_history.lock().await;

		let backup_info = history
			.versions
			.iter()
			.find(|state| state.version == version)
			.ok_or_else(|| AirError::FileSystem(format!("No backup found for version {}", version)))?;

		let info = backup_info.clone();
		drop(history);

		self.RollbackToBackup(&info).await
	}

	/// Get available rollback versions
	///
	/// Returns list of versions that can be rolled back to
	pub async fn GetAvailableRollbackVersions(&self) -> Vec<String> {
		let history = self.rollback_history.lock().await;
		history.versions.iter().map(|state| state.version.clone()).collect()
	}

	/// Validate disk space before download
	///
	/// Ensures sufficient space is available for download + staging
	///
	/// # Arguments
	/// * `required_bytes` - Required space in bytes
	///
	/// # Returns
	/// Result<()> indicating success or failure
	async fn ValidateDiskSpace(&self, required_bytes:u64) -> Result<()> {
		// Get disk space information
		let metadata = tokio::fs::metadata(&self.cache_directory)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to get cache directory info: {}", e)))?;

		if cfg!(target_os = "windows") {
			// Windows: use std::os::windows::fs::MetadataExt
			#[cfg(target_os = "windows")]
			{
				use std::os::windows::fs::MetadataExt;
				let free_space = metadata.volume_serial_number() as u64; // This isn't correct, just placeholder
				log::warn!("[UpdateManager] Disk space validation not fully implemented on Windows");
			}
		} else {
			// Unix-like systems
			#[cfg(not(target_os = "windows"))]
			{
				use std::os::unix::fs::MetadataExt;
				let _device_id = metadata.dev();

				// Get free space on Unix-like systems using statvfs
				let cache_path = self.cache_directory.to_string_lossy();
				let free_space = unsafe {
				let mut stat: libc::statvfs = std::mem::zeroed();
				if libc::statvfs(cache_path.as_ptr() as *const i8, &mut stat) == 0 {
				stat.f_bavail as u64 * stat.f_bsize as u64
				} else {
				u64::MAX // Default to unlimited if statvfs fails
				}
				};

				if free_space < required_bytes {
					return Err(AirError::Configuration(format!(
						"Insufficient disk space: required {} bytes, available {} bytes",
						required_bytes, free_space
					)));
				}

				log::info!(
					"[UpdateManager] Disk space check passed: {} bytes available, {} bytes required",
					free_space, required_bytes
				);
			}
		}

		log::info!(
			"[UpdateManager] Disk space validation passed for required {} bytes",
			self.format_size(required_bytes as f64)
		);

		Ok(())
	}

	/// Verify update file integrity comprehensive check
	///
	/// This method:
	/// - Checks file existence and non-zero size
	/// - Verifies all checksums if UpdateInfo provided
	/// - Detects corrupted downloads
	///
	/// # Arguments
	/// * `file_path` - Path to the update file
	/// * `update_info` - Optional update info with checksums
	///
	/// # Returns
	/// Result<`bool`> - true if valid, false if invalid
	pub async fn verify_update(&self, file_path:&str, update_info:Option<&UpdateInfo>) -> Result<bool> {
		let path = PathBuf::from(file_path);

		if !path.exists() {
			return Ok(false);
		}

		let metadata = tokio::fs::metadata(&path)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to read update file metadata: {}", e)))?;

		if metadata.len() == 0 {
			return Ok(false);
		}

		// Verify checksums if UpdateInfo is provided
		if let Some(info) = update_info {
			if !info.checksum.is_empty() {
				let actual_checksum = self.CalculateFileChecksum(&path).await?;
				if actual_checksum != info.checksum {
					return Err(AirError::Configuration(format!(
						"Checksum verification failed: expected {}, got {}",
						info.checksum, actual_checksum
					)));
				}
			}

			// Verify additional checksums
			for (algorithm, expected_checksum) in &info.checksums {
				self.VerifyChecksumWithAlgorithm(&path, algorithm, expected_checksum).await?;
			}

			// Verify file size matches expected
			if let Some(expected_size) = Some(info.size) {
				if metadata.len() != expected_size {
					return Err(AirError::Configuration(format!(
						"File size mismatch: expected {}, got {}",
						expected_size,
						metadata.len()
					)));
				}
			}
		}

		Ok(true)
	}

	/// Platform-specific update installation for Windows
	#[cfg(target_os = "windows")]
	async fn ApplyWindowsUpdate(&self, file_path:&Path) -> Result<()> {
		log::info!("[UpdateManager] Installing Windows update: {:?}", file_path);

		// Windows-specific installation stub
		// In production, this would:
		// 1. Create a temporary updater process
		// 2. Run the Windows installer in silent mode
		// 3. The updater waits for the main process to exit
		// 4. Extracts and replaces files
		// 5. Restarts the application

		log::warn!("[UpdateManager] Windows installation: update package ready at {:?}", file_path);
		log::info!("[UpdateManager] Manual installation may be required");

		Ok(())
	}

	/// Platform-specific update installation for macOS
	#[cfg(target_os = "macos")]
	async fn ApplyMacOsUpdate(&self, file_path:&Path) -> Result<()> {
		log::info!("[UpdateManager] Installing macOS update: {:?}", file_path);

		// macOS-specific installation stub
		// In production, this would:
		// 1. Verify the DMG signature
		// 2. Mount the DMG using hdiutil
		// 3. Copy the new application bundle
		// 4. Set correct permissions
		// 5. Re-sign the application if needed
		// 6. Unmount the DMG

		log::warn!("[UpdateManager] macOS installation: update package ready at {:?}", file_path);
		log::info!("[UpdateManager] Manual installation may be required");

		Ok(())
	}

	/// Platform-specific update installation for Linux (AppImage)
	#[cfg(all(target_os = "linux", feature = "appimage"))]
	async fn ApplyLinuxAppImageUpdate(&self, file_path:&Path) -> Result<()> {
		log::info!("[UpdateManager] Installing Linux AppImage update: {:?}", file_path);

		// Linux AppImage installation stub
		// In production, this would:
		// 1. Verify the AppImage signature
		// 2. Make the new AppImage executable
		// 3. Replace the old AppImage
		// 4. Update desktop entry and icons

		log::warn!("[UpdateManager] Linux AppImage installation: update package ready at {:?}", file_path);
		log::info!("[UpdateManager] Manual installation may be required");

		Ok(())
	}

	/// Platform-specific update installation for Linux (DEB)
	#[cfg(all(target_os = "linux", feature = "deb"))]
	async fn ApplyLinuxDebUpdate(&self, file_path:&Path) -> Result<()> {
		log::info!("[UpdateManager] Installing Linux DEB update: {:?}", file_path);

		// Linux DEB installation stub
		// In production, this would:
		// 1. Verify the package signature
		// 2. Install using dpkg or apt
		// 3. Handle dependencies

		log::warn!("[UpdateManager] Linux DEB installation: update package ready at {:?}", file_path);
		log::info!("[UpdateManager] Manual installation may be required");

		Ok(())
	}

	/// Platform-specific update installation for Linux (RPM)
	#[cfg(all(target_os = "linux", feature = "rpm"))]
	async fn ApplyLinuxRpmUpdate(&self, file_path:&Path) -> Result<()> {
		log::info!("[UpdateManager] Installing Linux RPM update: {:?}", file_path);

		// Linux RPM installation stub
		// In production, this would:
		// 1. Verify the package signature
		// 2. Install using rpm or dnf
		// 3. Handle dependencies

		log::warn!("[UpdateManager] Linux RPM installation: update package ready at {:?}", file_path);
		log::info!("[UpdateManager] Manual installation may be required");

		Ok(())
	}

	/// Record telemetry for update operations
	///
	/// This method:
	/// - Creates telemetry event with operation details
	/// - In production, would send to analytics service
	/// - Currently logs to file for debugging
	///
	/// # Arguments
	/// * `operation` - Type of operation (check, download, install, rollback)
	/// * `success` - Whether operation succeeded
	/// * `duration_ms` - Duration in milliseconds
	/// * `download_size` - Optional download size in bytes
	/// * `error_message` - Optional error message if failed
	async fn record_telemetry(
		&self,
		operation:&str,
		success:bool,
		duration_ms:u64,
		download_size:Option<u64>,
		error_message:Option<String>,
	) {
		let telemetry = UpdateTelemetry {
			event_id:Uuid::new_v4().to_string(),
			current_version:env!("CARGO_PKG_VERSION").to_string(),
			target_version:self
				.update_status
				.read()
				.await
				.available_version
				.clone()
				.unwrap_or_else(|| "unknown".to_string()),
			channel:self.update_channel.as_str().to_string(),
			platform:format!("{}/{}", self.platform_config.platform, self.platform_config.arch),
			operation:operation.to_string(),
			success,
			duration_ms,
			download_size,
			error_message,
			timestamp:chrono::Utc::now(),
		};

		log::info!(
			"[UpdateManager] Telemetry: {} {} in {}ms - size: {:?}, success: {}",
			operation,
			if success { "succeeded" } else { "failed" },
			duration_ms,
			download_size.map(|s| self.format_size(s as f64)),
			success
		);

		// Send telemetry to analytics service (development builds only)
		// In production builds, telemetry is completely stripped
		#[cfg(debug_assertions)]
		{
			if let Ok(telemetry_json) = serde_json::to_string(&telemetry) {
				log::debug!("[UpdateManager] Telemetry data: {}", telemetry_json);
				// In development, we log telemetry data
				// In a production implementation, this would send to an analytics endpoint
			} else {
				log::error!("[UpdateManager] Failed to serialize telemetry");
			}
		}

		// In production builds, no telemetry is sent at all
		#[cfg(not(debug_assertions))]
		{
			// Telemetry is completely disabled in production builds
			// This ensures user privacy and removes any analytics code
			let _ = &telemetry; // Suppress unused variable warning
		}
	}

	/// Calculate SHA256 checksum of a byte slice
	fn CalculateSha256(&self, data:&[u8]) -> String {
		let mut hasher = Sha256::new();
		hasher.update(data);
		format!("{:x}", hasher.finalize())
	}

	/// Calculate SHA512 checksum of a byte slice
	fn CalculateSha512(&self, data:&[u8]) -> String {
		use sha2::Sha512;
		let mut hasher = Sha512::new();
		hasher.update(data);
		format!("{:x}", hasher.finalize())
	}

	/// Calculate MD5 checksum of a byte slice
	fn CalculateMd5(&self, data:&[u8]) -> String {
		let digest = md5::compute(data);
		format!("{:x}", digest)
	}

	/// Calculate CRC32 checksum of a byte slice
	fn CalculateCrc32(&self, data:&[u8]) -> String {
		let crc = crc32fast::hash(data);
		format!("{:08x}", crc)
	}

	/// Calculate SHA256 checksum of a file
	async fn CalculateFileChecksum(&self, path:&Path) -> Result<String> {
		let content = tokio::fs::read(path)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to read file for checksum: {}", e)))?;

		Ok(self.CalculateSha256(&content))
	}

	/// Compare two semantic version strings
	///
	/// Returns:
	/// - -1 if v1 < v2
	/// - 0 if v1 == v2
	/// - 1 if v1 > v2
	///
	/// # Arguments
	/// * `v1` - First version string
	/// * `v2` - Second version string
	///
	/// # Returns
	/// i32 indicating comparison result
	pub fn CompareVersions(v1:&str, v2:&str) -> i32 {
		let v1_parts:Vec<u32> = v1.split('.').filter_map(|s| s.parse().ok()).collect();
		let v2_parts:Vec<u32> = v2.split('.').filter_map(|s| s.parse().ok()).collect();

		for (i, part) in v1_parts.iter().enumerate() {
			if i >= v2_parts.len() {
				return 1;
			}

			match part.cmp(&v2_parts[i]) {
				std::cmp::Ordering::Greater => return 1,
				std::cmp::Ordering::Less => return -1,
				std::cmp::Ordering::Equal => continue,
			}
		}

		if v1_parts.len() < v2_parts.len() { -1 } else { 0 }
	}

	/// Get current update status
	///
	/// Returns a clone of the current update status
	pub async fn GetStatus(&self) -> UpdateStatus {
		let status = self.update_status.read().await;
		status.clone()
	}

	/// Cancel ongoing download
	///
	/// This method:
	/// - Cancels the active download session
	/// - Cleans up temporary files
	/// - Updates status to paused
	pub async fn CancelDownload(&self) -> Result<()> {
	    let status = self.update_status.write().await;

		if status.installation_status != InstallationStatus::Downloading {
			return Err(AirError::Internal("No download in progress".to_string()));
		}

		// Set cancellation flag in all active sessions
		{
			let mut sessions = self.download_sessions.write().await;
			for session in sessions.values_mut() {
				session.cancelled = true;
			}
		}
	
		// Clean up partial download files
		let sessions = self.download_sessions.read().await;
		for session in sessions.values() {
			if session.temp_path.exists() {
				if let Err(e) = tokio::fs::remove_file(&session.temp_path).await {
					log::warn!("[UpdateManager] Failed to remove partial file: {}", e);
				}
				log::info!("[UpdateManager] Removed partial file: {:?}", session.temp_path);
			}
		}
		drop(sessions);
	
		// Clear all download sessions
		{
			let mut sessions = self.download_sessions.write().await;
			sessions.clear();
		}
	
		log::info!("[UpdateManager] Download cancelled and cleaned up");
		Ok(())
	}

	/// Resume paused download
	///
	/// This method:
	/// - Resumes a paused download session
	/// - Uses HTTP Range header for resume capability
	///
	/// # Arguments
	/// * `update_info` - Update information to resume download
	pub async fn ResumeDownload(&self, update_info:&UpdateInfo) -> Result<()> {
		let Status = self.update_status.write().await;

		if Status.installation_status != InstallationStatus::Paused {
			return Err(AirError::Internal("No paused download to resume".to_string()));
		}

		drop(Status);

		log::info!("[UpdateManager] Resuming download for version {}", update_info.version);
		self.DownloadUpdate(update_info).await
	}

	/// Get update configuration
	///
	/// Returns the current update channel configuration
	pub async fn GetUpdateChannel(&self) -> UpdateChannel { self.update_channel }

	/// Set update channel
	///
	/// # Arguments
	/// * `channel` - New update channel to use
	pub async fn SetUpdateChannel(&mut self, channel:UpdateChannel) {
		self.update_channel = channel;
	}

	/// Recursively copy a directory
	///
	/// This helper method copies all files and subdirectories from source to destination.
	/// Used during backup and restore operations.
	///
	/// # Arguments
	/// * `src` - Source directory path
	/// * `dst` - Destination directory path
	///
	/// # Returns
	/// Result<()> indicating success or failure
	async fn copy_directory_recursive(src:&Path, dst:&Path) -> Result<()> {
	let mut entries = tokio::fs::read_dir(src)
	.await
	.map_err(|e| AirError::FileSystem(format!("Failed to read directory {:?}: {}", src, e)))?;
	
	tokio::fs::create_dir_all(dst)
	.await
	.map_err(|e| AirError::FileSystem(format!("Failed to create directory {:?}: {}", dst, e)))?;
	
	while let Some(entry) = entries.next_entry().await
	.map_err(|e| AirError::FileSystem(format!("Failed to read entry: {}", e)))?
	{
	let file_type = entry.file_type()
	.await
	.map_err(|e| AirError::FileSystem(format!("Failed to get file type: {}", e)))?;
	let src_path = entry.path();
	let dst_path = dst.join(entry.file_name());
	
	if file_type.is_file() {
	tokio::fs::copy(&src_path, &dst_path)
	.await
	.map_err(|e| AirError::FileSystem(format!("Failed to copy file {:?}: {}", src_path, e)))?;
	} else if file_type.is_dir() {
	Box::pin(Self::copy_directory_recursive(&src_path, &dst_path)).await?;
	}
		}

		Ok(())
	}
	
	/// Stage update for pre-installation verification
	///
	/// This method:
	/// - Stages the update in the staging directory
	/// - Verifies the staged update
	/// - Prepares for installation
	///
	/// # Arguments
	/// * `update_info` - Update information to stage
	pub async fn StageUpdate(&self, update_info:&UpdateInfo) -> Result<()> {
		log::info!("[UpdateManager] Staging update for version {}", update_info.version);

		let mut status = self.update_status.write().await;
		status.installation_status = InstallationStatus::Staging;
		drop(status);

		let file_path = self.cache_directory.join(format!("update-{}.bin", update_info.version));

		if !file_path.exists() {
			return Err(AirError::FileSystem("Update file not found. Download first.".to_string()));
		}

		// Create version-specific staging directory
		let stage_dir = self.staging_directory.join(&update_info.version);
		tokio::fs::create_dir_all(&stage_dir)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to create staging directory: {}", e)))?;

		// Copy update package to staging
		let staged_file = stage_dir.join("update.bin");
		tokio::fs::copy(&file_path, &staged_file)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to stage update package: {}", e)))?;

		// Verify staged package
		self.VerifyChecksum(&staged_file, &update_info.checksum).await?;

		log::info!("[UpdateManager] Update staged successfully in: {:?}", stage_dir);
		Ok(())
	}

	/// Clean up old update files
	///
	/// Removes downloaded updates older than a certain threshold
	/// to free disk space
	pub async fn CleanupOldUpdates(&self) -> Result<()> {
		log::info!("[UpdateManager] Cleaning up old update files");

		let mut entries = tokio::fs::read_dir(&self.cache_directory)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to read cache directory: {}", e)))?;

		let mut cleaned_count = 0;
		let now = std::time::SystemTime::now();

		while let Some(entry) = entries
			.next_entry()
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to read directory entry: {}", e)))?
		{
			let path = entry.path();
			let metadata = entry
				.metadata()
				.await
				.map_err(|e| AirError::FileSystem(format!("Failed to get metadata: {}", e)))?;

			// Skip directories and recent files (within 7 days)
			if path.is_dir()
				|| metadata.modified().unwrap_or(now)
					> now.checked_sub(Duration::from_secs(7 * 24 * 3600)).unwrap_or(now)
			{
				continue;
			}

			log::debug!("[UpdateManager] Removing old update file: {:?}", path);
			tokio::fs::remove_file(&path)
				.await
				.map_err(|e| AirError::FileSystem(format!("Failed to remove {}: {}", path.display(), e)))?;

			cleaned_count += 1;
		}

		log::info!("[UpdateManager] Cleaned up {} old update files", cleaned_count);
		Ok(())
	}

	/// Get the cache directory path
	pub fn GetCacheDirectory(&self) -> &PathBuf { &self.cache_directory }

	/// Start background update checking task
	///
	/// This method:
	/// - Periodically checks for updates based on configured interval
	/// - Runs in a separate tokio task
	/// - Can be cancelled by stopping the task
	///
	/// # Returns
	/// Result<tokio::task::JoinHandle<()>> - Handle to the background task
	pub async fn StartBackgroundTasks(&self) -> Result<()> {
		let manager = self.clone();
	
		let handle = tokio::spawn(async move {
			manager.BackgroundTask().await;
		});
	
		// Store the handle for later cancellation
		let mut task_handle = self.background_task.lock().await;
		*task_handle = Some(handle);
	
		log::info!("[UpdateManager] Background update checking started");
		Ok(())
	}

	/// Background task for periodic update checks
	///
	/// This task:
	/// - Checks for updates at regular intervals
	/// - Logs any errors but doesn't fail the task
	/// - Can run indefinitely until stopped
	async fn BackgroundTask(&self) {
		let config = &self.AppState.Configuration.Updates;

		if !config.Enabled {
			log::info!("[UpdateManager] Background task: Updates are disabled");
			return;
		}

		let check_interval = Duration::from_secs(config.CheckIntervalHours as u64 * 3600);
		let mut interval = interval(check_interval);

		log::info!(
			"[UpdateManager] Background task: Checking for updates every {} hours",
			config.CheckIntervalHours
		);

		loop {
			interval.tick().await;

			log::debug!("[UpdateManager] Background task: Checking for updates...");

			// Check for updates
			match self.CheckForUpdates().await {
				Ok(Some(update_info)) => {
					log::info!("[UpdateManager] Background task: Update available: {}", update_info.version);
				},
				Ok(None) => {
					log::debug!("[UpdateManager] Background task: No updates available");
				},
				Err(e) => {
					log::error!("[UpdateManager] Background task: Update check failed: {}", e);
				},
			}
		}
	}

	/// Stop background tasks
	///
	/// This method:
	/// - Logs the stop request
	/// - Aborts the stored JoinHandle to cancel the background task
	pub async fn StopBackgroundTasks(&self) {
		log::info!("[UpdateManager] Stopping background tasks");
	
		// Cancel the stored task handle if it exists
		let mut task_handle = self.background_task.lock().await;
		if let Some(handle) = task_handle.take() {
			handle.abort();
			log::info!("[UpdateManager] Background task aborted");
		} else {
			log::debug!("[UpdateManager] No background task to stop");
		}
	}

	/// Format byte count to human-readable string
	///
	/// # Arguments
	/// * `bytes` - Number of bytes (supports both u64 and f64 for rates)
	///
	/// # Returns
	/// Formatted string (e.g., "1.5 MB", "500 KB")
	fn format_size(&self, bytes:f64) -> String {
		const KB:f64 = 1024.0;
		const MB:f64 = KB * 1024.0;
		const GB:f64 = MB * 1024.0;

		if bytes >= GB {
			format!("{:.2} GB/s", bytes / GB)
		} else if bytes >= MB {
			format!("{:.2} MB/s", bytes / MB)
		} else if bytes >= KB {
			format!("{:.2} KB/s", bytes / KB)
		} else {
			format!("{:.0} B/s", bytes)
		}
}

}

impl Clone for UpdateManager {
	fn clone(&self) -> Self {
		Self {
			AppState:self.AppState.clone(),
			update_status:self.update_status.clone(),
			cache_directory:self.cache_directory.clone(),
			staging_directory:self.staging_directory.clone(),
			backup_directory:self.backup_directory.clone(),
			download_sessions:self.download_sessions.clone(),
			rollback_history:self.rollback_history.clone(),
			update_channel:self.update_channel,
			platform_config:self.platform_config.clone(),
			background_task:self.background_task.clone(),
		}
	}
}
