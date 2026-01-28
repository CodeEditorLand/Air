//! # Air Vine gRPC Service
//!
//! Defines the gRPC service implementation for Air. This struct handles
//! incoming RPC calls from Mountain, dispatches them to the appropriate
//! services (authentication, updates, downloads, indexing), and returns
//! the results.

use std::sync::Arc;
use log::{debug, error, info, trace};
use serde_json::Value;
use tonic::{Request, Response, Status};

use crate::{
    ApplicationState::ApplicationState,
    Authentication::AuthenticationService,
    Downloader::DownloadManager,
    Indexing::FileIndexer,
    Updates::UpdateManager,
    Result,
    utils::{generate_request_id, current_timestamp},
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
        }
    }
}

#[tonic::async_trait]
impl crate::Vine::Generated::air_service_server::AirService for AirVinegRPCService {
    /// Handle authentication requests from Mountain
    async fn authenticate(
        &self,
        request: Request<crate::Vine::Generated::AuthenticationRequest>,
    ) -> Result<Response<crate::Vine::Generated::AuthenticationResponse>, Status> {
        let request_data = request.into_inner();
        let request_id = request_data.request_id;
        
        info!("[AirVinegRPCService] Authentication request received [ID: {}]", request_id);
        
        self.app_state.register_request(request_id.clone(), "authentication".to_string())
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
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
                
                Ok(Response::new(crate::Vine::Generated::AuthenticationResponse {
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
                
                Ok(Response::new(crate::Vine::Generated::AuthenticationResponse {
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
        request: Request<crate::Vine::Generated::UpdateCheckRequest>,
    ) -> Result<Response<crate::Vine::Generated::UpdateCheckResponse>, Status> {
        let request_data = request.into_inner();
        let request_id = request_data.request_id;
        
        info!("[AirVinegRPCService] Update check request received [ID: {}]", request_id);
        
        self.app_state.register_request(request_id.clone(), "updates".to_string())
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        let result = self.update_manager.check_for_updates().await;
        
        match result {
            Ok(update_info) => {
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Completed, Some(100.0))
                    .await
                    .ok();
                
                Ok(Response::new(crate::Vine::Generated::UpdateCheckResponse {
                    request_id,
                    update_available: update_info.is_some(),
                    version: update_info.as_ref().map(|info| info.version.clone()).unwrap_or_default(),
                    download_url: update_info.as_ref().map(|info| info.download_url.clone()).unwrap_or_default(),
                    release_notes: update_info.as_ref().map(|info| info.release_notes.clone()).unwrap_or_default(),
                    error: None,
                }))
            },
            Err(e) => {
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(e.to_string()), None)
                    .await
                    .ok();
                
                Ok(Response::new(crate::Vine::Generated::UpdateCheckResponse {
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
        request: Request<crate::Vine::Generated::DownloadRequest>,
    ) -> Result<Response<crate::Vine::Generated::DownloadResponse>, Status> {
        let request_data = request.into_inner();
        let request_id = request_data.request_id;
        
        info!("[AirVinegRPCService] Download request received [ID: {}] - URL: {}", request_id, request_data.url);
        
        self.app_state.register_request(request_id.clone(), "downloader".to_string())
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        let result = self.download_manager.download_file(
            request_data.url,
            request_data.destination_path,
            request_data.checksum,
        ).await;
        
        match result {
            Ok(file_info) => {
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Completed, Some(100.0))
                    .await
                    .ok();
                
                Ok(Response::new(crate::Vine::Generated::DownloadResponse {
                    request_id,
                    success: true,
                    file_path: file_info.path,
                    file_size: file_info.size,
                    checksum: file_info.checksum,
                    error: None,
                }))
            },
            Err(e) => {
                self.app_state.update_request_status(&request_id, crate::ApplicationState::RequestState::Failed(e.to_string()), None)
                    .await
                    .ok();
                
                Ok(Response::new(crate::Vine::Generated::DownloadResponse {
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
    
    /// Handle file indexing requests from Mountain
    async fn index_files(
        &self,
        request: Request<crate::Vine::Generated::IndexRequest>,
    ) -> Result<Response<crate::Vine::Generated::IndexResponse>, Status> {
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
                
                Ok(Response::new(crate::Vine::Generated::IndexResponse {
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
                
                Ok(Response::new(crate::Vine::Generated::IndexResponse {
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
        request: Request<crate::Vine::Generated::StatusRequest>,
    ) -> Result<Response<crate::Vine::Generated::StatusResponse>, Status> {
        let request_data = request.into_inner();
        
        debug!("[AirVinegRPCService] Status request received");
        
        let metrics = self.app_state.get_metrics().await;
        let resources = self.app_state.get_resource_usage().await;
        
        Ok(Response::new(crate::Vine::Generated::StatusResponse {
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
        _request: Request<crate::Vine::Generated::HealthCheckRequest>,
    ) -> Result<Response<crate::Vine::Generated::HealthCheckResponse>, Status> {
        debug!("[AirVinegRPCService] Health check request received");
        
        Ok(Response::new(crate::Vine::Generated::HealthCheckResponse {
            healthy: true,
            timestamp: current_timestamp(),
        }))
    }
}
