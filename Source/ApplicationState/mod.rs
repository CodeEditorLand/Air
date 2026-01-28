//! # Application State Management
//!
//! Manages the global state of the Air daemon, including configuration,
//! service status, and resource tracking.

use std::{collections::HashMap, sync::Arc};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

use crate::{Configuration::AirConfiguration, Result, AirError};

/// Application state structure
#[derive(Debug)]
pub struct ApplicationState {
    /// Current configuration
    pub configuration: Arc<AirConfiguration>,
    
    /// Service status tracking
    pub service_status: Arc<RwLock<HashMap<String, ServiceStatus>>>,
    
    /// Active requests tracking
    pub active_requests: Arc<Mutex<HashMap<String, RequestStatus>>>,
    
    /// Performance metrics
    pub metrics: Arc<RwLock<PerformanceMetrics>>,
    
    /// Resource usage tracking
    pub resources: Arc<RwLock<ResourceUsage>>,
}

/// Service status enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Error(String),
}

/// Request status tracking
#[derive(Debug, Clone)]
pub struct RequestStatus {
    pub request_id: String,
    pub service: String,
    pub started_at: u64,
    pub status: RequestState,
    pub progress: Option<f32>,
}

/// Request state enum
#[derive(Debug, Clone)]
pub enum RequestState {
    Pending,
    InProgress,
    Completed,
    Failed(String),
    Cancelled,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time: f64,
    pub uptime_seconds: u64,
    pub last_updated: u64,
}

/// Resource usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub disk_usage_mb: f64,
    pub network_usage_mbps: f64,
    pub last_updated: u64,
}

impl ApplicationState {
    /// Create a new ApplicationState instance
    pub async fn new(configuration: Arc<AirConfiguration>) -> Result<Self> {
        let state = Self {
            configuration,
            service_status: Arc::new(RwLock::new(HashMap::new())),
            active_requests: Arc::new(Mutex::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(PerformanceMetrics {
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                average_response_time: 0.0,
                uptime_seconds: 0,
                last_updated: crate::utils::current_timestamp(),
            })),
            resources: Arc::new(RwLock::new(ResourceUsage {
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
                disk_usage_mb: 0.0,
                network_usage_mbps: 0.0,
                last_updated: crate::utils::current_timestamp(),
            })),
        };
        
        // Initialize service status
        state.initialize_service_status().await?;
        
        Ok(state)
    }
    
    /// Initialize service status tracking
    async fn initialize_service_status(&self) -> Result<()> {
        let mut status = self.service_status.write().await;
        
        status.insert("authentication".to_string(), ServiceStatus::Starting);
        status.insert("updates".to_string(), ServiceStatus::Starting);
        status.insert("downloader".to_string(), ServiceStatus::Starting);
        status.insert("indexing".to_string(), ServiceStatus::Starting);
        status.insert("grpc".to_string(), ServiceStatus::Starting);
        
        Ok(())
    }
    
    /// Update service status
    pub async fn update_service_status(&self, service: &str, status: ServiceStatus) -> Result<()> {
        let mut service_status = self.service_status.write().await;
        service_status.insert(service.to_string(), status);
        Ok(())
    }
    
    /// Get service status
    pub async fn get_service_status(&self, service: &str) -> Option<ServiceStatus> {
        let service_status = self.service_status.read().await;
        service_status.get(service).cloned()
    }
    
    /// Register a new request
    pub async fn register_request(&self, request_id: String, service: String) -> Result<()> {
        let mut requests = self.active_requests.lock().await;
        
        requests.insert(request_id.clone(), RequestStatus {
            request_id: request_id.clone(),
            service,
            started_at: crate::utils::current_timestamp(),
            status: RequestState::Pending,
            progress: None,
        });
        
        Ok(())
    }
    
    /// Update request status
    pub async fn update_request_status(&self, request_id: &str, status: RequestState, progress: Option<f32>) -> Result<()> {
        let mut requests = self.active_requests.lock().await;
        
        if let Some(request) = requests.get_mut(request_id) {
            request.status = status;
            request.progress = progress;
        } else {
            return Err(AirError::Internal(format!("Request {} not found", request_id)));
        }
        
        Ok(())
    }
    
    /// Remove completed request
    pub async fn remove_request(&self, request_id: &str) -> Result<()> {
        let mut requests = self.active_requests.lock().await;
        requests.remove(request_id);
        Ok(())
    }
    
    /// Update performance metrics
    pub async fn update_metrics(&self, success: bool, response_time: u64) -> Result<()> {
        let mut metrics = self.metrics.write().await;
        
        metrics.total_requests += 1;
        if success {
            metrics.successful_requests += 1;
        } else {
            metrics.failed_requests += 1;
        }
        
        // Update average response time using exponential moving average
        let alpha = 0.1; // Smoothing factor
        metrics.average_response_time = alpha * (response_time as f64) + 
            (1.0 - alpha) * metrics.average_response_time;
        
        metrics.last_updated = crate::utils::current_timestamp();
        
        Ok(())
    }
    
    /// Update resource usage
    pub async fn update_resource_usage(&self) -> Result<()> {
        let mut resources = self.resources.write().await;
        
        // Use systemstat to get actual system metrics
        if let Ok(sys) = systemstat::System::new() {
            if let Ok(memory) = sys.memory() {
                resources.memory_usage_mb = (memory.total.as_u64() - memory.free.as_u64()) as f64 / 1024.0 / 1024.0;
            }
            
            if let Ok(cpu) = sys.cpu_load_aggregate() {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                if let Ok(cpu) = cpu.done() {
                    resources.cpu_usage_percent = (cpu.user + cpu.nice + cpu.system) * 100.0;
                }
            }
        }
        
        resources.last_updated = crate::utils::current_timestamp();
        
        Ok(())
    }
    
    /// Get performance metrics
    pub async fn get_metrics(&self) -> PerformanceMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }
    
    /// Get resource usage
    pub async fn get_resource_usage(&self) -> ResourceUsage {
        let resources = self.resources.read().await;
        resources.clone()
    }
    
    /// Get active request count
    pub async fn get_active_request_count(&self) -> usize {
        let requests = self.active_requests.lock().await;
        requests.len()
    }
}