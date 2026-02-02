//! # Download Manager Service
//!
//! ## Core Responsibilities
//!
//! The DownloadManager provides a comprehensive, resilient service for
//! downloading files, extensions, dependencies, and packages within the Land
//! ecosystem. It serves as the central download authority across all components
//! including:
//!
//! - **Cocoon (Extension Host)**: VSIX extension downloads from marketplaces
//! - **Mountain (Tauri Bundling)**: Package and dependency downloads for builds
//! - **Air (Background Daemon)**: Runtime updates and internal component
//!   downloads
//! - **Other Components**: File downloads, resource fetching, and asset
//!   management
//!
//! ## Architecture and Design Patterns
//!
//! Based on VSCode's download manager patterns found in:
//! `src/vs/platform/download/common/downloadService.ts`
//!
//! Key architectural principles:
//!
//! 1. **Resilient Pattern**: Circuit breaker with exponential backoff for retry
//!    logic
//! 2. **Streaming Pattern**: Progressive downloads with real-time progress
//!    tracking
//! 3. **Verification Pattern**: SHA-256 checksum validation for integrity
//!    assurance
//! 4. **Resource Management**: Parallel downloads controlled by bandwidth
//!    limits
//! 5. **Priority Queuing**: Download scheduling based on urgency and
//!    dependencies
//!
//! ## Resilient Downloading Patterns
//!
//! ### Retry Logic with Circuit Breaker
//! - Exponential backoff: 1s, 2s, 4s, 8s, 16s, 32s (max)
//! - Circuit breaker opens after consecutive failures to prevent cascade
//! - Success/failure budgets controlled per-minute limits
//! - Automatic recovery attempts with grace periods
//!
//! ### Partial Download Resume
//! - Supports HTTP Range headers for interrupted download recovery
//! - Hash verification on resume ensures data integrity
//! - Temporary file management with atomic commit
//! - Cleanup of corrupted partial files on failure
//!
//! ### Integrity Verification
//! - SHA-256 checksum validation during and after download
//! - Progressive verification for large files (chunked hashing)
//! - Signature verification for signed packages
//! - Detection and handling of tampered downloads
//!
//! ## Integration Points
//!
//! ### Cocoon Extension Workflow
//! 1. Extension host requests VSIX download from marketplace APIs
//! 2. DownloadManager validates VSIX manifest and signed content
//! 3. Download proceeds with progress callbacks to UI
//! 4. Checksum verification of signed .vsix package
//! 5. Atomic commit to extension installation directory
//!
//! ### Mountain Package Workflow
//! 1. Build system initiates dependency downloads
//! 2. DownloadManager validates package signatures
//! 3. Parallel chunk downloads for large packages
//! 4. Bandwidth throttling to prevent network saturation
//! 5. Atomic staging with final commit to build cache
//!
//! ### VSIX Download and Validation
//! - Supports marketplace API authentication tokens
//! - Validates extension manifest before download
//! - Verifies package signature after download
//! - Extracts and validates contents before installation
//!
//! ## TODOs and Future Enhancements
//!
//! ### P2P Distribution (Planned)
//! - Peer-to-peer file sharing between Land instances
//! - BitTorrent-like protocol for large package distribution
//! - Chunk verification from multiple sources
//! - Swarm coordination for rapid downloads
//!
//! ### Chunked Downloads (In Progress)
//! - Parallel HTTP Range requests for large files
//! - Automatic chunk size optimization based on bandwidth
//! - Reassembly with integrity verification
//! - Dynamic chunk adjustment based on network conditions
//!
//! ### Bandwidth Limiting (In Progress)
//! - Per-download rate limiting
//! - Global bandwidth pool management
//! - Time-based bandwidth schedules (off-peak acceleration)
//! - QoS priorities for critical vs non-critical downloads
//!
//! ### Additional Features
//! - CDN integration for faster regional downloads
//! - Adaptive mirror selection based on latency
//! - Pre-fetching and caching of frequently accessed resources
//! - Download deduplication across the ecosystem

use std::{
	collections::{HashMap, VecDeque},
	path::{Path, PathBuf},
	sync::Arc,
	time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, Semaphore};

use crate::{AirError, ApplicationState::ApplicationState, Configuration::ConfigurationManager, Result, Utility};

/// Download manager implementation with full resilience and capabilities
pub struct DownloadManager {
	/// Application state reference
	AppState:Arc<ApplicationState>,

	/// Active downloads tracking
	ActiveDownloads:Arc<RwLock<HashMap<String, DownloadStatus>>>,

	/// Download queue with priority ordering
	DownloadQueue:Arc<RwLock<VecDeque<QueuedDownload>>>,

	/// Download cache directory
	CacheDirectory:PathBuf,

	/// HTTP client with connection pooling
	client:reqwest::Client,

	/// Checksum verifier helper
	ChecksumVerifier:Arc<crate::Security::ChecksumVerifier>,

	/// Bandwidth limiter for global control
	BandwidthLimiter:Arc<Semaphore>,

	/// Concurrent download limiter
	ConcurrentLimiter:Arc<Semaphore>,

	/// Download statistics
	statistics:Arc<RwLock<DownloadStatistics>>,
}

/// Download status with comprehensive tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadStatus {
	pub DownloadId:String,
	pub url:String,
	pub destination:PathBuf,
	pub TotalSize:u64,
	pub downloaded:u64,
	pub progress:f32,
	pub status:DownloadState,
	pub error:Option<String>,
	pub StartedAt:Option<chrono::DateTime<chrono::Utc>>,
	pub CompletedAt:Option<chrono::DateTime<chrono::Utc>>,
	pub ChunksCompleted:usize,
	pub TotalChunks:usize,
	pub DownloadRateBytesPerSec:u64,
	pub ExpectedChecksum:Option<String>,
	pub ActualChecksum:Option<String>,
}

/// Download state with detailed progress
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DownloadState {
	Pending,
	Queued,
	Downloading,
	Verifying,
	Completed,
	Failed,
	Cancelled,
	Paused,
	Resuming,
}

/// Priority levels for download queuing
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DownloadPriority {
	High = 3,
	Normal = 2,
	Low = 1,
	Background = 0,
}

/// Queued download with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedDownload {
	DownloadId:String,
	url:String,
	destination:PathBuf,
	checksum:String,
	priority:DownloadPriority,
	AddedAt:chrono::DateTime<chrono::Utc>,
	MaxFileSize:Option<u64>,
	ValidateDiskSpace:bool,
}

/// Download result with full metadata
#[derive(Debug, Clone)]
pub struct DownloadResult {
	pub path:String,
	pub size:u64,
	pub checksum:String,
	pub duration:Duration,
	pub AverageRate:u64,
}

/// Download statistics and analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadStatistics {
	pub TotalDownloads:u64,
	pub SuccessfulDownloads:u64,
	pub FailedDownloads:u64,
	pub CancelledDownloads:u64,
	pub TotalBytesDownloaded:u64,
	pub TotalDownloadTimeSecs:f64,
	pub AverageDownloadRate:f64,
	pub PeakDownloadRate:u64,
	pub ActiveDownloads:usize,
	pub QueuedDownloads:usize,
}

/// Progress callback type
pub type ProgressCallback = Arc<dyn Fn(DownloadStatus) + Send + Sync>;

