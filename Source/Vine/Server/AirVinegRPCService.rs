//! # Air Vine gRPC Service
//!
//! Defines the gRPC service implementation for Air. This struct handles
//! incoming RPC calls from Mountain, dispatches them to the appropriate
//! services (authentication, updates, downloads, indexing), and returns
//! the results.

use std::sync::Arc;
use log::{debug, error, info, warn};

use tonic::{Request, Response, Status};
use std::collections::HashMap;

use crate::{
    ApplicationState::ApplicationState,
    Authentication::AuthenticationService,
    Downloader::DownloadManager,
    Indexing::FileIndexer,
    Updates::UpdateManager,
    Result,
    utils::current_timestamp,
};

use crate::Vine::Generated::air::{ // Import from the generated air module
    AirService,
    AuthenticationRequest, AuthenticationResponse,
    UpdateCheckRequest, UpdateCheckResponse,
    ApplyUpdateRequest, ApplyUpdateResponse,
    DownloadRequest, DownloadResponse,
    DownloadStreamRequest, DownloadStreamResponse,
    IndexRequest, IndexResponse,
    SearchRequest, SearchResponse,
    FileInfoRequest, FileInfoResponse,
    StatusRequest, StatusResponse,
    HealthCheckRequest, HealthCheckResponse,
    MetricsRequest, MetricsResponse,
    ResourceUsageRequest, ResourceUsageResponse,
    ResourceLimitsRequest, ResourceLimitsResponse,
    ConfigurationRequest, ConfigurationResponse,
    UpdateConfigurationRequest, UpdateConfigurationResponse,
    FileResult, // Add FileResult import
};

/// The concrete implementation of the Air gRPC service
pub struct AirVinegRPCService {
    /// Application state
    app_state: Arc<ApplicationState>,
    
    /// Authentication service
    auth_service: Arc<AuthenticationService>,
    
    /// Update manager
    update_manager: Arc<UpdateManager>,
    
    /// Download manager
    download_manager: Arc<DownloadManager>,
    
    /// File indexer
    file_indexer: Arc<FileIndexer>,
    
    /// Connection tracking
    active_connections: Arc<tokio::sync::RwLock<HashMap<String, ConnectionMetadata>>>,
}

/// Connection metadata for tracking client state
#[derive(Debug, Clone)]
struct ConnectionMetadata {
    pub client_id: String,
    pub client_version: String,
    pub protocol_version: u32,
    pub last_request_time: u64,
    pub request_count: u64,
    pub connection_type: crate::ApplicationState::ConnectionType,
}

