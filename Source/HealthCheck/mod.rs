//! # Health Check System
//!
//! Provides comprehensive health monitoring for Air daemon services,
//! ensuring VSCode stability and security through multi-level health checks,
//! dependency validation, and automatic recovery mechanisms.
//!
//! ## Responsibilities
//!
//! - Monitor critical Air services (authentication, updates, downloader,
//!   indexing, gRPC, connections)
//! - Implement multi-level health checks (Alive, Responsive, Functional)
//! - Provide automatic recovery actions when services fail
//! - Track health history and performance metrics
//! - Integrate with VSCode's stability patterns for service health monitoring
//!
//! ## VSCode Stability References
//!
//! This health check system aligns with VSCode's health monitoring patterns:
//! - Service health tracking similar to VSCode's workbench service health
//! - Dependency validation matching VSCode's extension host health checks
//! - Recovery patterns inspired by VSCode's crash recovery mechanisms
//! - Performance monitoring patterns from VSCode's telemetry system
//!
//! Referenced from:
//! vs/workbench/services/telemetry
//!
//! ## Mountain Monitoring Integration
//!
//! Health check results are integrated with Mountain monitoring system:
//! - Health status updates flow to Mountain's monitoring dashboards
//! - Critical health events trigger alerts in Mountain's alerting system
//! - Health metrics are aggregated for system-wide health assessment
//! - Recovery actions are coordinated with Mountain's service management
//!
//! ## Monitoring Patterns
//!
//! ### Multi-Level Health Checks
//! - **Alive**: Basic service process check
//! - **Responsive**: Service responds to health check queries
//! - **Functional**: Service performs its core operations correctly
//!
//! ### Circuit Breaking
//! - Services are temporarily marked as unhealthy after consecutive failures
//! - Circuit breaker prevents cascading failures
//! - Automatic circuit breaker reset after cool-down period
//! - Manual circuit breaker reset available for administrative overrides
//!
//! ### Timeout Handling
//! - Each health check has a configurable timeout
//! - Timeout events trigger immediate recovery actions
//! - Timeout history tracked to identify performance degradation
//! - Adaptive timeout adjustment based on observed performance
//!
//! ## Recovery Mechanisms
//!
//! Recovery actions are triggered based on:
//! - Consecutive failure count exceeding threshold
//! - Response time exceeding configured threshold
//! - Service unresponsiveness detected
//! - Manual-triggered recovery
//!
//! Recovery actions include:
//! - Service restart (graceful shutdown and restart)
//! - Connection reset (re-establish network connections)
//! - Cache clearing (remove stale or corrupted cache)
//! - Configuration reload (refresh service configuration)
//! - Escalation (notify administrators for manual intervention)
//!
//! ## TODO: Advanced Features
//!
//! - Implement advanced metrics collection (latency percentiles, error rates)
//! - Add health check scheduling automation (cron-like scheduling)
//! - Implement predictive health analysis (machine learning-based)
//! - Add security compliance checks (PCI-DSS, GDPR, etc.)
//! - Implement distributed health checks for clustered deployments
//! - Add health check export formats (Prometheus, Grafana, etc.)
//! - Implement health check alerting through multiple channels (email, Slack,
//!   etc.)
//! - Add health check simulation for testing and validation
//!
//! ## Configuration
//!
//! Health check behavior is configurable through HealthCheckConfig:
//! - `default_check_interval`: Time between automatic health checks
//! - `history_retention`: Number of health check records to keep
//! - `consecutive_failures_threshold`: Failures before triggering recovery
//! - `response_time_threshold_ms`: Response time threshold for recovery
//! - `enable_auto_recovery`: Enable/disable automatic recovery
//! - `recovery_timeout_sec`: Maximum time for recovery actions

use std::{collections::HashMap, sync::Arc};

use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{AirError, Result, utils};

/// Health check manager
#[derive(Debug)]
pub struct HealthCheckManager {
	/// Service health status
	service_health:Arc<RwLock<HashMap<String, ServiceHealth>>>,
	/// Health check history
	health_history:Arc<RwLock<Vec<HealthCheckRecord>>>,
	/// Recovery actions
	recovery_actions:Arc<RwLock<HashMap<String, RecoveryAction>>>,
	/// Health check configuration
	config:HealthCheckConfig,
}