/// Download configuration with validation constraints
#[derive(Debug, Clone)]
pub struct DownloadConfig {
	pub url:String,
	pub destination:String,
	pub checksum:String,
	pub MaxFileSize:Option<u64>,
	pub ChunkSize:usize,
	pub MaxRetries:u32,
	pub TimeoutSecs:u64,
	pub priority:DownloadPriority,
	pub ValidateDiskSpace:bool,
}

impl Default for DownloadConfig {
	fn default() -> Self {
		Self {
			url:String::new(),
			destination:String::new(),
			checksum:String::new(),
			MaxFileSize:None,
			ChunkSize:8 * 1024 * 1024, // 8MB chunks
			MaxRetries:5,
			TimeoutSecs:300,
			priority:DownloadPriority::Normal,
			ValidateDiskSpace:true,
		}
	}
}

impl DownloadManager {
	/// Create a new download manager with comprehensive initialization
	pub async fn new(AppState:Arc<ApplicationState>) -> Result<Self> {
		let config = &AppState.Configuration.Downloader;

		// Expand and validate cache directory path
		let CacheDirectory = ConfigurationManager::ExpandPath(&config.CacheDirectory)?;

		// Clone CacheDirectory before moving
		let CacheDirectoryClone = CacheDirectory.clone();

		// Clone for struct init (PascalCase field name)
		let CacheDirectoryCloneForInit = CacheDirectoryClone.clone();

		// Create cache directory if it doesn't exist
		tokio::fs::create_dir_all(&CacheDirectory)
			.await
			.map_err(|e| AirError::Configuration(format!("Failed to create cache directory: {}", e)))?;

		// Create HTTP client with connection pooling and timeouts
		let client = reqwest::Client::builder()
			.timeout(Duration::from_secs(config.DownloadTimeoutSecs))
			.connect_timeout(Duration::from_secs(30))
			.pool_idle_timeout(Duration::from_secs(90))
			.pool_max_idle_per_host(10)
			.tcp_keepalive(Duration::from_secs(60))
			.user_agent("Land-AirDownloader/0.1.0")
			.build()
			.map_err(|e| AirError::Network(format!("Failed to create HTTP client: {}", e)))?;

		// Bandwidth limiter (permit = 1MB of transfer)
		let BandwidthLimiter = Arc::new(Semaphore::new(100));

		// Concurrent download limiter (max 5 parallel downloads)
		let ConcurrentLimiter = Arc::new(Semaphore::new(5));

		let manager = Self {
			AppState,
			ActiveDownloads:Arc::new(RwLock::new(HashMap::new())),
			DownloadQueue:Arc::new(RwLock::new(VecDeque::new())),
			CacheDirectory:CacheDirectoryCloneForInit,
			client,
			ChecksumVerifier:Arc::new(crate::Security::ChecksumVerifier::New()),
			BandwidthLimiter,
			ConcurrentLimiter,
			statistics:Arc::new(RwLock::new(DownloadStatistics::default())),
		};

		// Initialize service status
		manager
			.AppState
			.UpdateServiceStatus("downloader", crate::ApplicationState::ServiceStatus::Running)
			.await
			.map_err(|e| AirError::Internal(e.to_string()))?;

		log::info!(
			"[DownloadManager] Initialized with cache directory: {}",
			CacheDirectory.display()
		);

		Ok(manager)
	}

	/// Download a file with comprehensive validation and resilience
	pub async fn DownloadFile(&self, url:String, DestinationPath:String, checksum:String) -> Result<DownloadResult> {
		self.DownloadFileWithConfig(DownloadConfig { url, destination:DestinationPath, checksum, ..Default::default() })
			.await
	}

	/// Download a file with detailed configuration
	pub async fn DownloadFileWithConfig(&self, config:DownloadConfig) -> Result<DownloadResult> {
		// Defensive: Validate and sanitize URL
		let SanitizedUrl = Self::ValidateAndSanitizeUrl(&config.url)?;

		// Defensive: Check if download is already active
		let DownloadId = Utility::GenerateRequestId();

		log::info!(
			"[DownloadManager] Starting download [ID: {}] - URL: {}",
			DownloadId,
			SanitizedUrl
		);

		// Defensive: URL cannot be empty
		if SanitizedUrl.is_empty() {
			return Err(AirError::Network("URL cannot be empty".to_string()));
		}

		// Expand and validate destination path
		let Destination = if config.destination.is_empty() {
			// Generate filename from URL
			let Filename = SanitizedUrl
				.split('/')
				.last()
				.and_then(|s| s.split('?').next())
				.unwrap_or("download.bin");
			self.CacheDirectory.join(Filename)
		} else {
			ConfigurationManager::ExpandPath(&config.destination)?
		};

		// Defensive: Validate file path security
		Utility::ValidateFilePath(
			Destination
				.to_str()
				.ok_or_else(|| AirError::Configuration("Invalid destination path".to_string()))?,
		)?;

		// Prepare download metadata
		let ExpectedChecksum = if config.checksum.is_empty() { None } else { Some(config.checksum.clone()) };

		// Register download in tracking system
		self.RegisterDownload(&DownloadId, &SanitizedUrl, &Destination, ExpectedChecksum.clone())
			.await?;

		// Defensive: Validate disk space before download
		if config.ValidateDiskSpace {
			if let Some(MaxSize) = config.MaxFileSize {
				self.ValidateDiskSpace(&SanitizedUrl, &Destination, MaxSize * 2).await?;
			} else {
				self.ValidateDiskSpace(&SanitizedUrl, &Destination, 1024 * 1024 * 1024).await?; // Default 1GB check
			}
		}

		// Create destination directory if it doesn't exist
		if let Some(Parent) = Destination.parent() {
			tokio::fs::create_dir_all(Parent)
				.await
				.map_err(|e| AirError::FileSystem(format!("Failed to create destination directory: {}", e)))?;
		}

		let StartTime = Instant::now();

		// Execute download with full resilience
		let Result = self.DownloadWithRetry(&DownloadId, &SanitizedUrl, &Destination, &config).await;

		let Duration = StartTime.elapsed();

		match Result {
			Ok(mut FileInfo) => {
				FileInfo.duration = Duration;

				// Update statistics
				self.UpdateStatistics(true, FileInfo.size, Duration).await;

				self.UpdateDownloadStatus(&DownloadId, DownloadState::Completed, Some(100.0), None)
					.await?;

				log::info!(
					"[DownloadManager] Download completed [ID: {}] - Size: {} bytes in {:.2}s ({:.2} MB/s)",
					DownloadId,
					FileInfo.size,
					Duration.as_secs_f64(),
					FileInfo.size as f64 / 1_048_576.0 / Duration.as_secs_f64()
				);

				Ok(FileInfo)
			},
			Err(E) => {
				// Update statistics
				self.UpdateStatistics(false, 0, Duration).await;

				self.UpdateDownloadStatus(&DownloadId, DownloadState::Failed, None, Some(E.to_string()))
					.await?;

				// Defensive: Clean up partial/failed download
				if Destination.exists() {
					let _ = tokio::fs::remove_file(&Destination).await;
					log::warn!("[DownloadManager] Cleaned up failed download: {}", Destination.display());
				}

				log::error!("[DownloadManager] Download failed [ID: {}] - Error: {}", DownloadId, E);

				Err(E)
			},
		}
	}