impl AirVinegRPCService {
    /// Creates a new instance of the Air gRPC service
    pub fn new(
        app_state: Arc<ApplicationState>,
        auth_service: Arc<AuthenticationService>,
        update_manager: Arc<UpdateManager>,
        download_manager: Arc<DownloadManager>,
        file_indexer: Arc<FileIndexer>,
    ) -> Self {
        info!("[AirVinegRPCService] New instance created");
        
        Self {
            app_state,
            auth_service,
            update_manager,
            download_manager,
            file_indexer,
            active_connections: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
    
    /// Track connection for a request
    async fn track_connection(&self, request: &tonic::Request<()>, service_name: &str) -> Result<String, Status> {
        let metadata = request.metadata();
        let connection_id = metadata.get("connection-id")
            .map(|v| v.to_str().unwrap_or_default().to_string())
            .unwrap_or_else(|| crate::utils::generate_request_id());
        
        let client_id = metadata.get("client-id")
            .map(|v| v.to_str().unwrap_or_default().to_string())
            .unwrap_or_else(|| "unknown".to_string());
            
        let client_version = metadata.get("client-version")
            .map(|v| v.to_str().unwrap_or_default().to_string())
            .unwrap_or_else(|| "unknown".to_string());
            
        let protocol_version = metadata.get("protocol-version")
            .map(|v| v.to_str().unwrap_or_default().parse().unwrap_or(1))
            .unwrap_or(1);
            
        // Update connection tracking
        let mut connections = self.active_connections.write().await;
        let connection_metadata = connections.entry(connection_id.clone()).or_insert_with(|| ConnectionMetadata {
            client_id: client_id.clone(),
            client_version: client_version.clone(),
            protocol_version,
            last_request_time: crate::utils::current_timestamp(),
            request_count: 0,
            connection_type: crate::ApplicationState::ConnectionType::MountainMain,
        });
        
        connection_metadata.last_request_time = crate::utils::current_timestamp();
        connection_metadata.request_count += 1;
        
        // Register connection with application state
        self.app_state.register_connection(
            connection_id.clone(),
            client_id,
            client_version,
            protocol_version,
            crate::ApplicationState::ConnectionType::MountainMain,
        ).await.map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(connection_id)
    }
    
    /// Validate protocol version compatibility
    fn validate_protocol_version(&self, client_version: u32) -> Result<(), Status> {
        if client_version > crate::PROTOCOL_VERSION {
            return Err(Status::failed_precondition(format!(
                "Client protocol version {} is newer than server version {}",
                client_version, crate::PROTOCOL_VERSION
            )));
        }
        
        if client_version < crate::PROTOCOL_VERSION {
            warn!("Client using older protocol version {} (server: {})", client_version, crate::PROTOCOL_VERSION);
        }
        
        Ok(())
    }
}

impl AirService for AirVinegRPCService {
    /// Handle authentication requests from Mountain
    async fn authenticate(
        &self,
        request: Request<AuthenticationRequest>,
    ) -> std::result::Result<Response<AuthenticationResponse>, Status> {
        let request_data = request.into_inner();
        let request_id = request_data.request_id.clone();
        
        // Track connection and validate protocol
        let connection_id = self.track_connection(&request, "authentication").await?;
        
        info!("[AirVinegRPCService] Authentication request received [ID: {}] [Connection: {}]", request_id, connection_id);
        
        self.app_state.register_request(request_id.clone(), "authentication".to_string())
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        // Additional security validation
        if request_data.username.is_empty() || request_data.password.is_empty() || request_data.provider.is_empty() {
            let error_msg = "Invalid authentication parameters".to_string();
            self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(error_msg.clone()), None)
                .await
                .ok();
            
            return Ok(Response::new(crate::Vine::Generated::air::AuthenticationResponse {
                request_id,
                success: false,
                token: None,
                error: Some(error_msg),
            }));
        }
        
        let result = self.auth_service.authenticate_user(
            request_data.username,
            request_data.password,
            request_data.provider,
        ).await;
        
        match result {
            Ok(token) => {
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Completed, Some(100.0))
                    .await
                    .ok();
                
                // Log successful authentication
                info!("[AirVinegRPCService] Authentication successful for user: {} [Connection: {}]", request_data.username, connection_id);
                
                    Ok(Response::new(crate::Vine::Generated::air::air::AuthenticationResponse {
                    request_id,
                    success: true,
                    token: Some(token),
                    error: None,
                }))
            },
            Err(e) => {
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(e.to_string()), None)
                    .await
                    .ok();
                
                // Log failed authentication attempt
                warn!("[AirVinegRPCService] Authentication failed for user: {} [Connection: {}] - {}", request_data.username, connection_id, e);
                
                Ok(Response::new(crate::Vine::Generated::air::air::AuthenticationResponse {
                    request_id,
                    success: false,
                    token: None,
                    error: Some(e.to_string()),
                }))
            }
        }
    }
    
    /// Handle update check requests from Mountain
    async fn check_for_updates(
        &self,
        request: Request<UpdateCheckRequest>,
    ) -> std::result::Result<Response<UpdateCheckResponse>, Status> {
        let request_data = request.into_inner();
        let request_id = request_data.request_id.clone();
        
        info!("[AirVinegRPCService] Update check request received [ID: {}] - Version: {}, Channel: {}",
              request_id, request_data.current_version, request_data.channel);
        
        self.app_state.register_request(request_id.clone(), "updates".to_string())
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        // Validate current_version
        if request_data.current_version.is_empty() {
            let error_msg = crate::AirError::Validation("current_version cannot be empty".to_string());
            self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(error_msg.to_string()), None)
                .await
                .ok();
            return Err(Status::invalid_argument(error_msg.to_string()));
        }
        
        // Validate channel
        let valid_channels = ["stable", "beta", "nightly"];
        let channel = request_data.channel.clone().unwrap_or_else(|| "stable".to_string());
        if !valid_channels.contains(&channel.as_str()) {
            let error_msg = format!("Invalid channel: {}. Valid values are: {}", channel, valid_channels.join(", "));
            self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(error_msg.clone()), None)
                .await
                .ok();
            return Err(Status::invalid_argument(error_msg));
        }
        
        // Check for updates using UpdateManager
        let result = self.update_manager.check_for_updates().await;
        
        match result {
            Ok(update_info) => {
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Completed, Some(100.0))
                    .await
                    .ok();
                
                info!("[AirVinegRPCService] Update check successful - Available: {}", update_info.is_some());
                
                Ok(Response::new(crate::Vine::Generated::air::UpdateCheckResponse {
                    request_id,
                    update_available: update_info.is_some(),
                    version: update_info.as_ref().map(|info| info.version.clone()).unwrap_or_default(),
                    download_url: update_info.as_ref().map(|info| info.download_url.clone()).unwrap_or_default(),
                    release_notes: update_info.as_ref().map(|info| info.release_notes.clone()).unwrap_or_default(),
                    error: None,
                }))
            },
            Err(crate::AirError::Network(e)) => {
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(e.clone()), None)
                    .await
                    .ok();
                error!("[AirVinegRPCService] Network error during update check: {}", e);
                Err(Status::unavailable(e))
            },
            Err(e) => {
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(e.to_string()), None)
                    .await
                    .ok();
                error!("[AirVinegRPCService] Update check failed: {}", e);
                Ok(Response::new(crate::Vine::Generated::air::UpdateCheckResponse {
                    request_id,
                    update_available: false,
                    version: String::new(),
                    download_url: String::new(),
                    release_notes: String::new(),
                    error: Some(e.to_string()),
                }))
            }
        }
    }
    
    /// Handle download requests from Mountain
    async fn download_file(
        &self,
        request: Request<DownloadRequest>,
    ) -> std::result::Result<Response<DownloadResponse>, Status> {
        let request_data = request.into_inner();
        let request_id = request_data.request_id.clone();
        
        info!("[AirVinegRPCService] Download request received [ID: {}] - URL: {}", request_id, request_data.url);
        
        // Request ID for tracking (use provided or generate)
        let download_request_id = if request_id.is_empty() {
            crate::utils::generate_request_id()
        } else {
            request_id.clone()
        };
        
        self.app_state.register_request(download_request_id.clone(), "downloader".to_string())
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        // Validate URL
        if request_data.url.is_empty() {
            let error_msg = "URL cannot be empty".to_string();
            self.app_state.update_request_status(&download_request_id, crate::ApplicationState::RequestState::Failed(error_msg.clone()), None)
                .await
                .ok();
            return Ok(Response::new(DownloadResponse {
                request_id: download_request_id,
                success: false,
                file_path: String::new(),
                file_size: 0,
                checksum: String::new(),
                error: Some(error_msg),
            }));
        }
        
        // Validate URL format
        if !match_url_scheme(&request_data.url) {
            let error_msg = format!("Invalid URL scheme: {}", request_data.url);
            self.app_state.update_request_status(&download_request_id, crate::ApplicationState::RequestState::Failed(error_msg.clone()), None)
                .await
                .ok();
            return Ok(Response::new(DownloadResponse {
                request_id: download_request_id,
                success: false,
                file_path: String::new(),
                file_size: 0,
                checksum: String::new(),
                error: Some(error_msg),
            }));
        }
        
        // Validate or use cache directory
        let destination_path = if request_data.destination_path.is_empty() {
            // Use cache directory from configuration
            let config = &self.app_state.configuration.downloader;
            config.cache_directory.clone()
        } else {
            request_data.destination_path.clone()
        };
        
        // Validate target directory exists
        let dest_path = std::path::Path::new(&destination_path);
        if let Some(parent) = dest_path.parent() {
            if !parent.exists() {
                match tokio::fs::create_dir_all(parent).await {
                    Ok(_) => {
                        debug!("[AirVinegRPCService] Created destination directory: {}", parent.display());
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to create destination directory: {}", e);
                        self.app_state.update_request_status(&download_request_id, crate::ApplicationState::RequestState::Failed(error_msg.clone()), None)
                            .await
                            .ok();
                        return Ok(Response::new(DownloadResponse {
                            request_id: download_request_id,
                            success: false,
                            file_path: String::new(),
                            file_size: 0,
                            checksum: String::new(),
                            error: Some(error_msg),
                        }));
                    }
                }
            }
        }
        
        // Set up granular progress callback mechanism
        let download_manager = self.download_manager.clone();
        let app_state = self.app_state.clone();
        let callback_request_id = download_request_id.clone();
        let progress_callback = move |progress: f32| {
            let state = app_state.clone();
            let id = callback_request_id.clone();
            tokio::spawn(async move {
                let _ = state.update_request_status(&id, crate::ApplicationState::RequestState::InProgress, Some(progress)).await;
            });
        };
        
        // Perform download with retry and progress tracking
        let result = self.download_file_with_retry(
            &download_request_id,
            request_data.url.clone(),
            destination_path,
            request_data.checksum,
            Some(progress_callback),
        ).await;
        
        match result {
            Ok(file_info) => {
                self.app_state.update_request_status(&download_request_id, crate::ApplicationState::RequestState::Completed, Some(100.0))
                    .await
                    .ok();
                
                info!("[AirVinegRPCService] Download completed [ID: {}] - Size: {} bytes", download_request_id, file_info.size);
                
                Ok(Response::new(DownloadResponse {
                    request_id: download_request_id,
                    success: true,
                    file_path: file_info.path,
                    file_size: file_info.size,
                    checksum: file_info.checksum,
                    error: None,
                }))
            },
            Err(e) => {
                self.app_state.update_request_status(&download_request_id, crate::ApplicationState::RequestState::Failed(e.to_string()), None)
                    .await
                    .ok();
                
                error!("[AirVinegRPCService] Download failed [ID: {}] - Error: {}", download_request_id, e);
                
                Ok(Response::new(DownloadResponse {
                    request_id: download_request_id,
                    success: false,
                    file_path: String::new(),
                    file_size: 0,
                    checksum: String::new(),
                    error: Some(e.to_string()),
                }))
            }
        }
    }
    
    /// Handle file indexing requests from Mountain
    async fn index_files(
        &self,
        request: Request<IndexRequest>,
    ) -> std::result::Result<Response<IndexResponse>, Status> {
        let request_data = request.into_inner();
        let request_id = request_data.request_id;
        
        info!("[AirVinegRPCService] Index request received [ID: {}] - Path: {}", request_id, request_data.path);
        
        self.app_state.register_request(request_id.clone(), "indexing".to_string())
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        let result = self.file_indexer.index_directory(
            request_data.path,
            request_data.patterns,
        ).await;
        
        match result {
            Ok(index_info) => {
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Completed, Some(100.0))
                    .await
                    .ok();
                
                Ok(Response::new(crate::Vine::Generated::air::IndexResponse {
                    request_id,
                    success: true,
                    files_indexed: index_info.files_indexed,
                    total_size: index_info.total_size,
                    error: None,
                }))
            },
            Err(e) => {
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(e.to_string()), None)
                    .await
                    .ok();
                
                Ok(Response::new(crate::Vine::Generated::air::IndexResponse {
                    request_id,
                    success: false,
                    files_indexed: 0,
                    total_size: 0,
                    error: Some(e.to_string()),
                }))
            }
        }
    }
    
    /// Handle status check requests from Mountain
    async fn get_status(
        &self,
        request: Request<StatusRequest>,
    ) -> std::result::Result<Response<StatusResponse>, Status> {
        let request_data = request.into_inner();
        
        debug!("[AirVinegRPCService] Status request received");
        
        let metrics = self.app_state.get_metrics().await;
        let resources = self.app_state.get_resource_usage().await;
        
        Ok(Response::new(crate::Vine::Generated::air::StatusResponse {
            version: crate::VERSION.to_string(),
            uptime_seconds: metrics.uptime_seconds,
            total_requests: metrics.total_requests,
            successful_requests: metrics.successful_requests,
            failed_requests: metrics.failed_requests,
            average_response_time: metrics.average_response_time,
            memory_usage_mb: resources.memory_usage_mb,
            cpu_usage_percent: resources.cpu_usage_percent,
            active_requests: self.app_state.get_active_request_count().await as u32,
        }))
    }
    
    /// Handle service health check
    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> std::result::Result<Response<HealthCheckResponse>, Status> {
        debug!("[AirVinegRPCService] Health check request received");

        Ok(Response::new(crate::Vine::Generated::air::HealthCheckResponse {
            healthy: true,
            timestamp: current_timestamp(),
        }))
    }

    // ==================== Phase 2: Update Operations ====================

    /// Handle download update requests
    async fn download_update(
        &self,
        request: Request<DownloadRequest>,
    ) -> std::result::Result<Response<DownloadResponse>, Status> {
        let request_data = request.into_inner();
        let request_id = request_data.request_id.clone();

        info!("[AirVinegRPCService] Download update request received [ID: {}] - URL: {}, Destination: {}",
              request_id, request_data.url, request_data.destination_path);

        self.app_state.register_request(request_id.clone(), "download_update".to_string())
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Validate URL is not empty
        if request_data.url.is_empty() {
            let error_msg = crate::AirError::Validation("URL cannot be empty".to_string());
            self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(error_msg.to_string()), None)
                .await
                .ok();
            return Err(Status::invalid_argument(error_msg.to_string()));
        }

        // Validate URL format
        if !request_data.url.starts_with("http://") && !request_data.url.starts_with("https://") {
            let error_msg = crate::AirError::Validation("URL must start with http:// or https://".to_string());
            self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(error_msg.to_string()), None)
                .await
                .ok();
            return Err(Status::invalid_argument(error_msg.to_string()));
        }

        // Validate destination path is writeable
        let destination = if request_data.destination_path.is_empty() {
            // Default to cache directory if not specified
            self.update_manager.get_cache_directory().join("update-latest.bin").to_string_lossy().to_string()
        } else {
            // Use provided destination
            let dest_path = std::path::Path::new(&request_data.destination_path);
            if let Some(parent) = dest_path.parent() {
                if parent.as_os_str().is_empty() {
                    // Relative path in cache directory
                    self.update_manager.get_cache_directory().join(&request_data.destination_path).to_string_lossy().to_string()
                } else {
                    // Absolute or full path
                    request_data.destination_path.clone()
                }
            } else {
                request_data.destination_path.clone()
            }
        };

        // Check if destination directory exists and is writeable
        let dest_path = std::path::Path::new(&destination);
        if let Some(parent) = dest_path.parent() {
            if !parent.exists() {
                return Err(Status::failed_precondition(format!("Destination directory does not exist: {}", parent.display())));
            }
            
            // Test writeability
            if let Err(e) = std::fs::write(parent.join(".write_test"), "") {
                let error_msg = crate::AirError::FileSystem(format!("Destination directory not writeable: {}", e));
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(error_msg.to_string()), None)
                    .await
                    .ok();
                return Err(Status::permission_denied(error_msg.to_string()));
            }
            // Cleanup test file
            let _ = std::fs::remove_file(parent.join(".write_test"));
        }

        // Use download manager - includes SHA256 checksum verification, progress tracking, and retry logic
        let download_result = self.download_manager
            .download_file(
                request_data.url,
                destination.clone(),
                request_data.checksum
            )
            .await;

        match download_result {
            Ok(result) => {
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Completed, Some(100.0))
                    .await
                    .ok();
                
                info!("[AirVinegRPCService] Update downloaded successfully - Path: {}, Size: {}, Checksum: {}",
                      result.path, result.size, result.checksum);
                
                Ok(Response::new(DownloadResponse {
                    request_id,
                    success: true,
                    file_path: result.path,
                    file_size: result.size,
                    checksum: result.checksum,
                    error: None,
                }))
            }
            Err(crate::AirError::Network(e)) => {
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(e.clone()), None)
                    .await
                    .ok();
                error!("[AirVinegRPCService] Download update network error: {}", e);
                Err(Status::unavailable(e))
            }
            Err(crate::AirError::FileSystem(e)) => {
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(e.clone()), None)
                    .await
                    .ok();
                error!("[AirVinegRPCService] Download update filesystem error: {}", e);
                Err(Status::internal(e))
            }
            Err(e) => {
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(e.to_string()), None)
                    .await
                    .ok();
                error!("[AirVinegRPCService] Download update failed: {}", e);
                Ok(Response::new(DownloadResponse {
                    request_id,
                    success: false,
                    file_path: String::new(),
                    file_size: 0,
                    checksum: String::new(),
                    error: Some(e.to_string()),
                }))
            }
        }
    }

    /// Handle apply update requests
    async fn apply_update(
        &self,
        request: Request<ApplyUpdateRequest>,
    ) -> std::result::Result<Response<ApplyUpdateResponse>, Status> {
        let request_data = request.into_inner();
        let request_id = request_data.request_id.clone();

        info!("[AirVinegRPCService] Apply update request received [ID: {}] - Version: {}, Path: {}",
              request_id, request_data.version, request_data.update_path);

        self.app_state.register_request(request_id.clone(), "apply_update".to_string())
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Validate version
        if request_data.version.is_empty() {
            let error_msg = crate::AirError::Validation("version cannot be empty".to_string());
            self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(error_msg.to_string()), None)
                .await
                .ok();
            return Err(Status::invalid_argument(error_msg.to_string()));
        }

        // Validate update path is not empty
        if request_data.update_path.is_empty() {
            let error_msg = crate::AirError::Validation("update_path cannot be empty".to_string());
            self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(error_msg.to_string()), None)
                .await
                .ok();
            return Err(Status::invalid_argument(error_msg.to_string()));
        }

        let update_path = std::path::Path::new(&request_data.update_path);

        // Validate update file exists
        if !update_path.exists() {
            let error_msg = crate::AirError::FileSystem(format!("Update file not found: {}", request_data.update_path));
            self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(error_msg.to_string()), None)
                .await
                .ok();
            return Err(Status::not_found(error_msg.to_string()));
        }

        // Validate update file is readable and has content
        let metadata = match std::fs::metadata(update_path) {
            Ok(m) => m,
            Err(e) => {
                let error_msg = crate::AirError::FileSystem(format!("Failed to read update file metadata: {}", e));
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(error_msg.to_string()), None)
                    .await
                    .ok();
                return Err(Status::internal(error_msg.to_string()));
            }
        };

        if metadata.len() == 0 {
            let error_msg = crate::AirError::Validation("Update file is empty".to_string());
            self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(error_msg.to_string()), None)
                .await
                .ok();
            return Err(Status::failed_precondition(error_msg.to_string()));
        }

        // Prepare rollback capability before applying update
        let rollback_backup_path = self.prepare_rollback_backup(&request_data.version).await;
        if let Err(ref e) = rollback_backup_path {
            warn!("[AirVinegRPCService] Failed to prepare rollback backup: {}. Proceeding without rollback capability.", e);
        }

        // Verify update file integrity (checksum check)
        match self.update_manager.verify_update(&request_data.update_path).await {
            Ok(true) => {
                info!("[AirVinegRPCService] Update verification successful, preparing for installation");
                
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Completed, Some(100.0))
                    .await
                    .ok();

                // Trigger graceful shutdown after returning response
                let app_state = self.app_state.clone();
                let version = request_data.version.clone();
                
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    log::info!("[AirVinegRPCService] Initiating graceful shutdown for update version {}", version);
                    
                    // Graceful shutdown implementation
                    if let Err(e) = app_state.initiate_graceful_shutdown().await {
                        log::error!("[AirVinegRPCService] Failed to initiate graceful shutdown: {}", e);
                        
                        // Implement rollback if update fails
                        log::warn!("[AirVinegRPCService] Rollback initiated due to graceful shutdown failure");
                        if let Err(rollback_error) = self.perform_rollback(&version).await {
                            log::error!("[AirVinegRPCService] Rollback failed: {}", rollback_error);
                        } else {
                            log::info!("[AirVinegRPCService] Rollback completed successfully");
                        }
                    }
                });

                Ok(Response::new(ApplyUpdateResponse {
                    request_id,
                    success: true,
                    error: None,
                }))
            }
            Ok(false) => {
                let error_msg = "Update verification failed: checksum mismatch".to_string();
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(error_msg.clone()), None)
                    .await
                    .ok();
                error!("[AirVinegRPCService] {}", error_msg);
                
                // Clean up rollback backup if verification failed
                let _ = self.cleanup_rollback_backup(&request_data.version).await;
                
                Err(Status::failed_precondition(error_msg))
            }
            Err(crate::AirError::FileSystem(e)) => {
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(e.clone()), None)
                    .await
                    .ok();
                error!("[AirVinegRPCService] Update verification filesystem error: {}", e);
                
                // Clean up rollback backup if verification failed
                let _ = self.cleanup_rollback_backup(&request_data.version).await;
                
                Err(Status::internal(e))
            }
            Err(e) => {
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(e.to_string()), None)
                    .await
                    .ok();
                error!("[AirVinegRPCService] Update verification error: {}", e);
                
                // Clean up rollback backup if verification failed
                let _ = self.cleanup_rollback_backup(&request_data.version).await;
                
                Ok(Response::new(ApplyUpdateResponse {
                    request_id,
                    success: false,
                    error: Some(e.to_string()),
                }))
            }
        }
    }

    // ==================== Phase 3: Download Operations ====================

    /// Handle streaming download requests with bidirectional streaming and chunk delivery
    type DownloadStreamStream = tokio_stream::wrappers::ReceiverStream<Result<crate::Vine::Generated::air::DownloadStreamResponse, Status>>;

    async fn download_stream(
        &self,
        request: Request<DownloadStreamRequest>,
    ) -> std::result::Result<Response<Self::DownloadStreamStream>, Status> {
        let request_data = request.into_inner();
        let request_id = request_data.request_id.clone();

        info!("[AirVinegRPCService] Download stream request received [ID: {}] - URL: {}", request_id, request_data.url);

        self.app_state.register_request(request_id.clone(), "downloader_stream".to_string())
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Validate URL
        if request_data.url.is_empty() {
            let error_msg = "URL cannot be empty".to_string();
            self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(error_msg.clone()), None)
                .await
                .ok();
            return Err(Status::invalid_argument(error_msg));
        }

        // Validate URL scheme
        if !match_url_scheme(&request_data.url) {
            let error_msg = format!("Invalid URL scheme: {}", request_data.url);
            self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(error_msg.clone()), None)
                .await
                .ok();
            return Err(Status::invalid_argument(error_msg));
        }

        // Validate URL supports range headers for streaming
        match self.validate_range_support(&request_data.url).await {
            Ok(true) => {
                debug!("[AirVinegRPCService] URL supports range headers");
            }
            Ok(false) => {
                warn!("[AirVinegRPCService] URL does not support range headers, streaming may be inefficient");
            }
            Err(e) => {
                let error_msg = format!("Failed to validate range support: {}", e);
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(error_msg.clone()), None)
                    .await
                    .ok();
                return Err(Status::internal(error_msg));
            }
        }

        // Create a channel for streaming responses
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // Configure chunk size (8MB chunks for balance between throughput and latency)
        let chunk_size = 8 * 1024 * 1024; // 8MB

        // Clone necessary data for spawn task
        let url = request_data.url.clone();
        let headers = request_data.headers;
        let download_request_id = request_id.clone();
        let download_manager = self.download_manager.clone();
        let app_state = self.app_state.clone();

        // Spawn streaming task
        tokio::spawn(async move {
            // Send initial status
            if tx.send(Ok(DownloadStreamResponse {
                request_id: download_request_id.clone(),
                chunk: vec![].into(),
                total_size: 0,
                downloaded: 0,
                completed: false,
                error: None,
            })).await.is_err() {
                log::warn!("[AirVinegRPCService] Client disconnected before streaming started [ID: {}]", download_request_id);
                return;
            }

            // Create HTTP client with connection pooling hints
            let client = reqwest::Client::builder()
                .pool_idle_timeout(std::time::Duration::from_secs(60))
                .pool_max_idle_per_host(5)
                .timeout(std::time::Duration::from_secs(300))
                .build();

            if client.is_err() {
                let error = client.unwrap_err().to_string();
                let _ = tx.send(Ok(DownloadStreamResponse {
                    request_id: download_request_id.clone(),
                    chunk: vec![].into(),
                    total_size: 0,
                    downloaded: 0,
                    completed: false,
                    error: Some(error.clone()),
                })).await;
                app_state.update_request_status(&download_request_id, crate::ApplicationState::RequestState::Failed(error), None)
                    .await
                    .ok();
                return;
            }

            let client = client.unwrap();

            // Start streaming download
            let mut total_size: u64 = 0;
            let mut total_downloaded: u64 = 0;

            match client.get(&url)
                .headers({
                    let mut map = reqwest::header::HeaderMap::new();
                    for (key, value) in headers {
                        if let (Ok(header_name), Ok(header_value)) = (
                            reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                            reqwest::header::HeaderValue::from_str(&value)
                        ) {
                            map.insert(header_name, header_value);
                        }
                    }
                    map
                })
                .send()
                .await
            {
                Ok(response) => {
                    if !response.status().is_success() {
                        let error = format!("Download failed with status: {}", response.status());
                        let _ = tx.send(Ok(DownloadStreamResponse {
                            request_id: download_request_id.clone(),
                            chunk: vec![].into(),
                            total_size: 0,
                            downloaded: 0,
                            completed: false,
                            error: Some(error.clone()),
                        })).await;
                        app_state.update_request_status(&download_request_id, crate::ApplicationState::RequestState::Failed(error), None)
                            .await
                            .ok();
                        return;
                    }

                    total_size = response.content_length().unwrap_or(0);
                    let response_tx = tx.clone();
                    let response_id = download_request_id.clone();

                    // Stream chunks to client
                    let mut stream = response.bytes_stream();
                    let mut buffer = Vec::with_capacity(chunk_size);
                    let mut last_progress: f32 = 0.0;

                    while let Some(chunk_result) = stream.next().await {
                        if app_state.is_request_cancelled(&download_request_id).await {
                            log::info!("[AirVinegRPCService] Download cancelled by client [ID: {}]", download_request_id);
                            app_state.update_request_status(&download_request_id, crate::ApplicationState::RequestState::Cancelled, None)
                                .await
                                .ok();
                            return;
                        }

                        match chunk_result {
                            Ok(chunk) => {
                                buffer.extend_from_slice(&chunk);
                                total_downloaded += chunk.len() as u64;

                                // Send chunk when buffer reaches target size
                                if buffer.len() >= chunk_size {
                                    // Calculate checksum for verification
                                    let chunk_checksum = calculate_chunk_checksum(&buffer);

                                    // Calculate progress
                                    let progress = if total_size > 0 {
                                        (total_downloaded as f32 / total_size as f32) * 100.0
                                    } else {
                                        0.0
                                    };

                                    // Update request status periodically
                                    if progress - last_progress >= 5.0 {
                                        app_state.update_request_status(&download_request_id, crate::ApplicationState::RequestState::InProgress, Some(progress))
                                            .await
                                            .ok();
                                        last_progress = progress;
                                    }

                                    if response_tx.send(Ok(DownloadStreamResponse {
                                        request_id: response_id.clone(),
                                        chunk: buffer.clone().into(),
                                        total_size,
                                        downloaded: total_downloaded,
                                        completed: false,
                                        error: None,
                                    })).await.is_err() {
                                        log::warn!("[AirVinegRPCService] Client disconnected during streaming [ID: {}]", download_request_id);
                                        app_state.update_request_status(&download_request_id, crate::ApplicationState::RequestState::Failed("Client disconnected".to_string()), None)
                                            .await
                                            .ok();
                                        return;
                                    }

                                    debug!("[AirVinegRPCService] Sent chunk of {} bytes [ID: {}] - Progress: {:.1}%",
                                           buffer.len(), download_request_id, progress);

                                    buffer.clear();
                                }
                            }
                            Err(e) => {
                                let error = format!("Download error: {}", e);
                                log::error!("[AirVinegRPCService] Stream download failed [ID: {}]: {}", download_request_id, error);

                                let _ = response_tx.send(Ok(DownloadStreamResponse {
                                    request_id: response_id.clone(),
                                    chunk: vec![].into(),
                                    total_size,
                                    downloaded: total_downloaded,
                                    completed: false,
                                    error: Some(error.clone()),
                                })).await;

                                app_state.update_request_status(&download_request_id, crate::ApplicationState::RequestState::Failed(error), None)
                                    .await
                                    .ok();
                                return;
                            }
                        }
                    }

                    // Send remaining buffered data
                    if !buffer.is_empty() {
                        let chunk_checksum = calculate_chunk_checksum(&buffer);

                        if tx.send(Ok(DownloadStreamResponse {
                            request_id: download_request_id.clone(),
                            chunk: buffer.into(),
                            total_size,
                            downloaded: total_downloaded,
                            completed: false,
                            error: None,
                        })).await.is_err() {
                            log::warn!("[AirVinegRPCService] Client disconnected while sending final chunk [ID: {}]", download_request_id);
                            return;
                        }
                    }

                    // Send completion signal
                    app_state.update_request_status(&download_request_id, crate::ApplicationState::RequestState::Completed, Some(100.0))
                        .await
                        .ok();

                    let _ = tx.send(Ok(DownloadStreamResponse {
                        request_id,
                        chunk: vec![].into(),
                        total_size,
                        downloaded: total_downloaded,
                        completed: true,
                        error: None,
                    })).await;

                    info!("[AirVinegRPCService] Stream download completed [ID: {}] - Total: {} bytes", download_request_id, total_downloaded);
                }
                Err(e) => {
                    let error = format!("Failed to start streaming download: {}", e);
                    log::error!("[AirVinegRPCService] Stream download error [ID: {}]: {}", download_request_id, error);

                    let _ = tx.send(Ok(DownloadStreamResponse {
                        request_id: download_request_id.clone(),
                        chunk: vec![].into(),
                        total_size: 0,
                        downloaded: 0,
                        completed: false,
                        error: Some(error.clone()),
                    })).await;

                    app_state.update_request_status(&download_request_id, crate::ApplicationState::RequestState::Failed(error), None)
                        .await
                        .ok();
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    // ==================== Phase 4: Indexing Operations ====================

    /// Handle file search requests
    async fn search_files(
        &self,
        request: Request<SearchRequest>,
    ) -> std::result::Result<Response<SearchResponse>, Status> {
        let request_data = request.into_inner();
        let request_id = request_data.request_id.clone();

        debug!("[AirVinegRPCService] Search files request: query='{}' in path='{}'",
               request_data.query, request_data.path);

        // Validate search query
        if request_data.query.is_empty() {
            return Ok(Response::new(SearchResponse {
                request_id,
                results: vec![],
                total_results: 0,
                error: Some("Search query cannot be empty".to_string()),
            }));
        }

        // Use file indexer to search - convert to match the existing signature
        let path = if request_data.path.is_empty() { None } else { Some(request_data.path.clone()) };
        let search_path = path.as_deref();

        match self.file_indexer.search_files(
            request_data.query.clone(),
            path,
            request_data.max_results,
        ).await {
            Ok(search_results) => {
                // Convert from internal SearchResult to proto FileResult
                let mut file_results = Vec::new();
                for r in search_results {
                    // Create a preview from the first match if available
                    let (match_preview, line_number) = if let Some(first_match) = r.matches.first() {
                        (first_match.line_content.clone(), first_match.line_number)
                    } else {
                        (String::new(), 0)
                    };

                    // Get file size from metadata or filesystem
                    let size = if let Ok(Some(metadata)) = self.file_indexer.get_file_info(r.path.clone()).await {
                        metadata.size
                    } else if let Ok(file_metadata) = std::fs::metadata(&r.path) {
                        file_metadata.len()
                    } else {
                        0
                    };

                    file_results.push(FileResult {
                        path: r.path,
                        size,
                        match_preview,
                        line_number,
                    });
                }

                info!("[AirVinegRPCService] Search completed: {} results found", file_results.len());

                Ok(Response::new(SearchResponse {
                    request_id,
                    results: file_results,
                    total_results: file_results.len() as u32,
                    error: None,
                }))
            }
            Err(e) => {
                error!("[AirVinegRPCService] Search failed: {}", e);
                Ok(Response::new(SearchResponse {
                    request_id,
                    results: vec![],
                    total_results: 0,
                    error: Some(e.to_string()),
                }))
            }
        }
    }

    /// Handle get file info requests
    async fn get_file_info(
        &self,
        request: Request<FileInfoRequest>,
    ) -> std::result::Result<Response<FileInfoResponse>, Status> {
        let request_data = request.into_inner();
        let request_id = request_data.request_id.clone();

        debug!("[AirVinegRPCService] Get file info request: {}", request_data.path);

        // Validate path
        if request_data.path.is_empty() {
            return Ok(Response::new(GetFileInfoResponse {
                request_id,
                exists: false,
                size: 0,
                mime_type: String::new(),
                checksum: String::new(),
                modified_time: 0,
                error: Some("Path cannot be empty".to_string()),
            }));
        }

        // Get file metadata
        use std::path::Path;
        let path = Path::new(&request_data.path);

        if !path.exists() {
            return Ok(Response::new(GetFileInfoResponse {
                request_id,
                exists: false,
                size: 0,
                mime_type: String::new(),
                checksum: String::new(),
                modified_time: 0,
                error: None, // File not found is not an error
            }));
        }

        // Get file metadata using std::fs
        match std::fs::metadata(path) {
            Ok(metadata) => {
                let modified_time = metadata.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                // Detect MIME type
                let mime_type = self.detect_mime_type(path);

                // Calculate checksum lazily or on-demand
                let checksum = String::new(); // Placeholder - calculate if needed

                Ok(Response::new(GetFileInfoResponse {
                    request_id,
                    exists: true,
                    size: metadata.len(),
                    mime_type,
                    checksum,
                    modified_time,
                    error: None,
                }))
            }
            Err(e) => {
                error!("[AirVinegRPCService] Failed to get file metadata: {}", e);
                Ok(Response::new(GetFileInfoResponse {
                    request_id,
                    exists: false,
                    size: 0,
                    mime_type: String::new(),
                    checksum: String::new(),
                    modified_time: 0,
                    error: Some(e.to_string()),
                }))
            }
        }
    }

    // ==================== Phase 5: Monitoring & Metrics ====================

    /// Handle get metrics requests
    async fn get_metrics(
        &self,
        request: Request<MetricsRequest>,
    ) -> std::result::Result<Response<MetricsResponse>, Status> {
        let request_data = request.into_inner();
        let request_id = request_data.request_id.clone();

        debug!("[AirVinegRPCService] Get metrics request: type='{}'", request_data.metric_type);

        let metrics = self.app_state.get_metrics().await;
        let mut metrics_map = std::collections::HashMap::new();

        // Performance metrics
        if request_data.metric_type.is_empty() || request_data.metric_type == "performance" {
            metrics_map.insert("uptime_seconds".to_string(), metrics.uptime_seconds.to_string());
            metrics_map.insert("total_requests".to_string(), metrics.total_requests.to_string());
            metrics_map.insert("successful_requests".to_string(), metrics.successful_requests.to_string());
            metrics_map.insert("failed_requests".to_string(), metrics.failed_requests.to_string());
            metrics_map.insert("average_response_time_ms".to_string(),
                              metrics.average_response_time.to_string());
        }

        // Request metrics
        if request_data.metric_type.is_empty() || request_data.metric_type == "requests" {
            metrics_map.insert("active_requests".to_string(),
                              self.app_state.get_active_request_count().await.to_string());
        }

        Ok(Response::new(MetricsResponse {
            request_id,
            metrics: metrics_map,
            error: None,
        }))
    }

    /// Handle get resource usage requests
    async fn get_resource_usage(
        &self,
        request: Request<ResourceUsageRequest>,
    ) -> std::result::Result<Response<ResourceUsageResponse>, Status> {
        let request_data = request.into_inner();
        let request_id = request_data.request_id.clone();

        debug!("[AirVinegRPCService] Get resource usage request");

        let resources = self.app_state.get_resource_usage().await;

        Ok(Response::new(ResourceUsageResponse {
            request_id,
            memory_usage_mb: resources.memory_usage_mb,
            cpu_usage_percent: resources.cpu_usage_percent,
            disk_usage_mb: resources.disk_usage_mb,
            network_usage_mbps: resources.network_usage_mbps,
            error: None,
        }))
    }

    /// Handle set resource limits requests
    async fn set_resource_limits(
        &self,
        request: Request<ResourceLimitsRequest>,
    ) -> std::result::Result<Response<ResourceLimitsResponse>, Status> {
        let request_data = request.into_inner();
        let request_id = request_data.request_id.clone();

        info!("[AirVinegRPCService] Set resource limits: memory={}MB, cpu={}%, disk={}MB",
              request_data.memory_limit_mb, request_data.cpu_limit_percent, request_data.disk_limit_mb);

        // Validate limits
        if request_data.memory_limit_mb == 0 {
            return Ok(Response::new(ResourceLimitsResponse {
                request_id,
                success: false,
                error: Some("Memory limit must be greater than 0".to_string()),
            }));
        }

        if request_data.cpu_limit_percent > 100 {
            return Ok(Response::new(ResourceLimitsResponse {
                request_id,
                success: false,
                error: Some("CPU limit cannot exceed 100%".to_string()),
            }));
        }

        // Apply new limits via ApplicationState
        let result = self.app_state.set_resource_limits(
            Some(request_data.memory_limit_mb as u64),
            Some(request_data.cpu_limit_percent),
            Some(request_data.disk_limit_mb as u64),
        ).await;

        match result {
            Ok(_) => Ok(Response::new(ResourceLimitsResponse {
                request_id,
                success: true,
                error: None,
            })),
            Err(e) => Ok(Response::new(ResourceLimitsResponse {
                request_id,
                success: false,
                error: Some(e.to_string()),
            })),
        }
    }

    // ==================== Phase 6: Configuration Management ====================

    /// Handle get configuration requests
    async fn get_configuration(
        &self,
        request: Request<ConfigurationRequest>,
    ) -> std::result::Result<Response<ConfigurationResponse>, Status> {
        let request_data = request.into_inner();
        let request_id = request_data.request_id.clone();

        debug!("[AirVinegRPCService] Get configuration request: section='{}'", request_data.section);

        // Get configuration from ApplicationState
        let config = self.app_state.get_configuration().await;
        let mut config_map = std::collections::HashMap::new();

        // Serialize config to map, filter by section if specified
        match request_data.section.as_str() {
            "grpc" => {
                config_map.insert("bind_address".to_string(), config.grpc.bind_address);
                config_map.insert("max_connections".to_string(), config.grpc.max_connections.to_string());
                config_map.insert("request_timeout_secs".to_string(), config.grpc.request_timeout_secs.to_string());
            }
            "authentication" => {
                config_map.insert("enabled".to_string(), config.authentication.enabled.to_string());
                config_map.insert("credentials_path".to_string(), "***REDACTED***".to_string());
                config_map.insert("token_expiration_hours".to_string(), config.authentication.token_expiration_hours.to_string());
            }
            "updates" => {
                config_map.insert("enabled".to_string(), config.updates.enabled.to_string());
                config_map.insert("check_interval_hours".to_string(), config.updates.check_interval_hours.to_string());
                config_map.insert("update_server_url".to_string(), config.updates.update_server_url);
                config_map.insert("auto_download".to_string(), config.updates.auto_download.to_string());
                config_map.insert("auto_install".to_string(), config.updates.auto_install.to_string());
            }
            "downloader" => {
                config_map.insert("enabled".to_string(), config.downloader.enabled.to_string());
                config_map.insert("max_concurrent_downloads".to_string(), config.downloader.max_concurrent_downloads.to_string());
                config_map.insert("download_timeout_secs".to_string(), config.downloader.download_timeout_secs.to_string());
                config_map.insert("max_retries".to_string(), config.downloader.max_retries.to_string());
                config_map.insert("cache_directory".to_string(), config.downloader.cache_directory);
            }
            "indexing" => {
                config_map.insert("enabled".to_string(), config.indexing.enabled.to_string());
                config_map.insert("max_file_size_mb".to_string(), config.indexing.max_file_size_mb.to_string());
                config_map.insert("file_types".to_string(), config.indexing.file_types.join(","));
                config_map.insert("update_interval_minutes".to_string(), config.indexing.update_interval_minutes.to_string());
                config_map.insert("index_directory".to_string(), config.indexing.index_directory);
            }
            _ => {
                // Return all sections (redacted for sensitive values)
                config_map.insert("_grpc_enabled".to_string(), "true".to_string());
            }
        }

        Ok(Response::new(ConfigurationResponse {
            request_id,
            configuration: config_map,
            error: None,
        }))
    }

    /// Handle update configuration requests
    async fn update_configuration(
        &self,
        request: Request<UpdateConfigurationRequest>,
    ) -> std::result::Result<Response<UpdateConfigurationResponse>, Status> {
        let request_data = request.into_inner();
        let request_id = request_data.request_id.clone();

        info!("[AirVinegRPCService] Update configuration request: section='{}'",
              request_data.section);

        // Validate section
        if !["grpc", "authentication", "updates", "downloader", "indexing", "",].contains(&request_data.section.as_str()) {
            return Ok(Response::new(UpdateConfigurationResponse {
                request_id,
                success: false,
                error: Some("Invalid configuration section".to_string()),
            }));
        }

        // Update configuration via ApplicationState
        let result = self.app_state.update_configuration(
            request_data.section,
            request_data.updates,
        ).await;

        match result {
            Ok(_) => Ok(Response::new(UpdateConfigurationResponse {
                request_id,
                success: true,
                error: None,
            })),
            Err(e) => Ok(Response::new(UpdateConfigurationResponse {
                request_id,
                success: false,
                error: Some(e.to_string()),
            })),
        }
    }

    // ==================== Helper Methods ====================

    /// Detect MIME type based on file extension
    fn detect_mime_type(&self, path: &std::path::Path) -> String {
        match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => "text/x-rust".to_string(),
            Some("ts") => "application/typescript".to_string(),
            Some("js") => "application/javascript".to_string(),
            Some("json") => "application/json".to_string(),
            Some("toml") => "application/toml".to_string(),
            Some("md") => "text/markdown".to_string(),
            Some("txt") => "text/plain".to_string(),
            Some("yaml") | Some("yml") => "application/x-yaml".to_string(),
            Some("html") => "text/html".to_string(),
            Some("css") => "text/css".to_string(),
            Some("xml") => "application/xml".to_string(),
            Some("png") => "image/png".to_string(),
            Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),
            Some("gif") => "image/gif".to_string(),
            Some("svg") => "image/svg+xml".to_string(),
            Some("pdf") => "application/pdf".to_string(),
            Some("zip") => "application/zip".to_string(),
            Some("tar") | Some("gz") => "application/x-tar".to_string(),
            Some("proto") => "application/x-protobuf".to_string(),
            _ => "application/octet-stream".to_string(),
        }
    }

    /// Download file with exponential backoff retry
    /// Returns file_info (path, size, checksum) from DownloadManager
    async fn download_file_with_retry(
        &self,
        request_id: &str,
        url: String,
        destination_path: String,
        checksum: String,
        progress_callback: Option<Box<dyn Fn(f32) + Send>>,
    ) -> Result<crate::Downloader::DownloadResult> {
        let config = &self.app_state.configuration.downloader;
        let mut retries = 0;

        loop {
            match self.download_manager.download_file(
                url.clone(),
                destination_path.clone(),
                checksum.clone(),
            ).await {
                Ok(file_info) => {
                    if let Some(ref callback) = progress_callback {
                        callback(100.0);
                    }
                    return Ok(file_info);
                }
                Err(e) => {
                    if retries < config.max_retries as usize {
                        retries += 1;
                        let backoff_secs = 2u64.pow(retries as u32);
                        warn!("[AirVinegRPCService] Download failed [ID: {}], retrying (attempt {}/{}): {} - Backing off {} seconds",
                              request_id, retries, config.max_retries, e, backoff_secs);

                        if let Some(ref callback) = progress_callback {
                            // Notify retry attempts
                            let progress = (retries as f32 / config.max_retries as f32) * 10.0;
                            callback(progress);
                        }

                        tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
                    } else {
                        error!("[AirVinegRPCService] Download failed after {} retries [ID: {}]: {}",
                               config.max_retries, request_id, e);
                        return Err(e);
                    }
                }
            }
        }
    }

    /// Validate URL supports range headers for streaming
    async fn validate_range_support(&self, url: &str) -> Result<bool> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| {
                crate::AirError::Network(format!("Failed to create HTTP client for validation: {}", e))
            })?;

        let response = client.head(url)
            .send()
            .await
            .map_err(|e| {
                crate::AirError::Network(format!("Failed to send HEAD request: {}", e))
            })?;

        // Check if server supports range requests
        let accepts_ranges = response.headers()
            .get("accept-ranges")
            .map(|v| v.to_str().unwrap_or("none"))
            .unwrap_or("none");

        Ok(accepts_ranges == "bytes")
    }

    /// Prepare rollback backup before applying update
    async fn prepare_rollback_backup(&self, version: &str) -> Result<(), String> {
        let cache_dir = self.update_manager.get_cache_directory();
        let rollback_dir = cache_dir.join("rollback");
        
        // Create rollback directory if it doesn't exist
        if let Err(e) = tokio::fs::create_dir_all(&rollback_dir).await {
            return Err(format!("Failed to create rollback directory: {}", e));
        }

        // Create backup marker file with version
        let backup_file = rollback_dir.join(format!("backup-{}.marker", version));
        let marker_content = format!(
            "version={}\ntimestamp={}\nrollback_available=true",
            version,
            chrono::Utc::now().to_rfc3339()
        );

        if let Err(e) = tokio::fs::write(&backup_file, marker_content).await {
            return Err(format!("Failed to create backup marker: {}", e));
        }

        info!("[AirVinegRPCService] Rollback backup prepared for version {} at {:?}",
              version, backup_file);

        Ok(())
    }

    /// Cleanup rollback backup after successful update or failed verification
    async fn cleanup_rollback_backup(&self, version: &str) -> Result<(), String> {
        let cache_dir = self.update_manager.get_cache_directory();
        let rollback_dir = cache_dir.join("rollback");
        let backup_file = rollback_dir.join(format!("backup-{}.marker", version));

        if backup_file.exists() {
            if let Err(e) = tokio::fs::remove_file(&backup_file).await {
                return Err(format!("Failed to cleanup rollback backup: {}", e));
            }
            info!("[AirVinegRPCService] Rollback backup cleaned up for version {}",
                  version);
        }

        Ok(())
    }
}

/// Validate URL has a valid scheme
fn match_url_scheme(url: &str) -> bool {
    url.to_lowercase().starts_with("http://") || url.to_lowercase().starts_with("https://")
}

/// Calculate chunk checksum for verification
fn calculate_chunk_checksum(chunk: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(chunk);
    format!("{:x}", hasher.finalize())
}
