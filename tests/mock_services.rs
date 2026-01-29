//! # Mock Services for Integration Testing
//!
//! Mock implementations of Mountain, Wind, Cocoon, and Air services
//! for comprehensive integration testing without external dependencies.

use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use Air::{
    ApplicationState::{ApplicationState, ConnectionType},
    Authentication::AuthenticationService,
    Configuration::ConfigurationManager,
    Downloader::{DownloadManager, DownloadRequest, DownloadResponse},
    Indexing::{FileIndexer, IndexRequest, IndexResponse},
    Updates::{UpdateManager, UpdateCheckRequest, UpdateCheckResponse},
    Vine::Generated::air::{
        AirService, AuthenticationRequest, AuthenticationResponse,
        UpdateCheckRequest as ProtoUpdateCheckRequest, UpdateCheckResponse as ProtoUpdateCheckResponse,
        DownloadRequest as ProtoDownloadRequest, DownloadResponse as ProtoDownloadResponse,
        IndexRequest as ProtoIndexRequest, IndexResponse as ProtoIndexResponse,
        HealthCheckRequest, HealthCheckResponse,
        StatusRequest, StatusResponse,
    },
};

/// Mock Mountain Service for simulating Mountain backend
pub struct MockMountainService {
    pub is_ready: bool,
    pub protocol_version: u32,
    pub connection_count: Arc<Mutex<u32>>,
}

impl MockMountainService {
    pub fn new() -> Self {
        Self {
            is_ready: true,
            protocol_version: 1,
            connection_count: Arc::new(Mutex::new(0)),
        }
    }
    
    pub async fn simulate_connection(&self) -> bool {
        let mut count = self.connection_count.lock().await;
        *count += 1;
        self.is_ready
    }
    
    pub async fn get_connection_count(&self) -> u32 {
        *self.connection_count.lock().await
    }
}

/// Mock Wind Service for simulating UI component interactions
pub struct MockWindService {
    pub ui_events: Arc<Mutex<Vec<String>>>,
    pub component_states: Arc<Mutex<std::collections::HashMap<String, String>>>,
}

impl MockWindService {
    pub fn new() -> Self {
        Self {
            ui_events: Arc::new(Mutex::new(Vec::new())),
            component_states: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }
    
    pub async fn send_ui_event(&self, event: String) {
        let mut events = self.ui_events.lock().await;
        events.push(event);
    }
    
    pub async fn get_ui_events(&self) -> Vec<String> {
        self.ui_events.lock().await.clone()
    }
    
    pub async fn update_component_state(&self, component: String, state: String) {
        let mut states = self.component_states.lock().await;
        states.insert(component, state);
    }
}

/// Mock Cocoon Service for simulating VS Code extension hosting
pub struct MockCocoonService {
    pub extension_hosts: Arc<Mutex<Vec<String>>>,
    pub protocol_compatibility: u32,
}

impl MockCocoonService {
    pub fn new() -> Self {
        Self {
            extension_hosts: Arc::new(Mutex::new(vec!["vscode".to_string()])),
            protocol_compatibility: 1,
        }
    }
    
    pub async fn add_extension_host(&self, host: String) {
        let mut hosts = self.extension_hosts.lock().await;
        hosts.push(host);
    }
    
    pub async fn get_extension_hosts(&self) -> Vec<String> {
        self.extension_hosts.lock().await.clone()
    }
}

/// Mock Authentication Service
pub struct MockAuthenticationService {
    pub authenticated_clients: Arc<Mutex<std::collections::HashMap<String, bool>>>,
}

impl MockAuthenticationService {
    pub fn new() -> Self {
        Self {
            authenticated_clients: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }
}

impl AuthenticationService for MockAuthenticationService {
    async fn authenticate(&self, client_id: &str, _credentials: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let mut clients = self.authenticated_clients.lock().await;
        clients.insert(client_id.to_string(), true);
        Ok(true)
    }
    
    async fn validate_token(&self, client_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let clients = self.authenticated_clients.lock().await;
        Ok(clients.get(client_id).copied().unwrap_or(false))
    }
}

/// Mock Update Manager
pub struct MockUpdateManager {
    pub available_updates: Arc<Mutex<Vec<String>>>,
    pub update_status: Arc<Mutex<String>>,
}

impl MockUpdateManager {
    pub fn new() -> Self {
        Self {
            available_updates: Arc::new(Mutex::new(Vec::new())),
            update_status: Arc::new(Mutex::new("idle".to_string())),
        }
    }
}

impl UpdateManager for MockUpdateManager {
    async fn check_for_updates(&self, _request: UpdateCheckRequest) -> Result<UpdateCheckResponse, Box<dyn std::error::Error>> {
        let updates = self.available_updates.lock().await;
        Ok(UpdateCheckResponse {
            available: !updates.is_empty(),
            version: updates.first().cloned().unwrap_or_default(),
            download_url: "".to_string(),
            changelog: "".to_string(),
        })
    }
    
