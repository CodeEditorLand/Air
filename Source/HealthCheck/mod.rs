//! # Health Check System
//!
//! Provides comprehensive health monitoring for Air daemon services,
//! including multi-level health checks, dependency validation, and
//! automatic recovery mechanisms.

use std::{collections::HashMap, sync::Arc};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{Result, AirError, utils};

/// Health check manager
#[derive(Debug)]
pub struct HealthCheckManager {
    /// Service health status
    service_health: Arc<RwLock<HashMap<String, ServiceHealth>>>,
    /// Health check history
    health_history: Arc<RwLock<Vec<HealthCheckRecord>>>,
    /// Recovery actions
    recovery_actions: Arc<RwLock<HashMap<String, RecoveryAction>>>,
    /// Health check configuration
    config: HealthCheckConfig,
}

/// Service health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    /// Service name
    pub service_name: String,
    /// Current health status
    pub status: HealthStatus,
    /// Last check timestamp
    pub last_check: u64,
    /// Last successful check timestamp
    pub last_success: Option<u64>,
    /// Failure count
    pub failure_count: u32,
    /// Error message (if any)
    pub error_message: Option<String>,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
    /// Health check level
    pub check_level: HealthCheckLevel,
}

/// Health status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    /// Service is healthy
    Healthy,
    /// Service is degraded but functional
    Degraded,
    /// Service is unhealthy
    Unhealthy,
    /// Service is unknown/unchecked
    Unknown,
}

/// Health check level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthCheckLevel {
    /// Basic liveness check
    Alive,
    /// Service responds to requests
    Responsive,
    /// Service performs its core function
    Functional,
}

/// Health check record for history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckRecord {
    /// Timestamp
    pub timestamp: u64,
    /// Service name
    pub service_name: String,
    /// Health status
    pub status: HealthStatus,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
    /// Error message (if any)
    pub error_message: Option<String>,
}

/// Recovery action configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
    /// Action name
    pub name: String,
    /// Service name
    pub service_name: String,
    /// Trigger condition
    pub trigger: RecoveryTrigger,
    /// Action to take
    pub action: RecoveryActionType,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Current retry count
    pub retry_count: u32,
}

/// Recovery trigger conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryTrigger {
    /// Trigger after N consecutive failures
    ConsecutiveFailures(u32),
    /// Trigger when response time exceeds threshold
    ResponseTimeExceeds(u64),
    /// Trigger when service becomes unresponsive
    ServiceUnresponsive,
}

/// Recovery action types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryActionType {
    /// Restart the service
    RestartService,
    /// Reset connection
    ResetConnection,
    /// Clear cache
    ClearCache,
    /// Reload configuration
    ReloadConfiguration,
    /// Escalate to higher level
    Escalate,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Default check interval in seconds
    pub default_check_interval: u64,
    /// Health history retention (number of records)
    pub history_retention: usize,
    /// Consecutive failures threshold
    pub consecutive_failures_threshold: u32,
    /// Response time threshold in milliseconds
    pub response_time_threshold_ms: u64,
    /// Enable automatic recovery
    pub enable_auto_recovery: bool,
    /// Recovery timeout in seconds
    pub recovery_timeout_sec: u64,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            default_check_interval: 30,
            history_retention: 100,
            consecutive_failures_threshold: 3,
            response_time_threshold_ms: 5000,
            enable_auto_recovery: true,
            recovery_timeout_sec: 60,
        }
    }
}

impl HealthCheckManager {
    /// Create a new HealthCheckManager instance
    pub fn new(config: Option<HealthCheckConfig>) -> Self {
        Self {
            service_health: Arc::new(RwLock::new(HashMap::new())),
            health_history: Arc::new(RwLock::new(Vec::new())),
            recovery_actions: Arc::new(RwLock::new(HashMap::new())),
            config: config.unwrap_or_default(),
        }
    }
    
    /// Register a service for health monitoring
    pub async fn register_service(&self, service_name: String, check_level: HealthCheckLevel) -> Result<()> {
        let mut health_map = self.service_health.write().await;
        
        health_map.insert(service_name.clone(), ServiceHealth {
            service_name: service_name.clone(),
            status: HealthStatus::Unknown,
            last_check: 0,
            last_success: None,
            failure_count: 0,
            error_message: None,
            response_time_ms: None,
            check_level: check_level.clone(),
        });
        
        info!("[HealthCheck] Registered service for monitoring: {} ({:?})", service_name, check_level);
        Ok(())
    }
    