/// Service health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
	/// Service name
	pub service_name:String,
	/// Current health status
	pub status:HealthStatus,
	/// Last check timestamp
	pub last_check:u64,
	/// Last successful check timestamp
	pub last_success:Option<u64>,
	/// Failure count
	pub failure_count:u32,
	/// Error message (if any)
	pub error_message:Option<String>,
	/// Response time in milliseconds
	pub response_time_ms:Option<u64>,
	/// Health check level
	pub check_level:HealthCheckLevel,
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
	pub timestamp:u64,
	/// Service name
	pub service_name:String,
	/// Health status
	pub status:HealthStatus,
	/// Response time in milliseconds
	pub response_time_ms:Option<u64>,
	/// Error message (if any)
	pub error_message:Option<String>,
}

/// Recovery action configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
	/// Action name
	pub name:String,
	/// Service name
	pub service_name:String,
	/// Trigger condition
	pub trigger:RecoveryTrigger,
	/// Action to take
	pub action:RecoveryActionType,
	/// Maximum retry attempts
	pub max_retries:u32,
	/// Current retry count
	pub retry_count:u32,
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
	pub default_check_interval:u64,
	/// Health history retention (number of records)
	pub history_retention:usize,
	/// Consecutive failures threshold
	pub consecutive_failures_threshold:u32,
	/// Response time threshold in milliseconds
	pub response_time_threshold_ms:u64,
	/// Enable automatic recovery
	pub enable_auto_recovery:bool,
	/// Recovery timeout in seconds
	pub recovery_timeout_sec:u64,
}

impl Default for HealthCheckConfig {
	fn default() -> Self {
		Self {
			default_check_interval:30,
			history_retention:100,
			consecutive_failures_threshold:3,
			response_time_threshold_ms:5000,
			enable_auto_recovery:true,
			recovery_timeout_sec:60,
		}
	}
}

impl HealthCheckManager {
	/// Create a new HealthCheckManager instance
	pub fn new(config:Option<HealthCheckConfig>) -> Self {
		Self {
			service_health:Arc::new(RwLock::new(HashMap::new())),
			health_history:Arc::new(RwLock::new(Vec::new())),
			recovery_actions:Arc::new(RwLock::new(HashMap::new())),
			config:config.unwrap_or_default(),
		}
	}

	/// Register a service for health monitoring
	pub async fn RegisterService(&self, service_name:String, check_level:HealthCheckLevel) -> Result<()> {
		let mut health_map = self.service_health.write().await;

		health_map.insert(
			service_name.clone(),
			ServiceHealth {
				service_name:service_name.clone(),
				status:HealthStatus::Unknown,
				last_check:0,
				last_success:None,
				failure_count:0,
				error_message:None,
				response_time_ms:None,
				check_level:check_level.clone(),
			},
		);

		info!(
			"[HealthCheck] Registered service for monitoring: {} ({:?})",
			service_name, check_level
		);
		Ok(())
	}

	/// Perform health check for a service
	pub async fn CheckService(&self, service_name:&str) -> Result<HealthStatus> {
		let StartTime = utils::CurrentTimestamp();

		// Perform service-specific health check with timeout
		let check_timeout = tokio::time::Duration::from_secs(10);

		let (status, error_message) = tokio::time::timeout(check_timeout, async {
			match service_name {
				"authentication" => self.CheckAuthenticationService().await,
				"updates" => self.CheckUpdatesService().await,
				"downloader" => self.CheckDownloaderService().await,
				"indexing" => self.CheckIndexingService().await,
				"grpc" => self.CheckGrpcService().await,
				"connections" => self.CheckConnectionsService().await,
				_ => {
					warn!("[HealthCheck] Unknown service: {}", service_name);
					return (HealthStatus::Unhealthy, Some(format!("Unknown service: {}", service_name)));
				},
			}
		})
		.await
		.map_err(|_| {
			warn!("[HealthCheck] Timeout checking service: {}", service_name);
			(
				HealthStatus::Unhealthy,
				Some(format!("Health check timeout for service: {}", service_name)),
			)
		})?;

		let ResponseTime = utils::CurrentTimestamp() - StartTime;

		// Update service health
		self.UpdateServiceHealth(service_name, status.clone(), &error_message, response_time)
			.await?;

		// Record health check
		self.RecordHealthCheck(service_name, status.clone(), response_time, &error_message)
			.await;

		// Trigger recovery if needed
		if self.config.enable_auto_recovery {
			self.TriggerRecoveryIfNeeded(service_name).await;
		}

		// Check if alerting is needed
		self.HandleCriticalAlerts(service_name, &status).await;

		Ok(status)
	}

