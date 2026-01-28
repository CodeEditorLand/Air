//! # Download Manager Service
//!
//! Handles background downloading of files, extensions, and dependencies
//! for the Land ecosystem. Provides resilient downloading with retry logic,
//! progress tracking, and verification.

use std::{collections::HashMap, path::PathBuf, sync::Arc};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use tokio_stream::StreamExt;

use crate::{ApplicationState::ApplicationState, Result, AirError, Configuration::ConfigurationManager};

/// Download manager implementation
pub struct DownloadManager {
    /// Application state
    app_state: Arc<ApplicationState>,
    
    /// Active downloads
    active_downloads: Arc<RwLock<HashMap<String, DownloadStatus>>>,
    
    /// Download cache directory
    cache_directory: PathBuf,
    
    /// HTTP client
    client: reqwest::Client,
}

/// Download status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadStatus {
    pub download_id: String,
    pub url: String,
    pub destination: PathBuf,
    pub total_size: u64,
    pub downloaded: u64,
    pub progress: f32,
    pub status: DownloadState,
    pub error: Option<String>,
}

/// Download state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DownloadState {
    Pending,
    Downloading,
    Verifying,
    Completed,
    Failed,
    Cancelled,
}

/// Download result
#[derive(Debug, Clone)]
pub struct DownloadResult {
    pub path: String,
    pub size: u64,
    pub checksum: String,
}