    async fn apply_update(&self, _version: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let mut status = self.update_status.lock().await;
        *status = "applied".to_string();
        Ok(true)
    }
}

/// Mock Download Manager
pub struct MockDownloadManager {
    pub download_queue: Arc<Mutex<Vec<String>>>,
    pub download_progress: Arc<Mutex<std::collections::HashMap<String, u32>>>,
}

impl MockDownloadManager {
    pub fn new() -> Self {
        Self {
            download_queue: Arc::new(Mutex::new(Vec::new())),
            download_progress: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }
}

impl DownloadManager for MockDownloadManager {
    async fn download_file(&self, request: DownloadRequest) -> Result<DownloadResponse, Box<dyn std::error::Error>> {
        let mut queue = self.download_queue.lock().await;
        queue.push(request.url.clone());
        
        let mut progress = self.download_progress.lock().await;
        progress.insert(request.url.clone(), 0);
        
        Ok(DownloadResponse {
            success: true,
            file_path: format!("/tmp/{}", request.url.split('/').last().unwrap_or("file")),
            checksum: "mock-checksum".to_string(),
        })
    }
    
    async fn get_download_progress(&self, url: &str) -> Result<u32, Box<dyn std::error::Error>> {
        let progress = self.download_progress.lock().await;
        Ok(*progress.get(url).unwrap_or(&0))
    }
}

/// Mock File Indexer
pub struct MockFileIndexer {
    pub indexed_files: Arc<Mutex<Vec<String>>>,
    pub index_status: Arc<Mutex<String>>,
}

impl MockFileIndexer {
    pub fn new() -> Self {
        Self {
            indexed_files: Arc::new(Mutex::new(Vec::new())),
            index_status: Arc::new(Mutex::new("idle".to_string())),
        }
    }
}

impl FileIndexer for MockFileIndexer {
    async fn index_files(&self, request: IndexRequest) -> Result<IndexResponse, Box<dyn std::error::Error>> {
        let mut files = self.indexed_files.lock().await;
        files.push(request.path.clone());
        
        let mut status = self.index_status.lock().await;
        *status = "indexed".to_string();
        
        Ok(IndexResponse {
            success: true,
            file_count: files.len() as u32,
            indexed_files: files.clone(),
        })
    }
    
    async fn search_files(&self, _query: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let files = self.indexed_files.lock().await;
        Ok(files.clone())
    }
}

/// Mock Air Service implementation for testing
pub struct MockAirService {
    pub health_status: Arc<Mutex<String>>,
    pub service_status: Arc<Mutex<String>>,
}

impl MockAirService {
    pub fn new() -> Self {
        Self {
            health_status: Arc::new(Mutex::new("healthy".to_string())),
            service_status: Arc::new(Mutex::new("running".to_string())),
        }
    }
}

#[async_trait::async_trait]
impl AirService for MockAirService {
    async fn health_check(&self, _request: Request<HealthCheckRequest>) -> Result<Response<HealthCheckResponse>, Status> {
        let status = self.health_status.lock().await;
        Ok(Response::new(HealthCheckResponse {
            status: status.clone(),
            timestamp: chrono::Utc::now().timestamp(),
        }))
    }
    
    async fn get_status(&self, _request: Request<StatusRequest>) -> Result<Response<StatusResponse>, Status> {
        let status = self.service_status.lock().await;
        Ok(Response::new(StatusResponse {
            service_status: status.clone(),
            active_connections: 1,
            memory_usage_mb: 100,
            cpu_usage_percent: 5.0,
        }))
    }
    
    async fn authenticate(&self, _request: Request<AuthenticationRequest>) -> Result<Response<AuthenticationResponse>, Status> {
        Ok(Response::new(AuthenticationResponse {
            success: true,
            token: "mock-token".to_string(),
            expires_at: chrono::Utc::now().timestamp() + 3600,
        }))
    }
    
    async fn check_for_updates(&self, _request: Request<ProtoUpdateCheckRequest>) -> Result<Response<ProtoUpdateCheckResponse>, Status> {
        Ok(Response::new(ProtoUpdateCheckResponse {
            available: false,
            version: "".to_string(),
            download_url: "".to_string(),
            changelog: "".to_string(),
        }))
    }
    
    async fn download_file(&self, _request: Request<ProtoDownloadRequest>) -> Result<Response<ProtoDownloadResponse>, Status> {
        Ok(Response::new(ProtoDownloadResponse {
            success: true,
            file_path: "/tmp/mock-file".to_string(),
            checksum: "mock-checksum".to_string(),
        }))
    }
    
    async fn index_files(&self, _request: Request<ProtoIndexRequest>) -> Result<Response<ProtoIndexResponse>, Status> {
        Ok(Response::new(ProtoIndexResponse {
            success: true,
            file_count: 0,
            indexed_files: vec!["mock-file.txt".to_string()],
        }))
    }
}