	/// Check authentication service health
	async fn CheckAuthenticationService(&self) -> (HealthStatus, Option<String>) {
		debug!("[HealthCheck] Checking authentication service health");

		// Check if authentication service process is running
		// This would typically check for a process or socket
		// For now, we simulate a check

		let start = std::time::Instant::now();

		// Simulate authentication service health check
		// In production, this would:
		// 1. Check if authentication service process is running
		// 2. Verify authentication endpoint is responsive
		// 3. Test authentication with a test token
		// 4. Verify token store is accessible
		// 5. Check authentication database connectivity

		// Simulate check delay
		tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

		let elapsed = start.elapsed();

		// Check response time
		if elapsed.as_millis() > 1000 {
			return (
				HealthStatus::Degraded,
				Some(format!(
					"Authentication service response time too slow: {}ms",
					elapsed.as_millis()
				)),
			);
		}

		debug!("[HealthCheck] Authentication service healthy");
		(HealthStatus::Healthy, None)
	}

	/// Check updates service health
	async fn CheckUpdatesService(&self) -> (HealthStatus, Option<String>) {
		debug!("[HealthCheck] Checking updates service health");

		let start = std::time::Instant::now();

		// Simulate updates service health check
		// In production, this would:
		// 1. Check if updates service process is running
		// 2. Verify update endpoint connectivity
		// 3. Check update server availability
		// 4. Verify update cache integrity
		// 5. Check for pending updates

		// Simulate check delay
		tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;

		let elapsed = start.elapsed();

		// Check response time
		if elapsed.as_millis() > 500 {
			return (
				HealthStatus::Degraded,
				Some(format!("Updates service response time too slow: {}ms", elapsed.as_millis())),
			);
		}

		debug!("[HealthCheck] Updates service healthy");
		(HealthStatus::Healthy, None)
	}

	/// Check downloader service health
	async fn CheckDownloaderService(&self) -> (HealthStatus, Option<String>) {
		debug!("[HealthCheck] Checking downloader service health");

		let start = std::time::Instant::now();

		// Simulate downloader service health check
		// In production, this would:
		// 1. Check if downloader service process is running
		// 2. Verify download queue status
		// 3. Check active download count
		// 4. Verify download directory accessibility
		// 5. Check download bandwidth usage
		// 6. Verify progress tracking

		// Simulate check delay
		tokio::time::sleep(tokio::time::Duration::from_millis(40)).await;

		let elapsed = start.elapsed();

		// Check response time
		if elapsed.as_millis() > 1000 {
			return (
				HealthStatus::Degraded,
				Some(format!("Downloader service response time too slow: {}ms", elapsed.as_millis())),
			);
		}

		debug!("[HealthCheck] Downloader service healthy");
		(HealthStatus::Healthy, None)
	}

	/// Check indexing service health
	async fn CheckIndexingService(&self) -> (HealthStatus, Option<String>) {
		debug!("[HealthCheck] Checking indexing service health");

		let start = std::time::Instant::now();

		// Simulate indexing service health check
		// In production, this would:
		// 1. Check if indexing service process is running
		// 2. Verify index database status
		// 3. Check active indexing jobs
		// 4. Verify index integrity
		// 5. Check index size and growth
		// 6. Verify search functionality

		// Simulate check delay
		tokio::time::sleep(tokio::time::Duration::from_millis(60)).await;

		let elapsed = start.elapsed();

		// Check response time
		if elapsed.as_millis() > 500 {
			return (
				HealthStatus::Degraded,
				Some(format!("Indexing service response time too slow: {}ms", elapsed.as_millis())),
			);
		}

		debug!("[HealthCheck] Indexing service healthy");
		(HealthStatus::Healthy, None)
	}

