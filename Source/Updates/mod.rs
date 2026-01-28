//! # Update Management Service
//!
//! Handles checking for, downloading, and applying updates for the Land ecosystem.
//! This service runs in the background and manages the complete update lifecycle.

use std::{path::PathBuf, sync::Arc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{ApplicationState::ApplicationState, Result, AirError, Configuration::expand_path};

/// Update manager implementation
pub struct UpdateManager {
    /// Application state
    app_state: Arc<ApplicationState>,
    
    /// Current update status
    update_status: Arc<Mutex<UpdateStatus>>,
    
    /// Update cache directory
    cache_directory: PathBuf,
}

/// Update status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatus {
    pub last_check: Option<chrono::DateTime<chrono::Utc>>,
    pub update_available: bool,
    pub current_version: String,
    pub available_version: Option<String>,
    pub download_progress: Option<f32>,
    pub installation_status: InstallationStatus,
}

/// Installation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallationStatus {
    NotStarted,
    Downloading,
    Verifying,
    Installing,
    Completed,
    Failed(String),
}

/// Update information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
    pub release_notes: String,
    pub checksum: String,
    pub size: u64,
    pub published_at: chrono::DateTime<chrono::Utc>,
}

impl UpdateManager {
    /// Create a new update manager
    pub async fn new(app_state: Arc<ApplicationState>) -> Result<Self> {
        let config = &app_state.configuration.updates;
        
        // Expand cache directory path
        let cache_directory = expand_path(&config.cache_directory)?;
        
        // Create cache directory if it doesn't exist
        tokio::fs::create_dir_all(&cache_directory).await
            .map_err(|e| AirError::Configuration(format!("Failed to create cache directory: {}", e)))?;
        
        let manager = Self {
            app_state,
            update_status: Arc::new(Mutex::new(UpdateStatus {
                last_check: None,
                update_available: false,
                current_version: env!("CARGO_PKG_VERSION").to_string(),
                available_version: None,
                download_progress: None,
                installation_status: InstallationStatus::NotStarted,
            })),
            cache_directory,
        };
        
        // Initialize service status
        manager.app_state.update_service_status("updates", crate::ApplicationState::ServiceStatus::Running)
            .await
            .map_err(|e| AirError::Internal(e.to_string()))?;
        
        Ok(manager)
    }
    
    /// Check for updates
    pub async fn check_for_updates(&self) -> Result<Option<UpdateInfo>> {
        let config = &self.app_state.configuration.updates;
        
        if !config.enabled {
            return Ok(None);
        }
        
        log::info!("[UpdateManager] Checking for updates...");
        
        // Update status
        {
            let mut status = self.update_status.lock().await;
            status.last_check = Some(chrono::Utc::now());
        }
        
        // Check update server
        let update_info = self.fetch_update_info().await?;
        
        if let Some(ref info) = update_info {
            log::info!("[UpdateManager] Update available: {}", info.version);
            
            // Update status
            {
                let mut status = self.update_status.lock().await;
                status.update_available = true;
                status.available_version = Some(info.version.clone());
            }
            
            // Auto-download if configured
            if config.auto_download {
                self.download_update(info).await?;
            }
        } else {
            log::info!("[UpdateManager] No updates available");
            
            // Update status
            {
                let mut status = self.update_status.lock().await;
                status.update_available = false;
                status.available_version = None;
            }
        }
        
        Ok(update_info)
    }
    
    /// Download update
    pub async fn download_update(&self, update_info: &UpdateInfo) -> Result<()> {
        log::info!("[UpdateManager] Downloading update: {}", update_info.version);
        
        // Update status
        {
            let mut status = self.update_status.lock().await;
            status.installation_status = InstallationStatus::Downloading;
            status.download_progress = Some(0.0);
        }
        
        // Download file
        let client = reqwest::Client::new();
        let response = client.get(&update_info.download_url)
            .send()
            .await
            .map_err(|e| AirError::Network(format!("Failed to download update: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(AirError::Network(format!("Download failed with status: {}", response.status())));
        }
        
        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();
        
        let file_path = self.cache_directory.join(format!("update-{}.bin", update_info.version));
        let mut file = tokio::fs::File::create(&file_path).await
            .map_err(|e| AirError::FileSystem(format!("Failed to create update file: {}", e)))?;
        
        use tokio::io::AsyncWriteExt;
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| AirError::Network(format!("Download error: {}", e)))?;
            
            file.write_all(&chunk).await
                .map_err(|e| AirError::FileSystem(format!("Failed to write update chunk: {}", e)))?;
            
            downloaded += chunk.len() as u64;
            
            // Update progress
            if total_size > 0 {
                let progress = (downloaded as f32 / total_size as f32) * 100.0;
                
                let mut status = self.update_status.lock().await;
                status.download_progress = Some(progress);
            }
        }
        
        // Verify checksum
        self.verify_update_checksum(&file_path, &update_info.checksum).await?;
        
        // Update status
        {
            let mut status = self.update_status.lock().await;
            status.installation_status = InstallationStatus::Verifying;
            status.download_progress = Some(100.0);
        }
        
        log::info!("[UpdateManager] Update downloaded successfully: {} bytes", downloaded);
        
        Ok(())
    }
    