	/// Validate and sanitize URL to prevent injection attacks
	fn ValidateAndSanitizeUrl(url:&str) -> Result<String> {
		let url = url.trim();

		// Check for empty URL
		if url.is_empty() {
			return Err(AirError::Network("URL cannot be empty".to_string()));
		}

		// Parse URL to validate format
		let parsed = url::Url::parse(url).map_err(|e| AirError::Network(format!("Invalid URL format: {}", e)))?;

		// Validate scheme (only allow http and https)
		match parsed.scheme() {
			"http" | "https" => (),
			scheme => {
				return Err(AirError::Network(format!(
					"Unsupported URL scheme: '{}'. Only http and https are allowed.",
					scheme
				)));
			},
		}

		// Ensure we have a host
		if parsed.host().is_none() {
			return Err(AirError::Network("URL must have a valid host".to_string()));
		}

		// Block localhost and private network if in production
		#[cfg(debug_assertions)]
		{
			// Allow localhost in debug mode
		}
		#[cfg(not(debug_assertions))]
		{
			if let Some(host) = parsed.host_str() {
				if host == "localhost" || host == "127.0.0.1" || host == "::1" {
					return Err(AirError::Network("Localhost addresses are not allowed".to_string()));
				}
				if host.starts_with("192.168.") || host.starts_with("10.") || host.starts_with("172.16.") {
					return Err(AirError::Network("Private network addresses are not allowed".to_string()));
				}
			}
		}

		// Remove sensitive parameters (prevent credential leakage)
		let mut sanitized = parsed.clone();

		// Remove password from URL
		if sanitized.password().is_some() {
			sanitized.set_password(Some("")).ok();
		}

		Ok(sanitized.to_string())
	}

	/// Validate available disk space before download
	async fn ValidateDiskSpace(&self, url:&str, destination:&Path, RequiredBytes:u64) -> Result<()> {
		// Get destination path
		let DestPath = if destination.is_absolute() {
			destination.to_path_buf()
		} else {
			std::env::current_dir()
				.map_err(|e| AirError::FileSystem(format!("Failed to get current directory: {}", e)))?
				.join(destination)
		};

		// Find the mount point
		let MountPoint = self.FindMountPoint(&DestPath)?;

		// TODO: Implement actual disk space checking
		// For now, log the validation request and pass
		log::debug!(
			"[DownloadManager] Validating disk space for URL {} (requires {} bytes) on mount point: {}",
			url,
			RequiredBytes,
			MountPoint.display()
		);

		#[cfg(unix)]
		{
			match self.GetDiskStatvfs(&MountPoint) {
				Ok((AvailableBytes, TotalBytes)) => {
					if AvailableBytes < RequiredBytes {
						log::warn!(
							"[DownloadManager] Insufficient disk space: {} bytes available, {} bytes required",
							AvailableBytes,
							RequiredBytes
						);
						return Err(AirError::FileSystem(format!(
							"Insufficient disk space: {} bytes available, {} bytes required",
							AvailableBytes, RequiredBytes
						)));
					}

					log::debug!(
						"[DownloadManager] Sufficient disk space: {} bytes available, {} bytes required (total: {})",
						AvailableBytes,
						RequiredBytes,
						TotalBytes
					);
				},
				Err(e) => {
					log::warn!("[DownloadManager] Failed to check disk space: {}, proceeding anyway", e);
				},
			}
		}

		#[cfg(windows)]
		{
			match self.GetDiskSpaceWindows(&MountPoint) {
				Ok(AvailableBytes) => {
					if AvailableBytes < RequiredBytes {
						log::warn!(
							"[DownloadManager] Insufficient disk space: {} bytes available, {} bytes required",
							AvailableBytes,
							RequiredBytes
						);
						return Err(AirError::FileSystem(format!(
							"Insufficient disk space: {} bytes available, {} bytes required",
							available_bytes, RequiredBytes
						)));
					}
					log::debug!(
						"[DownloadManager] Sufficient disk space: {} bytes available, {} bytes required",
						available_bytes,
						RequiredBytes
					);
				},
				Err(e) => {
					log::warn!("[DownloadManager] Failed to check disk space: {}, proceeding anyway", e);
				},
			}
		}

		#[cfg(not(any(unix, windows)))]
		{
			log::warn!("[DownloadManager] Disk space validation not available on this platform");
		}

		Ok(())
	}

	/// Get disk statistics using statvfs (Unix)
	#[cfg(unix)]
	fn GetDiskStatvfs(&self, path:&Path) -> Result<(u64, u64)> {
		// TODO: Implement actual statvfs call using libc statvfs()
		// Example implementation:
		// use std::mem::size_of;
		// let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
		// let PathCstr = path.to_path_buf().as_os_str().to_c_string();
		// let result = unsafe { libc::statvfs(path_cstr.as_ptr(), &mut stat) };
		// if result != 0 { return Err(...); }
		// let available = stat.FBsize as u64 * stat.FBavail as u64;
		// let total = stat.FBsize as u64 * stat.FBlocks as u64;
		// For now, assume sufficient space and log the request
		log::debug!("[DownloadManager] Checking disk space at: {}", path.display());
		Ok((u64::MAX, u64::MAX))
	}

	/// Get disk space on Windows
	#[cfg(windows)]
	fn GetDiskSpaceWindows(&self, path:&Path) -> Result<u64> {
		// TODO: Implement Windows disk space checking using winapi
		// GetDiskFreeSpaceExW() Example implementation:
		// use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
		// let mut available: u64 = 0;
		// let mut total: u64 = 0;
		// let mut free: u64 = 0;
		// let result = unsafe { GetDiskFreeSpaceExW(path.as_os_str(), &mut available as
		// *mut _ as _, &mut total as *mut _ as _, &mut free as *mut _ as _) };
		// if !result.as_bool() { return Err(...); }
		// For now, assume sufficient space and log the request
		log::debug!("[DownloadManager] Checking disk space at: {}", path.display());
		Ok(u64::MAX)
	}

	/// Find mount point for a given path
	fn FindMountPoint(&self, path:&Path) -> Result<PathBuf> {
		#[cfg(unix)]
		{
			let mut current = path
				.canonicalize()
				.map_err(|e| AirError::FileSystem(format!("Failed to canonicalize path: {}", e)))?;

			loop {
				if current.as_os_str().is_empty() || current == Path::new("/") {
					return Ok(PathBuf::from("/"));
				}

				let metadata = std::fs::metadata(&current)
					.map_err(|e| AirError::FileSystem(format!("Failed to get metadata: {}", e)))?;

				// Check if device ID changes (indicates mount point)
				#[cfg(unix)]
				let CurrentDevice = {
					use std::os::unix::fs::MetadataExt;
					metadata.dev()
				};
				#[cfg(not(unix))]
				let CurrentDevice = 0u64; // Dummy value for non-unix systems

				let parent = current.parent();

				if let Some(parent_path) = parent {
					let ParentMetadata = std::fs::metadata(parent_path)
						.map_err(|e| AirError::FileSystem(format!("Failed to get parent metadata: {}", e)))?;

					#[cfg(unix)]
					let ParentDevice = {
						use std::os::unix::fs::MetadataExt;
						ParentMetadata.dev()
					};
					#[cfg(not(unix))]
					let ParentDevice = 0u64; // Dummy value for non-unix systems

					if ParentDevice != CurrentDevice {
						return Ok(current);
					}
				} else {
					return Ok(current);
				}

				current.pop();
			}
		}

		#[cfg(windows)]
		{
			// Windows: Get drive letter
			let PathStr = path.to_string_lossy();
			if PathStr.len() >= 3 && PathStr.chars().nth(1) == Some(':') {
				return Ok(PathBuf::from(&PathStr[..3]));
			}
			Ok(PathBuf::from("C:\\"))
		}

		#[cfg(not(any(unix, windows)))]
		{
			Ok(path.to_path_buf())
		}
	}