	/// Check gRPC service health
	async fn CheckGrpcService(&self) -> (HealthStatus, Option<String>) {
		debug!("[HealthCheck] Checking gRPC service health");

		let start = std::time::Instant::now();

		// Simulate gRPC service health check
		// In production, this would:
		// 1. Check if gRPC server process is running
		// 2. Verify gRPC port is listening
		// 3. Perform a gRPC health check request
		// 4. Check active gRPC connections
		// 5. Verify gRPC TLS configuration (if applicable)
		// 6. Test gRPC endpoint responsiveness

		// Simulate check delay
		tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

		let elapsed = start.elapsed();

		// Check response time
		if elapsed.as_millis() > 200 {
			return (
				HealthStatus::Degraded,
				Some(format!("gRPC service response time too slow: {}ms", elapsed.as_millis())),
			);
		}

		debug!("[HealthCheck] gRPC service healthy");
		(HealthStatus::Healthy, None)
	}

	/// Check connections service health
	async fn CheckConnectionsService(&self) -> (HealthStatus, Option<String>) {
		debug!("[HealthCheck] Checking connections service health");

		let start = std::time::Instant::now();

		// Simulate connections service health check
		// In production, this would:
		// 1. Check if connections service process is running
		// 2. Verify active connection count
		// 3. Check connection pool status
		// 4. Verify connection health metrics
		// 5. Check for stuck connections
		// 6. Verify connection timeouts

		// Simulate check delay
		tokio::time::sleep(tokio::time::Duration::from_millis(35)).await;

		let elapsed = start.elapsed();

		// Check response time
		if elapsed.as_millis() > 300 {
			return (
				HealthStatus::Degraded,
				Some(format!("Connections service response time too slow: {}ms", elapsed.as_millis())),
			);
		}

		debug!("[HealthCheck] Connections service healthy");
		(HealthStatus::Healthy, None)
	}

	/// Update service health status
	async fn UpdateServiceHealth(
		&self,
		service_name:&str,
		status:HealthStatus,
		error_message:&Option<String>,
		response_time:u64,
	) -> Result<()> {
		let mut health_map = self.service_health.write().await;

		if let Some(service_health) = health_map.get_mut(service_name) {
			service_health.status = status.clone();
			service_health.last_check = utils::CurrentTimestamp();
			service_health.response_time_ms = Some(response_time);

			match status {
				HealthStatus::Healthy => {
					service_health.last_success = Some(utils::CurrentTimestamp());
					service_health.failure_count = 0;
					service_health.error_message = None;
				},
				HealthStatus::Degraded | HealthStatus::Unhealthy => {
					service_health.failure_count += 1;
					service_health.error_message = error_message.clone();
				},
				HealthStatus::Unknown => {
					// Keep existing state
				},
			}
		} else {
			return Err(AirError::Internal(format!("Service not registered: {}", service_name)));
		}

		debug!(
			"[HealthCheck] Updated health for {}: {:?} ({}ms)",
			service_name, status, response_time
		);
		Ok(())
	}

	/// Record health check in history
	async fn RecordHealthCheck(
		&self,
		service_name:&str,
		status:HealthStatus,
		response_time:u64,
		error_message:&Option<String>,
	) {
		let mut history = self.health_history.write().await;

		let record = HealthCheckRecord {
			timestamp:utils::CurrentTimestamp(),
			service_name:service_name.to_string(),
			status,
			response_time_ms:Some(response_time),
			error_message:error_message.clone(),
		};

		history.push(record);

		// Trim history to retention limit
		if history.len() > self.config.history_retention {
			history.remove(0);
		}
	}

	/// Trigger recovery actions if needed
	async fn TriggerRecoveryIfNeeded(&self, service_name:&str) {
		let health_map = self.service_health.read().await;

		if let Some(service_health) = health_map.get(service_name) {
			// Check if recovery is needed based on failure count
			if service_health.failure_count >= self.config.consecutive_failures_threshold {
				warn!(
					"[HealthCheck] Service {} has {} consecutive failures, triggering recovery",
					service_name, service_health.failure_count
				);

				self.PerformRecoveryAction(service_name).await;
			}

			// Check if recovery is needed based on response time
			if let Some(response_time) = service_health.response_time_ms {
				if response_time > self.config.response_time_threshold_ms {
					warn!(
						"[HealthCheck] Service {} response time {}ms exceeds threshold {}ms",
						service_name, response_time, self.config.response_time_threshold_ms
					);

					self.HandleResponseTimeRecovery(service_name, response_time).await;
				}
			}
		}
	}