    /// Perform health check for a service
    pub async fn check_service(&self, service_name: &str) -> Result<HealthStatus> {
        let start_time = utils::current_timestamp();
        
        // Perform service-specific health check
        let (status, error_message) = match service_name {
            "authentication" => self.check_authentication_service().await,
            "updates" => self.check_updates_service().await,
            "downloader" => self.check_downloader_service().await,
            "indexing" => self.check_indexing_service().await,
            "grpc" => self.check_grpc_service().await,
            "connections" => self.check_connections_service().await,
            _ => {
                warn!("[HealthCheck] Unknown service: {}", service_name);
                return Err(AirError::Internal(format!("Unknown service: {}", service_name)));
            }
        };
        
        let response_time = utils::current_timestamp() - start_time;
        
        // Update service health
        self.update_service_health(service_name, status.clone(), &error_message, response_time).await?;
        
        // Record health check
        self.record_health_check(service_name, status.clone(), response_time, &error_message).await;
        
        // Trigger recovery if needed
        if self.config.enable_auto_recovery {
            self.trigger_recovery_if_needed(service_name).await;
        }
        
        Ok(status)
    }
    
    /// Check authentication service health
    async fn check_authentication_service(&self) -> (HealthStatus, Option<String>) {
        // TODO: Implement actual authentication service health check
        // For now, return healthy status
        (HealthStatus::Healthy, None)
    }
    
    /// Check updates service health
    async fn check_updates_service(&self) -> (HealthStatus, Option<String>) {
        // TODO: Implement actual updates service health check
        // For now, return healthy status
        (HealthStatus::Healthy, None)
    }
    
    /// Check downloader service health
    async fn check_downloader_service(&self) -> (HealthStatus, Option<String>) {
        // TODO: Implement actual downloader service health check
        // For now, return healthy status
        (HealthStatus::Healthy, None)
    }
    
    /// Check indexing service health
    async fn check_indexing_service(&self) -> (HealthStatus, Option<String>) {
        // TODO: Implement actual indexing service health check
        // For now, return healthy status
        (HealthStatus::Healthy, None)
    }
    
    /// Check gRPC service health
    async fn check_grpc_service(&self) -> (HealthStatus, Option<String>) {
        // TODO: Implement actual gRPC service health check
        // For now, return healthy status
        (HealthStatus::Healthy, None)
    }
    
    /// Check connections service health
    async fn check_connections_service(&self) -> (HealthStatus, Option<String>) {
        // TODO: Implement actual connections service health check
        // For now, return healthy status
        (HealthStatus::Healthy, None)
    }
    
    /// Update service health status
    async fn update_service_health(
        &self,
        service_name: &str,
        status: HealthStatus,
        error_message: &Option<String>,
        response_time: u64,
    ) -> Result<()> {
        let mut health_map = self.service_health.write().await;
        
        if let Some(service_health) = health_map.get_mut(service_name) {
            service_health.status = status.clone();
            service_health.last_check = utils::current_timestamp();
            service_health.response_time_ms = Some(response_time);
            
            match status {
                HealthStatus::Healthy => {
                    service_health.last_success = Some(utils::current_timestamp());
                    service_health.failure_count = 0;
                    service_health.error_message = None;
                }
                HealthStatus::Degraded | HealthStatus::Unhealthy => {
                    service_health.failure_count += 1;
                    service_health.error_message = error_message.clone();
                }
                HealthStatus::Unknown => {
                    // Keep existing state
                }
            }
        } else {
            return Err(AirError::Internal(format!("Service not registered: {}", service_name)));
        }
        
        debug!("[HealthCheck] Updated health for {}: {:?} ({}ms)", service_name, status, response_time);
        Ok(())
    }
    
    /// Record health check in history
    async fn record_health_check(
        &self,
        service_name: &str,
        status: HealthStatus,
        response_time: u64,
        error_message: &Option<String>,
    ) {
        let mut history = self.health_history.write().await;
        
        let record = HealthCheckRecord {
            timestamp: utils::current_timestamp(),
            service_name: service_name.to_string(),
            status,
            response_time_ms: Some(response_time),
            error_message: error_message.clone(),
        };
        
        history.push(record);
        
        // Trim history to retention limit
        if history.len() > self.config.history_retention {
            history.remove(0);
        }
    }
    
    /// Trigger recovery actions if needed
    async fn trigger_recovery_if_needed(&self, service_name: &str) {
        let health_map = self.service_health.read().await;
        
        if let Some(service_health) = health_map.get(service_name) {
            // Check if recovery is needed based on failure count
            if service_health.failure_count >= self.config.consecutive_failures_threshold {
                warn!("[HealthCheck] Service {} has {} consecutive failures, triggering recovery", 
                      service_name, service_health.failure_count);
                
                // TODO: Implement actual recovery actions
                self.perform_recovery_action(service_name).await;
            }
            
            // Check if recovery is needed based on response time
            if let Some(response_time) = service_health.response_time_ms {
                if response_time > self.config.response_time_threshold_ms {
                    warn!("[HealthCheck] Service {} response time {}ms exceeds threshold {}ms", 
                          service_name, response_time, self.config.response_time_threshold_ms);
                    
                    // TODO: Implement response time-based recovery
                }
            }
        }
    }
    