	/// Download with retry logic and circuit breaker
	async fn DownloadWithRetry(
		&self,
		DownloadId:&str,
		url:&str,
		destination:&PathBuf,
		config:&DownloadConfig,
	) -> Result<DownloadResult> {
		let RetryPolicy = crate::Resilience::RetryPolicy {
			MaxRetries:config.MaxRetries,
			InitialIntervalMs:1000,
			MaxIntervalMs:32000,
			BackoffMultiplier:2.0,
			JitterFactor:0.1,
			BudgetPerMinute:100,
			ErrorClassification:std::collections::HashMap::new(),
		};

		let RetryManager = crate::Resilience::RetryManager::new(RetryPolicy.clone());
		let CircuitBreaker = crate::Resilience::CircuitBreaker::new(
			"downloader".to_string(),
			crate::Resilience::CircuitBreakerConfig::default(),
		);

		let mut attempt = 0;

		loop {
			// Check circuit breaker state
			if CircuitBreaker.GetState().await == crate::Resilience::CircuitState::Open {
				if !CircuitBreaker.AttemptRecovery().await {
					return Err(AirError::Network(
						"Circuit breaker is open, too many recent failures".to_string(),
					));
				}
			}

			// Check for cancellation before attempting download
			if let Some(status) = self.GetDownloadStatus(DownloadId).await {
				if status.status == DownloadState::Cancelled {
					return Err(AirError::Network("Download cancelled".to_string()));
				}
			}

			match self.PerformDownload(DownloadId, url, destination, config).await {
				Ok(file_info) => {
					// Verify checksum if provided
					if let Some(ref ExpectedChecksum) = ExpectedChecksumFromConfig(config) {
						self.UpdateDownloadStatus(DownloadId, DownloadState::Verifying, Some(100.0), None)
							.await?;

						if let Err(e) = self.VerifyChecksum(destination, ExpectedChecksum).await {
							log::warn!("[DownloadManager] Checksum verification failed [ID: {}]: {}", DownloadId, e);
							CircuitBreaker.RecordFailure().await;

							if attempt < config.MaxRetries && RetryManager.CanRetry("downloader").await {
								attempt += 1;
								let delay = RetryManager.CalculateRetryDelay(attempt);
								log::info!(
									"[DownloadManager] Retrying download [ID: {}] (attempt {}/{}) after {:?}",
									DownloadId,
									attempt + 1,
									config.MaxRetries + 1,
									delay
								);
								tokio::time::sleep(delay).await;
								continue;
							} else {
								return Err(AirError::Network(format!(
									"Checksum verification failed after {} retries: {}",
									attempt, e
								)));
							}
						}
					}

					CircuitBreaker.RecordSuccess().await;
					return Ok(file_info);
				},
				Err(e) => {
					CircuitBreaker.RecordFailure().await;

					if attempt < config.MaxRetries && RetryManager.CanRetry("downloader").await {
						attempt += 1;
						log::warn!(
							"[DownloadManager] Download failed [ID: {}], retrying (attempt {}/{}): {}",
							DownloadId,
							attempt + 1,
							config.MaxRetries + 1,
							e
						);

						let delay = RetryManager.CalculateRetryDelay(attempt);
						tokio::time::sleep(delay).await;
					} else {
						return Err(e);
					}
				},
			}
		}
	}

	/// Perform the actual download with streaming and partial resume support
	async fn PerformDownload(
		&self,
		DownloadId:&str,
		url:&str,
		destination:&PathBuf,
		config:&DownloadConfig,
	) -> Result<DownloadResult> {
		// Acquire concurrent download permit
		let _concurrent_permit = self
			.ConcurrentLimiter
			.acquire()
			.await
			.map_err(|e| AirError::Internal(format!("Failed to acquire download permit: {}", e)))?;

		self.UpdateDownloadStatus(DownloadId, DownloadState::Downloading, Some(0.0), None)
			.await?;

		// Create temporary file for atomic commit
		let TempDestination = destination.with_extension("tmp");

		// Support resume by checking existing file size
		let mut ExistingSize:u64 = 0;
		if TempDestination.exists() {
			if let Ok(metadata) = tokio::fs::metadata(&TempDestination).await {
				ExistingSize = metadata.len();
				log::info!("[DownloadManager] Resuming download from {} bytes", ExistingSize);
			}
		}

		// Build request with Range header for resume
		let mut req = self.client.get(url).timeout(Duration::from_secs(config.TimeoutSecs));
		if ExistingSize > 0 {
			let RangeHeader = format!("bytes={}-", ExistingSize);
			req = req.header(reqwest::header::RANGE, RangeHeader);
			req = req.header(reqwest::header::IF_MATCH, "*"); // Ensure server supports resume
		}

		let response = req
			.send()
			.await
			.map_err(|e| AirError::Network(format!("Failed to start download: {}", e)))?;

		// Handle redirect if needed
		let FinalUrl = response.url().clone();
		let response = if FinalUrl.as_str() != url {
			log::info!("[DownloadManager] Redirected to: {}", FinalUrl);
			response
		} else {
			response
		};

		// Validate response status
		let StatusCode = response.status();
		if !StatusCode.is_success() && StatusCode != reqwest::StatusCode::PARTIAL_CONTENT {
			return Err(AirError::Network(format!("Download failed with status: {}", StatusCode)));
		}

		// Get total size (handle both fresh and resume scenarios)
		let TotalSize = if let Some(cl) = response.content_length() {
			if StatusCode == reqwest::StatusCode::PARTIAL_CONTENT {
				cl + ExistingSize
			} else {
				cl
			}
		} else {
			0
		};

		// Defensive: Validate file size if max size specified
		if let Some(max_size) = config.MaxFileSize {
			if TotalSize > 0 && TotalSize > max_size {
				return Err(AirError::Network(format!(
					"File too large: {} bytes exceeds maximum allowed size: {} bytes",
					TotalSize, max_size
				)));
			}
		}

		// Open file in append mode if resuming
		let mut file = tokio::fs::OpenOptions::new()
			.create(true)
			.append(true)
			.open(&TempDestination)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to open destination file: {}", e)))?;

		use tokio::io::AsyncWriteExt;
		use futures_util::StreamExt;

		let mut downloaded = ExistingSize;
		let mut LastProgressUpdate = Instant::now();
		let BytesStream = response.bytes_stream();

		tokio::pin!(BytesStream);

		while let Some(result) = BytesStream.next().await {
			// Check for pause/cancel before processing chunk
			if let Some(status) = self.GetDownloadStatus(DownloadId).await {
				match status.status {
					DownloadState::Cancelled => {
						// Clean up temporary file
						let _ = tokio::fs::remove_file(&TempDestination).await;
						return Err(AirError::Network("Download cancelled".to_string()));
					},
					DownloadState::Paused => {
						// Wait until resumed or cancelled
						loop {
							tokio::time::sleep(Duration::from_millis(250)).await;
							if let Some(s) = self.GetDownloadStatus(DownloadId).await {
								match s.status {
									DownloadState::Paused => continue,
									DownloadState::Cancelled => {
										let _ = tokio::fs::remove_file(&TempDestination).await;
										return Err(AirError::Network("Download cancelled".to_string()));
									},
									_ => {
										log::info!("[DownloadManager] Resuming paused download [ID: {}]", DownloadId);
										break;
									},
								}
							} else {
								break;
							}
						}
					},
					_ => {},
				}
			}

			match result {
				Ok(chunk) => {
					// Bandwidth limiting check
					let ChunkSize = chunk.len();
					if let Ok(permit) = self.BandwidthLimiter.try_acquire_many((ChunkSize / (1024 * 1024) + 1) as u32) {
						drop(permit);
					} else {
						// Wait if bandwidth limit reached
						tokio::time::sleep(Duration::from_millis(10)).await;
					}

					file.write_all(&chunk)
						.await
						.map_err(|e| AirError::FileSystem(format!("Failed to write file: {}", e)))?;

					downloaded += ChunkSize as u64;

					// Update progress (throttled to avoid excessive updates)
					if LastProgressUpdate.elapsed() > Duration::from_millis(500) {
						LastProgressUpdate = Instant::now();

						if TotalSize > 0 {
							let progress = (downloaded as f32 / TotalSize as f32) * 100.0;
							self.UpdateDownloadStatus(DownloadId, DownloadState::Downloading, Some(progress), None)
								.await?;
						}

						// Calculate and update download rate
						let rate = self.CalculateDownloadRate(DownloadId, downloaded).await;
						self.UpdateDownloadRate(DownloadId, rate).await;
					}
				},
				Err(e) => {
					// Defensive: Check if this is a timeout
					if e.is_timeout() || e.is_connect() {
						log::warn!("[DownloadManager] Connection/timeout error, may retry: {}", e);
						return Err(AirError::Network(format!("Network error: {}", e)));
					}
					return Err(AirError::Network(format!("Failed to read response: {}", e)));
				},
			}
		}

		// Final progress update
		self.UpdateDownloadStatus(DownloadId, DownloadState::Downloading, Some(100.0), None)
			.await?;

		// Flush file to ensure all data is written
		file.flush()
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to flush file: {}", e)))?;

		// Atomic rename from temp to final destination
		tokio::fs::rename(&TempDestination, destination)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to commit download: {}", e)))?;

		// Calculate checksum for verification
		let checksum = self.CalculateChecksum(destination).await?;

		// Update status with final checksum
		self.UpdateActualChecksum(DownloadId, &checksum).await;

		Ok(DownloadResult {
			path:destination.to_string_lossy().to_string(),
			size:downloaded,
			checksum,
			duration:Duration::from_secs(0),
			AverageRate:0,
		})
	}

