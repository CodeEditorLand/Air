//! Download manager implementation with full resilience and capabilities.
//!
//! Provides a comprehensive, resilient service for downloading files,
//! extensions, dependencies, and packages within the Land ecosystem.
//! Serves Cocoon (Extension Host), Mountain (Tauri Bundling), Air
//! (Background Daemon), and other components.

use std::{
	collections::{HashMap, VecDeque},
	path::{Path, PathBuf},
	sync::Arc,
	time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, Semaphore};

use crate::{
	AirError,
	ApplicationState::ApplicationState::Struct as AppStateStruct,
	Configuration::ConfigurationManager,
	Downloader::RateLimit::TokenBucket,
	Downloader::Types::{
		DownloadConfig, DownloadPriority, DownloadResult, DownloadState, DownloadStatus,
		DownloadStatistics, QueuedDownload,
	},
	Result,
	Utility,
	dev_log,
};

/// Download manager implementation with full resilience and capabilities.
pub struct Struct {
	/// Application state reference
	AppState: Arc<AppStateStruct>,

	/// Active downloads tracking
	ActiveDownloads: Arc<RwLock<HashMap<String, DownloadStatus>>>,

	/// Download queue with priority ordering
	DownloadQueue: Arc<RwLock<VecDeque<QueuedDownload>>>,

	/// Download cache directory
	CacheDirectory: PathBuf,

	/// HTTP client with connection pooling
	client: reqwest::Client,

	/// Checksum verifier helper
	ChecksumVerifier: Arc<crate::Security::ChecksumVerifier::Struct>,

	/// Bandwidth limiter for global control
	BandwidthLimiter: Arc<Semaphore>,

	/// Token bucket for rate limiting
	TokenBucket: Arc<RwLock<TokenBucket>>,

	/// Concurrent download limiter
	ConcurrentLimiter: Arc<Semaphore>,

	/// Download statistics
	statistics: Arc<RwLock<DownloadStatistics>>,
}

/// Chunk information for parallel downloads.
#[derive(Debug, Clone)]
struct ChunkInfo {
	start: u64,
	end: u64,
	downloaded: u64,
	temp_path: PathBuf,
}

/// Parallel download result.
#[derive(Debug)]
struct ParallelDownloadResult {
	chunks: Vec<ChunkInfo>,
	total_size: u64,
}

/// Helper function to extract expected checksum from config.
fn ExpectedChecksumFromConfig(config: &DownloadConfig) -> Option<&str> {
	if config.checksum.is_empty() {
		None
	} else {
		Some(&config.checksum)
	}
}

impl Struct {
	/// Create a new download manager with comprehensive initialization.
	pub async fn New(AppState: Arc<AppStateStruct>) -> Result<Self> {
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
		let dns_port = Mist::dns_port();

		let client = crate::HTTP::Client::secured_client_builder(dns_port)
			.map_err(|e| AirError::Network(format!("Failed to create HTTP client: {}", e)))?
			.timeout(Duration::from_secs(config.DownloadTimeoutSecs))
			.connect_timeout(Duration::from_secs(30))
			.pool_idle_timeout(Duration::from_secs(90))
			.pool_max_idle_per_host(10)
			.tcp_keepalive(Duration::from_secs(60))
			.user_agent("Land-AirDownloader/0.1.0")
			.build()
			.map_err(|e| AirError::Network(format!("Failed to build HTTP client: {}", e)))?;

		// Bandwidth limiter (permit = 1MB of transfer) - kept for global limit
		let BandwidthLimiter = Arc::new(Semaphore::new(100));

		// Token bucket for precise bandwidth throttling (default: 100 MB/s)
		let TokenBucket = Arc::new(RwLock::new(TokenBucket::new(100 * 1024 * 1024, 5.0)));

		// Concurrent download limiter (max 5 parallel downloads)
		let ConcurrentLimiter = Arc::new(Semaphore::new(5));

		let manager = Self {
			AppState,
			ActiveDownloads: Arc::new(RwLock::new(HashMap::new())),
			DownloadQueue: Arc::new(RwLock::new(VecDeque::new())),
			CacheDirectory: CacheDirectoryCloneForInit,
			client,
			ChecksumVerifier: Arc::new(crate::Security::ChecksumVerifier::Struct::New()),
			BandwidthLimiter,
			TokenBucket,
			ConcurrentLimiter,
			statistics: Arc::new(RwLock::new(DownloadStatistics::default())),
		};

		// Initialize service status
		manager
			.AppState
			.UpdateServiceStatus("downloader", crate::ApplicationState::ServiceStatus::ServiceStatus::Running)
			.await
			.map_err(|e| AirError::Internal(e.to_string()))?;

		dev_log!(
			"update",
			"[DownloadManager] Initialized with cache directory: {}",
			CacheDirectory.display()
		);

		Ok(manager)
	}