impl DownloadManager {
    /// Create a new download manager
    pub async fn new(app_state: Arc<ApplicationState>) -> Result<Self> {
        let config = &app_state.configuration.downloader;
        
        // Expand cache directory path
        let cache_directory = ConfigurationManager::expand_path(&config.cache_directory)?;
        
        // Create cache directory if it doesn't exist
        tokio::fs::create_dir_all(&cache_directory).await
            .map_err(|e| AirError::Configuration(format!("Failed to create cache directory: {}", e)))?;
        
        // Create HTTP client with timeout
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.download_timeout_secs))
            .build()
            .map_err(|e| AirError::Network(format!("Failed to create HTTP client: {}", e)))?;
        
        let manager = Self {
            app_state,
            active_downloads: Arc::new(RwLock::new(HashMap::new())),
            cache_directory,
            client,
        };
        
        // Initialize service status
        manager.app_state.update_service_status("downloader", crate::ApplicationState::ServiceStatus::Running)
            .await
            .map_err(|e| AirError::Internal(e.to_string()))?;
        
        Ok(manager)
    }
    
    /// Download a file
    pub async fn download_file(&self, url: String, destination_path: String, checksum: String) -> Result<DownloadResult> {
        let download_id = crate::utils::generate_request_id();
        
        log::info!("[DownloadManager] Starting download [ID: {}] - URL: {}", download_id, url);
        
        // Validate inputs
        if url.is_empty() {
            return Err(AirError::Network("URL cannot be empty".to_string()));
        }
        
        let destination = if destination_path.is_empty() {
            // Generate filename from URL
            let filename = url.split('/').last().unwrap_or("download.bin");
            self.cache_directory.join(filename)
        } else {
            expand_path(&destination_path)?
        };
        
        // Register download
        self.register_download(&download_id, &url, &destination).await?;
        
        // Create destination directory if it doesn't exist
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| AirError::FileSystem(format!("Failed to create destination directory: {}", e)))?;
        }
        
        // Download with retry logic
        let result = self.download_with_retry(&download_id, &url, &destination, &checksum).await;
        
        match result {
            Ok(file_info) => {
                self.update_download_status(&download_id, DownloadState::Completed, Some(100.0), None).await?;
                
                log::info!("[DownloadManager] Download completed [ID: {}] - Size: {} bytes", download_id, file_info.size);
                
                Ok(file_info)
            },
            Err(e) => {
                self.update_download_status(&download_id, DownloadState::Failed, None, Some(e.to_string())).await?;
                
                log::error!("[DownloadManager] Download failed [ID: {}] - Error: {}", download_id, e);
                
                Err(e)
            }
        }
    }
    
    /// Download with retry logic
    async fn download_with_retry(
        &self,
        download_id: &str,
        url: &str,
        destination: &PathBuf,
        checksum: &str,
    ) -> Result<DownloadResult> {
        let config = &self.app_state.configuration.downloader;
        let mut retries = 0;
        
        loop {
            match self.perform_download(download_id, url, destination).await {
                Ok(file_info) => {
                    // Verify checksum if provided
                    if !checksum.is_empty() {
                        self.update_download_status(download_id, DownloadState::Verifying, Some(100.0), None).await?;
                        
                        if let Err(e) = self.verify_checksum(destination, checksum).await {
                            log::warn!("[DownloadManager] Checksum verification failed [ID: {}]: {}", download_id, e);
                            
                            if retries < config.max_retries {
                                retries += 1;
                                log::info!("[DownloadManager] Retrying download [ID: {}] (attempt {}/{})", download_id, retries, config.max_retries);
                                continue;
                            } else {
                                return Err(AirError::Network(format!("Checksum verification failed after {} retries", config.max_retries)));
                            }
                        }
                    }
                    
                    return Ok(file_info);
                },
                Err(e) => {
                    if retries < config.max_retries {
                        retries += 1;
                        log::warn!("[DownloadManager] Download failed [ID: {}], retrying (attempt {}/{}): {}", download_id, retries, config.max_retries, e);
                        
                        // Exponential backoff
                        tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(retries))).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }
    
    /// Perform the actual download
    async fn perform_download(&self, download_id: &str, url: &str, destination: &PathBuf) -> Result<DownloadResult> {
        self.update_download_status(download_id, DownloadState::Downloading, Some(0.0), None).await?;
        
        let response = self.client.get(url)
            .send()
            .await
            .map_err(|e| AirError::Network(format!("Failed to start download: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(AirError::Network(format!("Download failed with status: {}", response.status())));
        }
        
        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();
        
        let mut file = tokio::fs::File::create(destination).await
            .map_err(|e| AirError::FileSystem(format!("Failed to create destination file: {}", e)))?;
        
        use tokio::io::AsyncWriteExt;
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| AirError::Network(format!("Download error: {}", e)))?;
            
            file.write_all(&chunk).await
                .map_err(|e| AirError::FileSystem(format!("Failed to write chunk: {}", e)))?;
            
            downloaded += chunk.len() as u64;
            
            // Update progress
            if total_size > 0 {
                let progress = (downloaded as f32 / total_size as f32) * 100.0;
                self.update_download_status(download_id, DownloadState::Downloading, Some(progress), None).await?;
            }
        }
        
        // Calculate checksum
        let checksum = self.calculate_checksum(destination).await?;
        
        Ok(DownloadResult {
            path: destination.to_string_lossy().to_string(),
            size: downloaded,
            checksum,
        })
    }
    
    /// Verify file checksum
    async fn verify_checksum(&self, file_path: &PathBuf, expected_checksum: &str) -> Result<()> {
        let actual_checksum = self.calculate_checksum(file_path).await?;
        
        if actual_checksum != expected_checksum {
            return Err(AirError::Network("Checksum verification failed".to_string()));
        }
        
        Ok(())
    }
    
    /// Calculate file checksum
    async fn calculate_checksum(&self, file_path: &PathBuf) -> Result<String> {
        let content = tokio::fs::read(file_path).await
            .map_err(|e| AirError::FileSystem(format!("Failed to read file for checksum: {}", e)))?;
        
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let checksum = format!("{:x}", hasher.finalize());
        
        Ok(checksum)
    }
    
    /// Register a new download
    async fn register_download(&self, download_id: &str, url: &str, destination: &PathBuf) -> Result<()> {
        let mut downloads = self.active_downloads.write().await;
        
        downloads.insert(download_id.to_string(), DownloadStatus {
            download_id: download_id.to_string(),
            url: url.to_string(),
            destination: destination.clone(),
            total_size: 0,
            downloaded: 0,
            progress: 0.0,
            status: DownloadState::Pending,
            error: None,
        });
        
        Ok(())
    }
    
    /// Update download status
    async fn update_download_status(
        &self,
        download_id: &str,
        status: DownloadState,
        progress: Option<f32>,
        error: Option<String>,
    ) -> Result<()> {
        let mut downloads = self.active_downloads.write().await;
        
        if let Some(download) = downloads.get_mut(download_id) {
            download.status = status;
            if let Some(progress) = progress {
                download.progress = progress;
            }
            download.error = error;
        }
        
        Ok(())
    }
    
    /// Get download status
    pub async fn get_download_status(&self, download_id: &str) -> Option<DownloadStatus> {
        let downloads = self.active_downloads.read().await;
        downloads.get(download_id).cloned()
    }
    
    /// Cancel a download
    pub async fn cancel_download(&self, download_id: &str) -> Result<()> {
        self.update_download_status(download_id, DownloadState::Cancelled, None, None).await?;
        
        // In a real implementation, this would cancel the actual download
        log::info!("[DownloadManager] Download cancelled [ID: {}]", download_id);
        
        Ok(())
    }
    
    /// Get active download count
    pub async fn get_active_download_count(&self) -> usize {
        let downloads = self.active_downloads.read().await;
        downloads.len()
    }
    
    /// Start background tasks
    pub async fn start_background_tasks(&self) -> Result<tokio::task::JoinHandle<()>> {
        let manager = self.clone();
        
        let handle = tokio::spawn(async move {
            manager.background_task().await;
        });
        
        Ok(handle)
    }
    
    /// Background task for cleanup
    async fn background_task(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60)); // 1 minute
        
        loop {
            interval.tick().await;
            
            // Clean up completed downloads
            self.cleanup_completed_downloads().await;
            
            // Clean up old cache files
            if let Err(e) = self.cleanup_cache().await {
                log::error!("[DownloadManager] Cache cleanup failed: {}", e);
            }
        }
    }
    
    /// Clean up completed downloads
    async fn cleanup_completed_downloads(&self) {
        let mut downloads = self.active_downloads.write().await;
        
        downloads.retain(|_, download| {
            !matches!(download.status, DownloadState::Completed | DownloadState::Failed | DownloadState::Cancelled)
        });
        
        log::debug!("[DownloadManager] Cleaned up completed downloads");
    }
    
    /// Clean up old cache files
    async fn cleanup_cache(&self) -> Result<()> {
        let max_age = chrono::Duration::days(7); // Keep files for 7 days
        let now = chrono::Utc::now();
        
        let mut entries = tokio::fs::read_dir(&self.cache_directory).await
            .map_err(|e| AirError::FileSystem(format!("Failed to read cache directory: {}", e)))?;
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| AirError::FileSystem(format!("Failed to read cache entry: {}", e)))? {
            
            let metadata = entry.metadata().await
                .map_err(|e| AirError::FileSystem(format!("Failed to get file metadata: {}", e)))?;
            
            if metadata.is_file() {
                let modified = metadata.modified()
                    .map_err(|e| AirError::FileSystem(format!("Failed to get modification time: {}", e)))?;
                
                let modified_time = chrono::DateTime::<chrono::Utc>::from(modified);
                
                if now - modified_time > max_age {
                    tokio::fs::remove_file(entry.path()).await
                        .map_err(|e| AirError::FileSystem(format!("Failed to remove old cache file: {}", e)))?;
                    
                    log::debug!("[DownloadManager] Removed old cache file: {}", entry.file_name().to_string_lossy());
                }
            }
        }
        
        Ok(())
    }
    
    /// Stop background tasks
    pub async fn stop_background_tasks(&self) {
        log::info!("[DownloadManager] Stopping background tasks");
    }
}

impl Clone for DownloadManager {
    fn clone(&self) -> Self {
        Self {
            app_state: self.app_state.clone(),
            active_downloads: self.active_downloads.clone(),
            cache_directory: self.cache_directory.clone(),
            client: self.client.clone(),
        }
    }
}