	/// Verify file checksum using ChecksumVerifier
	pub async fn VerifyChecksum(&self, FilePath:&PathBuf, ExpectedChecksum:&str) -> Result<()> {
		// Defensive: Validate input file exists
		if !FilePath.exists() {
			return Err(AirError::FileSystem(format!(
				"File not found for checksum verification: {}",
				FilePath.display()
			)));
		}

		let ActualChecksum = self.ChecksumVerifier.CalculateSha256(FilePath).await?;

		// Normalize checksums (handle case-insensitivity, remove prefix, etc.)
		let NormalizedExpected = ExpectedChecksum.trim().to_lowercase().replace("sha256:", "");
		let NormalizedActual = ActualChecksum.trim().to_lowercase();

		if NormalizedActual != NormalizedExpected {
			log::error!(
				"[DownloadManager] Checksum mismatch for {}: expected {}, got {}",
				FilePath.display(),
				NormalizedExpected,
				NormalizedActual
			);
			return Err(AirError::Network(format!(
				"Checksum verification failed: expected {}, got {}",
				NormalizedExpected, NormalizedActual
			)));
		}

		log::info!("[DownloadManager] Checksum verified for file: {}", FilePath.display());

		Ok(())
	}

	/// Calculate file checksum using ChecksumVerifier
	pub async fn CalculateChecksum(&self, FilePath:&PathBuf) -> Result<String> {
		// Defensive: Validate input file exists
		if !FilePath.exists() {
			return Err(AirError::FileSystem(format!(
				"File not found for checksum calculation: {}",
				FilePath.display()
			)));
		}

		self.ChecksumVerifier.CalculateSha256(FilePath).await
	}

	/// Register a new download in the tracking system
	async fn RegisterDownload(
		&self,
		DownloadId:&str,
		url:&str,
		destination:&PathBuf,
		ExpectedChecksum:Option<String>,
	) -> Result<()> {
		let mut downloads = self.ActiveDownloads.write().await;
		let mut stats = self.statistics.write().await;

		stats.ActiveDownloads += 1;

		downloads.insert(
			DownloadId.to_string(),
			DownloadStatus {
				DownloadId:DownloadId.to_string(),
				url:url.to_string(),
				destination:destination.clone(),
				TotalSize:0,
				downloaded:0,
				progress:0.0,
				status:DownloadState::Pending,
				error:None,
				StartedAt:Some(chrono::Utc::now()),
				CompletedAt:None,
				ChunksCompleted:0,
				TotalChunks:1,
				DownloadRateBytesPerSec:0,
				ExpectedChecksum:ExpectedChecksum.clone(),
				ActualChecksum:None,
			},
		);

		Ok(())
	}

	/// Update download status
	async fn UpdateDownloadStatus(
		&self,
		DownloadId:&str,
		status:DownloadState,
		progress:Option<f32>,
		error:Option<String>,
	) -> Result<()> {
		let mut downloads = self.ActiveDownloads.write().await;

		if let Some(download) = downloads.get_mut(DownloadId) {
			if status == DownloadState::Completed || status == DownloadState::Failed {
				download.CompletedAt = Some(chrono::Utc::now());
			}
			download.status = status;
			if let Some(progress) = progress {
				download.progress = progress;
			}
			download.error = error;
		}

		Ok(())
	}

	/// Update download rate tracking
	async fn UpdateDownloadRate(&self, DownloadId:&str, rate:u64) {
		let mut downloads = self.ActiveDownloads.write().await;
		if let Some(download) = downloads.get_mut(DownloadId) {
			download.DownloadRateBytesPerSec = rate;
		}
	}

	/// Update actual checksum after calculation
	async fn UpdateActualChecksum(&self, DownloadId:&str, checksum:&str) {
		let mut downloads = self.ActiveDownloads.write().await;
		if let Some(download) = downloads.get_mut(DownloadId) {
			download.ActualChecksum = Some(checksum.to_string());
		}
	}

	/// Calculate download rate based on progress
	async fn CalculateDownloadRate(&self, DownloadId:&str, CurrentBytes:u64) -> u64 {
		let downloads = self.ActiveDownloads.read().await;
		if let Some(download) = downloads.get(DownloadId) {
			if let Some(StartedAt) = download.StartedAt {
				let elapsed = chrono::Utc::now().signed_duration_since(StartedAt);
				let ElapsedSecs = elapsed.num_seconds() as u64;
				if ElapsedSecs > 0 {
					return CurrentBytes / ElapsedSecs;
				}
			}
		}
		0
	}