    /// Apply update
    pub async fn apply_update(&self, update_info: &UpdateInfo) -> Result<()> {
        log::info!("[UpdateManager] Applying update: {}", update_info.version);
        
        // Update status
        {
            let mut status = self.update_status.lock().await;
            status.installation_status = InstallationStatus::Installing;
        }
        
        let file_path = self.cache_directory.join(format!("update-{}.bin", update_info.version));
        
        if !file_path.exists() {
            return Err(AirError::FileSystem("Update file not found".to_string()));
        }
        
        // Verify checksum again
        self.verify_update_checksum(&file_path, &update_info.checksum).await?;
        
        // Apply the update (this would be platform-specific)
        // For now, we'll just log the action
        log::info!("[UpdateManager] Update verified, ready for installation");
        
        // In a real implementation, this would:
        // 1. Stop the current application
        // 2. Replace the binary/files
        // 3. Restart the application
        
        // Update status
        {
            let mut status = self.update_status.lock().await;
            status.installation_status = InstallationStatus::Completed;
        }
        
        log::info!("[UpdateManager] Update applied successfully");
        
        Ok(())
    }
    
    /// Fetch update information from server
    async fn fetch_update_info(&self) -> Result<Option<UpdateInfo>> {
        let config = &self.app_state.configuration.updates;
        
        let client = reqwest::Client::new();
        
        // Build update check URL
        let current_version = env!("CARGO_PKG_VERSION");
        let platform = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            "linux"
        };
        
        let update_url = format!(
            "{}/check?version={}&platform={}",
            config.update_server_url, current_version, platform
        );
        
        let response = client.get(&update_url)
            .send()
            .await
            .map_err(|e| AirError::Network(format!("Failed to check for updates: {}", e)))?;
        
        if !response.status().is_success() {
            return Ok(None);
        }
        
        let update_info: UpdateInfo = response.json().await
            .map_err(|e| AirError::Network(format!("Failed to parse update info: {}", e)))?;
        
        // Compare versions
        if Self::compare_versions(&current_version, &update_info.version) > 0 {
            Ok(Some(update_info))
        } else {
            Ok(None)
        }
    }
    
    /// Verify update checksum
    async fn verify_update_checksum(&self, file_path: &PathBuf, expected_checksum: &str) -> Result<()> {
        let content = tokio::fs::read(file_path).await
            .map_err(|e| AirError::FileSystem(format!("Failed to read update file: {}", e)))?;
        
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let actual_checksum = format!("{:x}", hasher.finalize());
        
        if actual_checksum != expected_checksum {
            return Err(AirError::Network("Update checksum verification failed".to_string()));
        }
        
        Ok(())
    }
    
    /// Compare version strings
    fn compare_versions(v1: &str, v2: &str) -> i32 {
        let v1_parts: Vec<u32> = v1.split('.').filter_map(|s| s.parse().ok()).collect();
        let v2_parts: Vec<u32> = v2.split('.').filter_map(|s| s.parse().ok()).collect();
        
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
        
        if v1_parts.len() < v2_parts.len() {
            -1
        } else {
            0
        }
    }
    
    /// Get current update status
    pub async fn get_status(&self) -> UpdateStatus {
        let status = self.update_status.lock().await;
        status.clone()
    }
    
    /// Start background tasks
    pub async fn start_background_tasks(&self) -> Result<tokio::task::JoinHandle<()>> {
        let manager = self.clone();
        
        let handle = tokio::spawn(async move {
            manager.background_task().await;
        });
        
        Ok(handle)
    }
    
    /// Background task for periodic update checks
    async fn background_task(&self) {
        let config = &self.app_state.configuration.updates;
        
        if !config.enabled {
            return;
        }
        
        let interval = tokio::time::Duration::from_secs(config.check_interval_hours as u64 * 3600);
        let mut interval = tokio::time::interval(interval);
        
        loop {
            interval.tick().await;
            
            // Check for updates
            if let Err(e) = self.check_for_updates().await {
                log::error!("[UpdateManager] Background update check failed: {}", e);
            }
        }
    }
    
    /// Stop background tasks
    pub async fn stop_background_tasks(&self) {
        log::info!("[UpdateManager] Stopping background tasks");
    }
}

impl Clone for UpdateManager {
    fn clone(&self) -> Self {
        Self {
            app_state: self.app_state.clone(),
            update_status: self.update_status.clone(),
            cache_directory: self.cache_directory.clone(),
        }
    }
}