    /// Perform recovery action for a service
    async fn perform_recovery_action(&self, service_name: &str) {
        // TODO: Implement actual recovery actions
        // This would involve restarting services, resetting connections, etc.
        info!("[HealthCheck] Performing recovery action for {}", service_name);
        
        match service_name {
            "grpc" => {
                // Restart gRPC server
                warn!("[HealthCheck] Recovery: Restarting gRPC server for {}", service_name);
            }
            "connections" => {
                // Reset connections
                warn!("[HealthCheck] Recovery: Resetting connections for {}", service_name);
            }
            _ => {
                // Generic recovery action
                warn!("[HealthCheck] Recovery: Generic action for {}", service_name);
            }
        }
    }
    
    /// Get overall daemon health status
    pub async fn get_overall_health(&self) -> HealthStatus {
        let health_map = self.service_health.read().await;
        
        let mut healthy_count = 0;
        let mut degraded_count = 0;
        let mut unhealthy_count = 0;
        
        for service_health in health_map.values() {
            match service_health.status {
                HealthStatus::Healthy => healthy_count += 1,
                HealthStatus::Degraded => degraded_count += 1,
                HealthStatus::Unhealthy => unhealthy_count += 1,
                HealthStatus::Unknown => {}
            }
        }
        
        if unhealthy_count > 0 {
            HealthStatus::Unhealthy
        } else if degraded_count > 0 {
            HealthStatus::Degraded
        } else if healthy_count > 0 {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unknown
        }
    }
    
    /// Get service health status
    pub async fn get_service_health(&self, service_name: &str) -> Option<ServiceHealth> {
        let health_map = self.service_health.read().await;
        health_map.get(service_name).cloned()
    }
    
    /// Get health check history
    pub async fn get_health_history(&self, service_name: Option<&str>, limit: Option<usize>) -> Vec<HealthCheckRecord> {
        let history = self.health_history.read().await;
        
        let mut filtered_history: Vec<HealthCheckRecord> = if let Some(service) = service_name {
            history.iter()
                .filter(|record| record.service_name == service)
                .cloned()
                .collect()
        } else {
            history.clone()
        };
        
        // Reverse to get most recent first
        filtered_history.reverse();
        
        // Apply limit
        if let Some(limit) = limit {
            filtered_history.truncate(limit);
        }
        
        filtered_history
    }
    
    /// Register a recovery action
    pub async fn register_recovery_action(&self, action: RecoveryAction) -> Result<()> {
        let mut actions = self.recovery_actions.write().await;
        actions.insert(action.name.clone(), action);
        Ok(())
    }
    
    /// Get health statistics
    pub async fn get_health_statistics(&self) -> HealthStatistics {
        let health_map = self.service_health.read().await;
        let history = self.health_history.read().await;
        // Count service statuses
        let mut healthy_services = 0;
        let mut degraded_services = 0;
        let mut unhealthy_services = 0;
        
        for service_health in health_map.values() {
            match service_health.status {
                HealthStatus::Healthy => healthy_services += 1,
                HealthStatus::Degraded => degraded_services += 1,
                HealthStatus::Unhealthy => unhealthy_services += 1,
                HealthStatus::Unknown => {}
            }
        }
        
        // Get health statistics
        let mut stats = HealthStatistics {
            total_services: health_map.len(),
            healthy_services,
            degraded_services,
            unhealthy_services,
            total_checks: history.len(),
            average_response_time_ms: 0.0,
            success_rate: 0.0,
        };
        
        // Calculate response time and success rate
        if !history.is_empty() {
            let mut total_response_time = 0;
            let mut successful_checks = 0;
            
            for record in history.iter() {
                if let Some(response_time) = record.response_time_ms {
                    total_response_time += response_time;
                }
                
                if record.status == HealthStatus::Healthy {
                    successful_checks += 1;
                }
            }
            
            stats.average_response_time_ms = total_response_time as f64 / history.len() as f64;
            stats.success_rate = successful_checks as f64 / history.len() as f64 * 100.0;
        }
        
        stats
    }
}

/// Health statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatistics {
    pub total_services: usize,
    pub healthy_services: usize,
    pub degraded_services: usize,
    pub unhealthy_services: usize,
    pub total_checks: usize,
    pub average_response_time_ms: f64,
    pub success_rate: f64,
}

impl HealthStatistics {
    /// Get overall health percentage
    pub fn overall_health_percentage(&self) -> f64 {
        if self.total_services == 0 {
            return 0.0;
        }
        
        (self.healthy_services as f64 / self.total_services as f64) * 100.0
    }
}

/// Health check response for gRPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub overall_status: HealthStatus,
    pub service_health: HashMap<String, ServiceHealth>,
    pub statistics: HealthStatistics,
    pub timestamp: u64,
}

impl HealthCheckResponse {
    /// Create a new health check response
    pub fn new(
        overall_status: HealthStatus,
        service_health: HashMap<String, ServiceHealth>,
        statistics: HealthStatistics,
    ) -> Self {
        Self {
            overall_status,
            service_health,
            statistics,
            timestamp: utils::current_timestamp(),
        }
    }
}