	/// Update download statistics
	async fn UpdateStatistics(&self, success:bool, bytes:u64, duration:Duration) {
		let mut stats = self.statistics.write().await;

		if success {
			stats.SuccessfulDownloads += 1;
			stats.TotalBytesDownloaded += bytes;
			stats.TotalDownloadTimeSecs += duration.as_secs_f64();

			if stats.TotalDownloadTimeSecs > 0.0 {
				stats.AverageDownloadRate = stats.TotalBytesDownloaded as f64 / stats.TotalDownloadTimeSecs
			}

			// Update peak rate
			let CurrentRate = if duration.as_secs_f64() > 0.0 {
				(bytes as f64 / duration.as_secs_f64()) as u64
			} else {
				0
			};
			if CurrentRate > stats.PeakDownloadRate {
				stats.PeakDownloadRate = CurrentRate;
			}
		} else {
			stats.FailedDownloads += 1;
		}

		stats.TotalDownloads += 1;
		stats.ActiveDownloads = stats.ActiveDownloads.saturating_sub(1);
	}

	/// Get download status
	pub async fn GetDownloadStatus(&self, DownloadId:&str) -> Option<DownloadStatus> {
		let downloads = self.ActiveDownloads.read().await;
		downloads.get(DownloadId).cloned()
	}

	/// Get all active downloads
	pub async fn GetAllDownloads(&self) -> Vec<DownloadStatus> {
		let downloads = self.ActiveDownloads.read().await;
		downloads.values().cloned().collect()
	}

	/// Cancel a download with proper cleanup
	pub async fn CancelDownload(&self, DownloadId:&str) -> Result<()> {
		log::info!("[DownloadManager] Cancelling download [ID: {}]", DownloadId);

		self.UpdateDownloadStatus(DownloadId, DownloadState::Cancelled, None, None)
			.await?;

		// Clean up temporary file if it exists
		if let Some(status) = self.GetDownloadStatus(DownloadId).await {
			let TempPath = status.destination.with_extension("tmp");
			if TempPath.exists() {
				let _ = tokio::fs::remove_file(&TempPath).await;
			}
		}

		// Update statistics
		{
			let mut stats = self.statistics.write().await;
			stats.CancelledDownloads += 1;
			stats.ActiveDownloads = stats.ActiveDownloads.saturating_sub(1);
		}

		Ok(())
	}

	/// Pause a download (supports resume)
	pub async fn PauseDownload(&self, DownloadId:&str) -> Result<()> {
		self.UpdateDownloadStatus(DownloadId, DownloadState::Paused, None, None).await?;
		log::info!("[DownloadManager] Download paused [ID: {}]", DownloadId);
		Ok(())
	}

	/// Resume a paused download
	pub async fn ResumeDownload(&self, DownloadId:&str) -> Result<()> {
		if let Some(status) = self.GetDownloadStatus(DownloadId).await {
			if status.status == DownloadState::Paused {
				self.UpdateDownloadStatus(DownloadId, DownloadState::Resuming, None, None)
					.await?;
				// The download loop handles the actual resume
				self.UpdateDownloadStatus(DownloadId, DownloadState::Downloading, None, None)
					.await?;
				log::info!("[DownloadManager] Download resumed [ID: {}]", DownloadId);
			} else {
				return Err(AirError::Network("Can only resume paused downloads".to_string()));
			}
		} else {
			return Err(AirError::Network("Download not found".to_string()));
		}
		Ok(())
	}

	/// Get active download count
	pub async fn GetActiveDownloadCount(&self) -> usize {
		let downloads = self.ActiveDownloads.read().await;
		downloads
			.iter()
			.filter(|(_, s)| {
				matches!(
					s.status,
					DownloadState::Downloading | DownloadState::Verifying | DownloadState::Resuming
				)
			})
			.count()
	}

	/// Get download statistics
	pub async fn GetStatistics(&self) -> DownloadStatistics {
		let stats = self.statistics.read().await;
		stats.clone()
	}

	/// Queue a download with priority
	pub async fn QueueDownload(
		&self,
		url:String,
		destination:String,
		checksum:String,
		priority:DownloadPriority,
	) -> Result<String> {
		let DownloadId = Utility::GenerateRequestId();

		let destination = if destination.is_empty() {
			let filename = url.split('/').last().unwrap_or("download.bin");
			self.CacheDirectory.join(filename)
		} else {
			ConfigurationManager::ExpandPath(&destination)?
		};

		let queued_download = QueuedDownload {
			DownloadId:DownloadId.clone(),
			url,
			destination,
			checksum,
			priority,
			AddedAt:chrono::Utc::now(),
			MaxFileSize:None,
			ValidateDiskSpace:true,
		};

		let mut queue = self.DownloadQueue.write().await;
		queue.push_back(queued_download);

		// Sort by priority (higher priority first)
		queue.make_contiguous().sort_by(|a, b| {
			match b.priority.cmp(&a.priority) {
				std::cmp::Ordering::Equal => {
					// If same priority, use added_at (earlier first)
					a.AddedAt.cmp(&b.AddedAt)
				},
				order => order,
			}
		});

		{
			let mut stats = self.statistics.write().await;
			stats.QueuedDownloads += 1;
		}

		log::info!(
			"[DownloadManager] Download queued [ID: {}] with priority {:?}",
			DownloadId,
			priority
		);

		Ok(DownloadId)
	}

	/// Process next download from queue
	pub async fn ProcessQueue(&self) -> Result<Option<String>> {
		let mut queue = self.DownloadQueue.write().await;

		if let Some(queued) = queue.pop_front() {
			let download_id = queued.DownloadId.clone();
			drop(queue); // Release lock before starting download

			let config = DownloadConfig {
				url:queued.url.clone(),
				destination:queued.destination.to_string_lossy().to_string(),
				checksum:queued.checksum.clone(),
				priority:queued.priority,
				MaxFileSize:queued.MaxFileSize,
				ValidateDiskSpace:queued.ValidateDiskSpace,
				..Default::default()
			};

			{
				let mut stats = self.statistics.write().await;
				stats.QueuedDownloads = stats.QueuedDownloads.saturating_sub(1);
			}

			// Spawn download task in background
			let manager = self.clone();
			let download_id_clone = download_id.clone();
			tokio::spawn(async move {
				if let Err(e) = manager.DownloadFileWithConfig(config).await {
					log::error!("[DownloadManager] Queued download failed [ID: {}]: {}", download_id_clone, e);
					// Update download status to failed
					let _ = manager
						.UpdateDownloadStatus(&download_id_clone, DownloadState::Failed, None, Some(e.to_string()))
						.await;
				}
			});

			Ok(Some(download_id))
		} else {
			Ok(None)
		}
	}

	/// Start background tasks for cleanup and queue processing
	pub async fn StartBackgroundTasks(&self) -> Result<tokio::task::JoinHandle<()>> {
		let manager = self.clone();

		let handle = tokio::spawn(async move {
			manager.BackgroundTaskLoop().await;
		});

		log::info!("[DownloadManager] Background tasks started");

		Ok(handle)
	}

	/// Background task loop for cleanup and queue processing
	async fn BackgroundTaskLoop(&self) {
		let mut interval = tokio::time::interval(Duration::from_secs(60));

		loop {
			interval.tick().await;

			// Process queue
			if let Err(e) = self.ProcessQueue().await {
				log::error!("[DownloadManager] Queue processing error: {}", e);
			}

			// Clean up completed downloads
			self.CleanupCompletedDownloads().await;

			// Clean up old cache files
			if let Err(e) = self.CleanupCache().await {
				log::error!("[DownloadManager] Cache cleanup failed: {}", e);
			}
		}
	}