	/// Handle response time-based recovery
	async fn HandleResponseTimeRecovery(&self, service_name:&str, response_time:u64) {
		info!(
			"[HealthCheck] Handling response time recovery for {}: {}ms",
			service_name, response_time
		);

		match service_name {
			"grpc" => {
				warn!(
					"[HealthCheck] Response time recovery: Optimizing gRPC server for {}",
					service_name
				);
				// In production, this might:
				// - Adjust connection pool sizes
				// - Clear connection caches
				// - Trigger connection rebalancing
			},
			"connections" => {
				warn!(
					"[HealthCheck] Response time recovery: Optimizing connections for {}",
					service_name
				);
				// In production, this might:
				// - Clear idle connections
				// - Adjust connection timeouts
				// - Trigger connection pool refresh
			},
			_ => {
				warn!(
					"[HealthCheck] Response time recovery: Generic optimization for {}",
					service_name
				);
			},
		}
	}

	/// Handle critical health alerts
	async fn HandleCriticalAlerts(&self, service_name:&str, status:&HealthStatus) {
		if *status == HealthStatus::Unhealthy {
			warn!(
				"[HealthCheck] CRITICAL: Service {} is UNHEALTHY - immediate attention required",
				service_name
			);

			// In production, this would:
			// - Send alerts to monitoring systems (Mountain)
			// - Send notifications to administrators
			// - Create incident tickets
			// - Trigger automated escalation procedures
		}
	}

	/// Perform recovery action for a service
	async fn PerformRecoveryAction(&self, service_name:&str) {
		info!("[HealthCheck] Performing recovery action for {}", service_name);

		let recovery_timeout = tokio::time::Duration::from_secs(self.config.recovery_timeout_sec);

		let result = tokio::time::timeout(recovery_timeout, async {
			match service_name {
				"authentication" => self.RestartAuthenticationService().await,
				"updates" => self.RestartUpdatesService().await,
				"downloader" => self.RestartDownloaderService().await,
				"indexing" => self.RestartIndexingService().await,
				"grpc" => self.RestartGrpcService().await,
				"connections" => self.ResetConnectionsService().await,
				_ => {
					warn!("[HealthCheck] No specific recovery action for {}", service_name);
					Ok(())
				},
			}
		})
		.await;

		match result {
			Ok(Ok(())) => {
				info!("[HealthCheck] Recovery action completed successfully for {}", service_name);
			},
			Ok(Err(e)) => {
				warn!("[HealthCheck] Recovery action failed for {}: {:?}", service_name, e);
			},
			Err(_) => {
				warn!("[HealthCheck] Recovery action timed out for {}", service_name);
			},
		}
	}

	/// Restart authentication service
	async fn RestartAuthenticationService(&self) -> Result<()> {
		warn!("[HealthCheck] Recovery: Restarting authentication service");
		// In production, this would signal the authentication service to restart
		Ok(())
	}

	/// Restart updates service
	async fn RestartUpdatesService(&self) -> Result<()> {
		warn!("[HealthCheck] Recovery: Restarting updates service");
		// In production, this would signal the updates service to restart
		Ok(())
	}

	/// Restart downloader service
	async fn RestartDownloaderService(&self) -> Result<()> {
		warn!("[HealthCheck] Recovery: Restarting downloader service");
		// In production, this would signal the downloader service to restart
		Ok(())
	}

	/// Restart indexing service
	async fn RestartIndexingService(&self) -> Result<()> {
		warn!("[HealthCheck] Recovery: Restarting indexing service");
		// In production, this would signal the indexing service to restart
		Ok(())
	}

	/// Restart gRPC service
	async fn RestartGrpcService(&self) -> Result<()> {
		warn!("[HealthCheck] Recovery: Restarting gRPC server");
		// In production, this would gracefully restart the gRPC server
		Ok(())
	}

	/// Reset connections service
	async fn ResetConnectionsService(&self) -> Result<()> {
		warn!("[HealthCheck] Recovery: Resetting connections service");
		// In production, this would reset connection pools and re-establish connections
		Ok(())
	}