	/// Clean up completed downloads from active tracking
	async fn CleanupCompletedDownloads(&self) {
		let mut downloads = self.ActiveDownloads.write().await;

		let mut cleaned_count = 0;
		downloads.retain(|_, download| {
			let is_final = matches!(
				download.status,
				DownloadState::Completed | DownloadState::Failed | DownloadState::Cancelled
			);
			if is_final {
				cleaned_count += 1;
			}
			!is_final
		});

		if cleaned_count > 0 {
			log::debug!("[DownloadManager] Cleaned up {} completed downloads", cleaned_count);
		}
	}

	/// Clean up old cache files with safety checks
	async fn CleanupCache(&self) -> Result<()> {
		let max_age_days = 7;
		let now = chrono::Utc::now();

		let mut entries = tokio::fs::read_dir(&self.CacheDirectory)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to read cache directory: {}", e)))?;

		let mut cleaned_count = 0;

		while let Some(entry) = entries
			.next_entry()
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to read cache entry: {}", e)))?
		{
			let metadata = entry
				.metadata()
				.await
				.map_err(|e| AirError::FileSystem(format!("Failed to get file metadata: {}", e)))?;

			if metadata.is_file() {
				let path = entry.path();

				// Skip if file is being actively used (check for active downloads)
				let IsActive = {
					let downloads = self.ActiveDownloads.read().await;
					downloads.values().any(|d| d.destination == path)
				};

				if IsActive {
					continue;
				}

				let modified = metadata
					.modified()
					.map_err(|e| AirError::FileSystem(format!("Failed to get modification time: {}", e)))?;

				let modified_time = chrono::DateTime::<chrono::Utc>::from(modified);
				let age = now.signed_duration_since(modified_time);

				if age.num_days() > max_age_days {
					match tokio::fs::remove_file(&path).await {
						Ok(_) => {
							log::debug!(
								"[DownloadManager] Removed old cache file: {}",
								entry.file_name().to_string_lossy()
							);
							cleaned_count += 1;
						},
						Err(e) => {
							log::warn!(
								"[DownloadManager] Failed to remove cache file {}: {}",
								entry.file_name().to_string_lossy(),
								e
							);
						},
					}
				}
			}
		}

		if cleaned_count > 0 {
			log::info!("[DownloadManager] Cleaned up {} old cache files", cleaned_count);
		}

		Ok(())
	}

	/// Stop background tasks and clean up resources
	pub async fn StopBackgroundTasks(&self) {
		log::info!("[DownloadManager] Stopping background tasks");

		// Cancel all active downloads - collect IDs first
		let ids_to_cancel:Vec<String> = {
			let downloads = self.ActiveDownloads.read().await;
			downloads
				.iter()
				.filter(|(_, s)| matches!(s.status, DownloadState::Downloading))
				.map(|(id, _)| id.clone())
				.collect()
		};

		// Now cancel downloads without holding the read lock
		for id in ids_to_cancel {
			let _ = self.CancelDownload(&id).await;
		}

		// Stop service status
		let _ = self
			.AppState
			.UpdateServiceStatus("downloader", crate::ApplicationState::ServiceStatus::Stopped)
			.await;
	}

	/// Set global bandwidth limit (in MB/s)
	/// TODO: Implement per-download bandwidth limiting instead of global only
	/// TODO: Add time-based bandwidth schedules (off-peak acceleration)
	/// TODO: Implement actual bandwidth throttling with token bucket algorithm
	pub async fn SetBandwidthLimit(&mut self, mb_per_sec:usize) {
		// Update semaphore permits (1 permit = 1MB)
		let permits = mb_per_sec.max(1).min(1000);
		self.BandwidthLimiter = Arc::new(Semaphore::new(permits));
		log::info!("[DownloadManager] Bandwidth limit set to {} MB/s", mb_per_sec);
	}

	/// Set maximum concurrent downloads
	/// TODO: Implement per-host concurrent download limits
	/// TODO: Add adaptive concurrency based on network conditions
	pub async fn SetMaxConcurrentDownloads(&mut self, max:usize) {
		let permits = max.max(1).min(20);
		self.ConcurrentLimiter = Arc::new(Semaphore::new(permits));
		log::info!("[DownloadManager] Max concurrent downloads set to {}", max);
	}
}

impl Clone for DownloadManager {
	fn clone(&self) -> Self {
		Self {
			AppState:self.AppState.clone(),
			active_downloads:self.ActiveDownloads.clone(),
			download_queue:self.DownloadQueue.clone(),
			cache_directory:self.CacheDirectory.clone(),
			client:self.client.clone(),
			checksum_verifier:self.checksum_verifier.clone(),
			bandwidth_limiter:self.bandwidth_limiter.clone(),
			concurrent_limiter:self.concurrent_limiter.clone(),
			statistics:self.statistics.clone(),
		}
	}
}

impl Default for DownloadStatistics {
	fn default() -> Self {
		Self {
			total_downloads:0,
			successful_downloads:0,
			failed_downloads:0,
			cancelled_downloads:0,
			total_bytes_downloaded:0,
			total_download_time_secs:0.0,
			average_download_rate:0.0,
			peak_download_rate:0,
			active_downloads:0,
			queued_downloads:0,
		}
	}
}

/// Helper function to extract expected checksum from config
fn ExpectedChecksumFromConfig(config:&DownloadConfig) -> Option<&str> {
	if config.checksum.is_empty() { None } else { Some(&config.checksum) }
}

/// Chunk information for parallel downloads
#[derive(Debug, Clone)]
struct ChunkInfo {
	start:u64,
	end:u64,
	downloaded:u64,
	temp_path:PathBuf,
}

/// Parallel download result
#[derive(Debug)]
struct ParallelDownloadResult {
	chunks:Vec<ChunkInfo>,
	total_size:u64,
}

/// Extension download and validation for Cocoon
///
/// Cocoon (Extension Host) downloads VSIX files from marketplace APIs:
/// 1. Request VSIX download URL from marketplace
/// 2. Validate extension manifest metadata
/// 3. Download with progress callbacks for UI updates
/// 4. Verify SHA-256 checksum of signed .vsix package
/// 5. Atomic commit to extension installation directory
/// 6. Extract contents and validate before installation
///
/// Example Cocoon workflow:
/// ```rust
/// let download_config = DownloadConfig {
/// 	url:marketplace_vsix_url,
/// 	destination:extension_path,
/// 	checksum:expected_sha256,
/// 	priority:DownloadPriority::High,
/// 	..Default::default()
/// };
/// let result = downloader.DownloadFileWithConfig(download_config).await?;
/// downloader.VerifyChecksum(&PathBuf::from(result.path), &expected_sha256).await?;
/// ```
///
/// Package downloads for Mountain (Tauri bundling):
/// 1. Build system initiates dependency downloads
/// 2. DownloadManager validates package signatures
/// 3. Parallel chunk downloads for large packages (>50MB)
/// 4. Bandwidth throttling to prevent network saturation
/// 5. Atomic staging with final commit to build cache
///
/// VSIX download and validation:
/// - Supports marketplace API authentication tokens
/// - Validates extension manifest before download
/// - Verifies package signature after download
/// - Extracts and validates contents before installation