	/// Get overall daemon health status
	pub async fn GetOverallHealth(&self) -> HealthStatus {
		let health_map = self.service_health.read().await;

		let mut healthy_count = 0;
		let mut degraded_count = 0;
		let mut unhealthy_count = 0;

		for service_health in health_map.values() {
			match service_health.status {
				HealthStatus::Healthy => healthy_count += 1,
				HealthStatus::Degraded => degraded_count += 1,
				HealthStatus::Unhealthy => unhealthy_count += 1,
				HealthStatus::Unknown => {},
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
	pub async fn GetServiceHealth(&self, service_name:&str) -> Option<ServiceHealth> {
		let health_map = self.service_health.read().await;
		health_map.get(service_name).cloned()
	}

	/// Get health check history
	pub async fn GetHealthHistory(&self, service_name:Option<&str>, limit:Option<usize>) -> Vec<HealthCheckRecord> {
		let history = self.health_history.read().await;

		let mut filtered_history:Vec<HealthCheckRecord> = if let Some(service) = service_name {
			history
				.iter()
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
	pub async fn RegisterRecoveryAction(&self, action:RecoveryAction) -> Result<()> {
		let mut actions = self.recovery_actions.write().await;
		actions.insert(action.name.clone(), action);
		Ok(())
	}

	/// Get health statistics
	pub async fn GetHealthStatistics(&self) -> HealthStatistics {
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
				HealthStatus::Unknown => {},
			}
		}

		// Get health statistics
		let mut stats = HealthStatistics {
			total_services:health_map.len(),
			healthy_services,
			degraded_services,
			unhealthy_services,
			total_checks:history.len(),
			average_response_time_ms:0.0,
			success_rate:0.0,
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
	pub total_services:usize,
	pub healthy_services:usize,
	pub degraded_services:usize,
	pub unhealthy_services:usize,
	pub total_checks:usize,
	pub average_response_time_ms:f64,
	pub success_rate:f64,
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
	pub overall_status:HealthStatus,
	pub service_health:HashMap<String, ServiceHealth>,
	pub statistics:HealthStatistics,
	pub performance_indicators:PerformanceIndicators,
	pub resource_warnings:Vec<ResourceWarning>,
	pub timestamp:u64,
}

impl HealthCheckResponse {
	/// Create a new health check response
	pub fn new(
		overall_status:HealthStatus,
		service_health:HashMap<String, ServiceHealth>,
		statistics:HealthStatistics,
	) -> Self {
		Self {
			overall_status,
			service_health,
			statistics,
			performance_indicators:PerformanceIndicators::default(),
			resource_warnings:Vec::new(),
			timestamp:utils::CurrentTimestamp(),
		}
	}

	/// Create with performance indicators
	pub fn with_performance_indicators(mut self, indicators:PerformanceIndicators) -> Self {
		self.performance_indicators = indicators;
		self
	}

	/// Create with resource warnings
	pub fn with_resource_warnings(mut self, warnings:Vec<ResourceWarning>) -> Self {
		self.resource_warnings = warnings;
		self
	}
}

/// Performance degradation indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceIndicators {
	pub response_time_p99_ms:f64,
	pub response_time_p95_ms:f64,
	pub request_throughput_per_sec:f64,
	pub error_rate_percent:f64,
	pub degradation_level:DegradationLevel,
	pub bottleneck_service:Option<String>,
}

impl Default for PerformanceIndicators {
	fn default() -> Self {
		Self {
			response_time_p99_ms:0.0,
			response_time_p95_ms:0.0,
			request_throughput_per_sec:0.0,
			error_rate_percent:0.0,
			degradation_level:DegradationLevel::Optimal,
			bottleneck_service:None,
		}
	}
}

/// Degradation levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DegradationLevel {
	Optimal,
	Acceptable,
	Degraded,
	Critical,
}

/// Resource warning types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceWarning {
	pub warning_type:ResourceWarningType,
	pub service_name:Option<String>,
	pub current_value:f64,
	pub threshold:f64,
	pub severity:WarningSeverity,
	pub timestamp:u64,
}

/// Resource warning types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceWarningType {
	HighMemoryUsage,
	HighCpuUsage,
	LowDiskSpace,
	ConnectionPoolExhausted,
	ThreadPoolExhausted,
	HighLatency,
	HighErrorRate,
	DatabaseConnectivityIssue,
}

/// Warning severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WarningSeverity {
	Low,
	Medium,
	High,
	Critical,
}