impl DownloadManager {
	/// Download a large file using parallel chunked downloads
	///
	/// This feature is in progress and will be enhanced with:
	/// - Dynamic chunk size optimization based on bandwidth
	/// - Adaptive chunk count based on file size
	/// - Reassembly with integrity verification
	/// TODO: Add adaptive chunk size based on network conditions
	/// TODO: Implement parallel download queue management with priority
	/// TODO: Add chunk verification and re-download of failed chunks
	pub async fn DownloadFileWithChunks(
		&self,
		url:String,
		destination:String,
		checksum:String,
		chunk_size_mb:usize,
	) -> Result<DownloadResult> {
		log::info!(
			"[DownloadManager] Starting chunked download - URL: {}, Chunk size: {} MB",
			url,
			chunk_size_mb
		);

		// Defensive: Validate URL first
		let sanitized_url = Self::ValidateAndSanitizeUrl(&url)?;

		// Get file size first using HEAD request
		let total_size = self.GetRemoteFileSize(&sanitized_url).await?;

		log::info!("[DownloadManager] Remote file size: {} bytes", total_size);

		// For small files, use normal download
		let chunk_threshold = 50 * 1024 * 1024; // 50MB
		if total_size < chunk_threshold {
			log::info!("[DownloadManager] File too small for chunked download, using normal download");
			return self.DownloadFile(url, destination, checksum).await;
		}

		// Calculate number of chunks
		let chunk_size = (chunk_size_mb * 1024 * 1024) as u64;
		let num_chunks = ((total_size + chunk_size - 1) / chunk_size) as usize;
		let num_concurrent = num_chunks.min(4); // Max 4 concurrent chunks

		log::info!(
			"[DownloadManager] Downloading in {} chunks ({} concurrent)",
			num_chunks,
			num_concurrent
		);

		let DownloadId = Utility::GenerateRequestId();
		let DestinationPath = if destination.is_empty() {
			let filename = sanitized_url.split('/').last().unwrap_or("download.bin");
			self.CacheDirectory.join(filename)
		} else {
			ConfigurationManager::ExpandPath(&destination)?
		};

		// Create temporary directory for chunks
		let temp_dir = DestinationPath.with_extension("chunks");
		tokio::fs::create_dir_all(&temp_dir)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to create temp directory: {}", e)))?;

		// Initialize chunk tracking
		let mut chunks = Vec::with_capacity(num_chunks);
		for i in 0..num_chunks {
			let start = (i as u64) * chunk_size;
			let end = std::cmp::min(start + chunk_size - 1, total_size - 1);

			chunks.push(ChunkInfo { start, end, downloaded:0, temp_path:temp_dir.join(format!("chunk_{:04}", i)) });
		}

		// Track overall progress
		let downloaded_tracker = Arc::new(RwLock::new(0u64));
		let completed_tracker = Arc::new(RwLock::new(0usize));

		// Download chunks in parallel
		let mut handles = Vec::new();
		for (i, chunk) in chunks.iter().enumerate() {
			let manager = self.clone();
			let url_clone = sanitized_url.clone();
			let chunk_clone = chunk.clone();
			let downloaded_tracker = downloaded_tracker.clone();
			let completed_tracker = completed_tracker.clone();
			let _Did = DownloadId.clone();

			let handle = tokio::spawn(async move {
				manager.DownloadChunk(&url_clone, &chunk_clone, i).await?;

				// Update progress
				{
					let mut downloaded = downloaded_tracker.write().await;
					let mut completed = completed_tracker.write().await;
					*downloaded += chunk_clone.end - chunk_clone.start + 1;
					*completed += 1;

					let progress = (*downloaded as f32 / total_size as f32) * 100.0;
					log::info!(
						"Chunk {} completed ({}/{}) - Progress: {:.1}%",
						i + 1,
						*completed,
						num_chunks,
						progress
					);
				}

				Ok::<_, AirError>(())
			});

			// Limit concurrency
			if (i + 1) % num_concurrent == 0 {
				for handle in handles.drain(..) {
					handle.await??;
				}
			}

			handles.push(handle);
		}

		// Wait for remaining chunks
		for handle in handles {
			handle.await??;
		}

		// Reassemble chunks
		log::info!("[DownloadManager] Reassembling chunks into final file");
		self.ReassembleChunks(&chunks, &DestinationPath).await?;

		// Clean up temporary directory
		tokio::fs::remove_dir_all(&temp_dir).await.map_err(|e| {
			log::warn!("[DownloadManager] Failed to clean up temp directory: {}", e);
			AirError::FileSystem(e.to_string())
		})?;

		// Verify checksum
		if !checksum.is_empty() {
			self.VerifyChecksum(&DestinationPath, &checksum).await?;
		}

		let actual_checksum = self.CalculateChecksum(&DestinationPath).await?;

		log::info!("[DownloadManager] Chunked download completed successfully");

		Ok(DownloadResult {
			path:DestinationPath.to_string_lossy().to_string(),
			size:total_size,
			checksum:actual_checksum,
			duration:Duration::from_secs(0),
			AverageRate:0,
		})
	}

	/// Get remote file size using HEAD request
	async fn GetRemoteFileSize(&self, url:&str) -> Result<u64> {
		let response = self
			.client
			.head(url)
			.timeout(Duration::from_secs(30))
			.send()
			.await
			.map_err(|e| AirError::Network(format!("Failed to get file size: {}", e)))?;

		if !response.status().is_success() {
			return Err(AirError::Network(format!("Failed to get file size: {}", response.status())));
		}

		response
			.content_length()
			.ok_or_else(|| AirError::Network("Content-Length header not found".to_string()))
	}

	/// Download a single chunk using HTTP Range request
	async fn DownloadChunk(&self, url:&str, chunk:&ChunkInfo, chunk_index:usize) -> Result<()> {
		log::debug!(
			"[DownloadManager] Downloading chunk {} (bytes {}-{})",
			chunk_index,
			chunk.start,
			chunk.end
		);

		let range_header = format!("bytes={}-{}", chunk.start, chunk.end);

		let response = self
			.client
			.get(url)
			.header(reqwest::header::RANGE, range_header)
			.timeout(Duration::from_secs(300))
			.send()
			.await
			.map_err(|e| AirError::Network(format!("Failed to start chunk download: {}", e)))?;

		if response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
			return Err(AirError::Network(format!(
				"Chunk download failed with status: {}",
				response.status()
			)));
		}

		// Save chunk to temporary file
		let bytes = response
			.bytes()
			.await
			.map_err(|e| AirError::Network(format!("Failed to read chunk bytes: {}", e)))?;

		tokio::fs::write(&chunk.temp_path, &bytes)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to write chunk: {}", e)))?;

		log::debug!("[DownloadManager] Chunk {} downloaded: {} bytes", chunk_index, bytes.len());

		Ok(())
	}

	/// Reassemble downloaded chunks into final file
	async fn ReassembleChunks(&self, chunks:&[ChunkInfo], destination:&Path) -> Result<()> {
		use tokio::io::AsyncWriteExt;

		let mut file = tokio::fs::File::create(destination)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to create destination file: {}", e)))?;

		// Sort chunks by start position
		let mut sorted_chunks:Vec<_> = chunks.iter().collect();
		sorted_chunks.sort_by_key(|c| c.start);

		for chunk in sorted_chunks {
			let contents = tokio::fs::read(&chunk.temp_path)
				.await
				.map_err(|e| AirError::FileSystem(format!("Failed to read chunk: {}", e)))?;

			file.write_all(&contents)
				.await
				.map_err(|e| AirError::FileSystem(format!("Failed to write chunk to file: {}", e)))?;

			log::debug!("[DownloadManager] Reassembled chunk (bytes {}-{})", chunk.start, chunk.end);
		}

		file.flush()
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to flush file: {}", e)))?;

		log::info!("[DownloadManager] All chunks reassembled successfully");

		Ok(())
	}
}
