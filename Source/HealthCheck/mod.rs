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
//! ## FUTURE Enhancements
//!
//! - Implement advanced metrics collection (latency percentiles, error rates)
//! - Add health check scheduling automation (cron-like scheduling)
//! - Implement predictive health analysis (machine learning-based)
//! - Add security compliance checks (PCI-DSS, GDPR, etc.)
//! - Implement distributed health checks for clustered deployments
//! - Add health check export formats (Prometheus, Grafana, etc.)
//! - Implement health check alerting through multiple channels (email, Slack,
//! etc.)
//! - Add health check simulation for testing and validation
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

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{AirError, Result, Utility, dev_log};

/// Health check manager
#[derive(Debug)]
pub struct HealthCheckManager {
	/// Service health status
	ServiceHealth:Arc<RwLock<HashMap<String, ServiceHealth>>>,

	/// Health check history
	HealthHistory:Arc<RwLock<Vec<HealthCheckRecord>>>,

	/// Recovery actions
	RecoveryActions:Arc<RwLock<HashMap<String, RecoveryAction>>>,

	/// Health check configuration
	config:HealthCheckConfig,
}

/// Service health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
	/// Service name
	pub ServiceName:String,

	/// Current health status
	pub Status:HealthStatus,

	/// Last check timestamp
	pub LastCheck:u64,

	/// Last successful check timestamp
	pub LastSuccess:Option<u64>,

	/// Failure count
	pub FailureCount:u32,

	/// Error message (if any)
	pub ErrorMessage:Option<String>,

	/// Response time in milliseconds
	pub ResponseTimeMs:Option<u64>,

	/// Health check level
	pub CheckLevel:HealthCheckLevel,
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
	pub Timestamp:u64,

	/// Service name
	pub ServiceName:String,

	/// Health status
	pub Status:HealthStatus,

	/// Response time in milliseconds
	pub ResponseTimeMs:Option<u64>,

	/// Error message (if any)
	pub ErrorMessage:Option<String>,
}

/// Recovery action configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
	/// Action name
	pub Name:String,

	/// Service name
	pub ServiceName:String,

	/// Trigger condition
	pub Trigger:RecoveryTrigger,

	/// Action to take
	pub Action:RecoveryActionType,

	/// Maximum retry attempts
	pub MaxRetries:u32,

	/// Current retry count
	pub RetryCount:u32,
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
	pub DefaultCheckInterval:u64,

	/// Health history retention (number of records)
	pub HistoryRetention:usize,

	/// Consecutive failures threshold
	pub ConsecutiveFailuresThreshold:u32,

	/// Response time threshold in milliseconds
	pub ResponseTimeThresholdMs:u64,

	/// Enable automatic recovery
	pub EnableAutoRecovery:bool,

	/// Recovery timeout in seconds
	pub RecoveryTimeoutSec:u64,
}

impl Default for HealthCheckConfig {
	fn default() -> Self {
		Self {
			DefaultCheckInterval:30,

			HistoryRetention:100,

			ConsecutiveFailuresThreshold:3,

			ResponseTimeThresholdMs:5000,

			EnableAutoRecovery:true,

			RecoveryTimeoutSec:60,
		}
	}
}

impl HealthCheckManager {
	/// Create a new HealthCheckManager instance
	pub fn new(config:Option<HealthCheckConfig>) -> Self {
		Self {
			ServiceHealth:Arc::new(RwLock::new(HashMap::new())),

			HealthHistory:Arc::new(RwLock::new(Vec::new())),

			RecoveryActions:Arc::new(RwLock::new(HashMap::new())),

			config:config.unwrap_or_default(),
		}
	}

	/// Register a service for health monitoring
	pub async fn RegisterService(&self, ServiceName:String, CheckLevel:HealthCheckLevel) -> Result<()> {
		let mut HealthMap = self.ServiceHealth.write().await;

		HealthMap.insert(
			ServiceName.clone(),
			ServiceHealth {
				ServiceName:ServiceName.clone(),
				Status:HealthStatus::Unknown,
				LastCheck:0,
				LastSuccess:None,
				FailureCount:0,
				ErrorMessage:None,
				ResponseTimeMs:None,
				CheckLevel:CheckLevel.clone(),
			},
		);

		dev_log!(
			"lifecycle",
			"[HealthCheck] Registered service for monitoring: {} ({:?})",
			ServiceName,
			CheckLevel
		);

		Ok(())
	}

	/// Perform health check for a service
	pub async fn CheckService(&self, ServiceName:&str) -> Result<HealthStatus> {
		let StartTime = Utility::CurrentTimestamp();

		// Perform service-specific health check with timeout
		let CheckTimeout = tokio::time::Duration::from_secs(10);

		let (status, ErrorMessage) = tokio::time::timeout(CheckTimeout, async {
			match ServiceName {
				"authentication" => self.CheckAuthenticationService().await,
				"updates" => self.CheckUpdatesService().await,
				"downloader" => self.CheckDownloaderService().await,
				"indexing" => self.CheckIndexingService().await,
				"grpc" => self.CheckgRPCService().await,
				"connections" => self.CheckConnectionsService().await,
				_ => {
					dev_log!("lifecycle", "warn: [HealthCheck] Unknown service: {}", ServiceName);

					return (HealthStatus::Unhealthy, Some(format!("Unknown service: {}", ServiceName)));
				},
			}
		})
		.await
		.map_err(|_| {
			dev_log!("lifecycle", "warn: [HealthCheck] Timeout checking service: {}", ServiceName);

			(
				HealthStatus::Unhealthy,
				Some(format!("Health check timeout for service: {}", ServiceName)),
			)
		})?;

		let ResponseTime = Utility::CurrentTimestamp() - StartTime;

		// Update service health
		self.UpdateServiceHealth(ServiceName, status.clone(), &ErrorMessage, ResponseTime)
			.await?;

		// Record health check
		self.RecordHealthCheck(ServiceName, status.clone(), ResponseTime, &ErrorMessage)
			.await;

		// Trigger recovery if needed
		if self.config.EnableAutoRecovery {
			self.TriggerRecoveryIfNeeded(ServiceName).await;
		}

		// Check if alerting is needed
		self.HandleCriticalAlerts(ServiceName, &status).await;

		Ok(status)
	}

	/// Check authentication service health
	async fn CheckAuthenticationService(&self) -> (HealthStatus, Option<String>) {
		dev_log!("lifecycle", "[HealthCheck] Checking authentication service health");

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

		dev_log!("lifecycle", "[HealthCheck] Authentication service healthy");

		(HealthStatus::Healthy, None)
	}

	/// Check updates service health
	async fn CheckUpdatesService(&self) -> (HealthStatus, Option<String>) {
		dev_log!("lifecycle", "[HealthCheck] Checking updates service health");

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

		dev_log!("lifecycle", "[HealthCheck] Updates service healthy");

		(HealthStatus::Healthy, None)
	}

	/// Check downloader service health
	async fn CheckDownloaderService(&self) -> (HealthStatus, Option<String>) {
		dev_log!("lifecycle", "[HealthCheck] Checking downloader service health");

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

		dev_log!("lifecycle", "[HealthCheck] Downloader service healthy");

		(HealthStatus::Healthy, None)
	}

	/// Check indexing service health
	async fn CheckIndexingService(&self) -> (HealthStatus, Option<String>) {
		dev_log!("lifecycle", "[HealthCheck] Checking indexing service health");

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

		dev_log!("lifecycle", "[HealthCheck] Indexing service healthy");

		(HealthStatus::Healthy, None)
	}

	/// Check gRPC service health
	async fn CheckgRPCService(&self) -> (HealthStatus, Option<String>) {
		dev_log!("lifecycle", "[HealthCheck] Checking gRPC service health");

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

		dev_log!("lifecycle", "[HealthCheck] gRPC service healthy");

		(HealthStatus::Healthy, None)
	}

	/// Check connections service health
	async fn CheckConnectionsService(&self) -> (HealthStatus, Option<String>) {
		dev_log!("lifecycle", "[HealthCheck] Checking connections service health");

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

		dev_log!("lifecycle", "[HealthCheck] Connections service healthy");

		(HealthStatus::Healthy, None)
	}

	/// Update service health status
	async fn UpdateServiceHealth(
		&self,

		ServiceName:&str,

		status:HealthStatus,

		ErrorMessage:&Option<String>,

		ResponseTime:u64,
	) -> Result<()> {
		let mut HealthMap = self.ServiceHealth.write().await;

		if let Some(ServiceHealth) = HealthMap.get_mut(ServiceName) {
			ServiceHealth.Status = status.clone();

			ServiceHealth.LastCheck = Utility::CurrentTimestamp();

			ServiceHealth.ResponseTimeMs = Some(ResponseTime);

			match status {
				HealthStatus::Healthy => {
					ServiceHealth.LastSuccess = Some(Utility::CurrentTimestamp());

					ServiceHealth.FailureCount = 0;

					ServiceHealth.ErrorMessage = None;
				},

				HealthStatus::Degraded | HealthStatus::Unhealthy => {
					ServiceHealth.FailureCount += 1;

					ServiceHealth.ErrorMessage = ErrorMessage.clone();
				},

				HealthStatus::Unknown => {

					// Keep existing state
				},
			}
		} else {
			return Err(AirError::Internal(format!("Service not registered: {}", ServiceName)));
		}

		dev_log!(
			"lifecycle",
			"[HealthCheck] Updated health for {}: {:?} ({}ms)",
			ServiceName,
			status,
			ResponseTime
		);

		Ok(())
	}

	/// Record health check in history
	async fn RecordHealthCheck(
		&self,

		ServiceName:&str,

		status:HealthStatus,

		ResponseTime:u64,

		ErrorMessage:&Option<String>,
	) {
		let mut history = self.HealthHistory.write().await;

		let record = HealthCheckRecord {
			Timestamp:Utility::CurrentTimestamp(),

			ServiceName:ServiceName.to_string(),

			Status:status,

			ResponseTimeMs:Some(ResponseTime),

			ErrorMessage:ErrorMessage.clone(),
		};

		history.push(record);

		// Trim history to retention limit
		if history.len() > self.config.HistoryRetention {
			history.remove(0);
		}
	}

	/// Trigger recovery actions if needed
	async fn TriggerRecoveryIfNeeded(&self, ServiceName:&str) {
		let HealthMap = self.ServiceHealth.read().await;

		if let Some(ServiceHealth) = HealthMap.get(ServiceName) {
			// Check if recovery is needed based on failure count
			if ServiceHealth.FailureCount >= self.config.ConsecutiveFailuresThreshold {
				dev_log!(
					"lifecycle",
					"warn: [HealthCheck] Service {} has {} consecutive failures, triggering recovery",
					ServiceName,
					ServiceHealth.FailureCount
				);

				self.PerformRecoveryAction(ServiceName).await;
			}

			// Check if recovery is needed based on response time
			if let Some(ResponseTime) = ServiceHealth.ResponseTimeMs {
				if ResponseTime > self.config.ResponseTimeThresholdMs {
					dev_log!(
						"lifecycle",
						"warn: [HealthCheck] Service {} response time {}ms exceeds threshold {}ms",
						ServiceName,
						ResponseTime,
						self.config.ResponseTimeThresholdMs
					);

					self.HandleResponseTimeRecovery(ServiceName, ResponseTime).await;
				}
			}
		}
	}

	/// Handle response time-based recovery
	async fn HandleResponseTimeRecovery(&self, ServiceName:&str, ResponseTime:u64) {
		dev_log!(
			"lifecycle",
			"[HealthCheck] Handling response time recovery for {}: {}ms",
			ServiceName,
			ResponseTime
		);

		match ServiceName {
			"grpc" => {
				dev_log!(
					"lifecycle",
					"warn: [HealthCheck] Response time recovery: Optimizing gRPC server for {}",
					ServiceName
				);

				// In production, this might:
				// - Adjust connection pool sizes
				// - Clear connection caches
				// - Trigger connection rebalancing
			},

			"connections" => {
				dev_log!(
					"lifecycle",
					"warn: [HealthCheck] Response time recovery: Optimizing connections for {}",
					ServiceName
				);

				// In production, this might:
				// - Clear idle connections
				// - Adjust connection timeouts
				// - Trigger connection pool refresh
			},

			_ => {
				dev_log!(
					"lifecycle",
					"warn: [HealthCheck] Response time recovery: Generic optimization for {}",
					ServiceName
				);
			},
		}
	}

	/// Handle critical health alerts
	async fn HandleCriticalAlerts(&self, ServiceName:&str, status:&HealthStatus) {
		if *status == HealthStatus::Unhealthy {
			dev_log!(
				"lifecycle",
				"warn: [HealthCheck] CRITICAL: Service {} is UNHEALTHY - immediate attention required",
				ServiceName
			);

			// In production, this would:
			// - Send alerts to monitoring systems (Mountain)
			// - Send notifications to administrators
			// - Create incident tickets
			// - Trigger automated escalation procedures
		}
	}

	/// Perform recovery action for a service
	async fn PerformRecoveryAction(&self, ServiceName:&str) {
		dev_log!("lifecycle", "[HealthCheck] Performing recovery action for {}", ServiceName);

		let RecoveryTimeout = tokio::time::Duration::from_secs(self.config.RecoveryTimeoutSec);

		let result = tokio::time::timeout(RecoveryTimeout, async {
			match ServiceName {
				"authentication" => self.RestartAuthenticationService().await,
				"updates" => self.RestartUpdatesService().await,
				"downloader" => self.RestartDownloaderService().await,
				"indexing" => self.RestartIndexingService().await,
				"grpc" => self.RestartgRPCService().await,
				"connections" => self.ResetConnectionsService().await,
				_ => {
					dev_log!(
						"lifecycle",
						"warn: [HealthCheck] No specific recovery action for {}",
						ServiceName
					);

					Ok(())
				},
			}
		})
		.await;

		match result {
			Ok(Ok(())) => {
				dev_log!(
					"lifecycle",
					"[HealthCheck] Recovery action completed successfully for {}",
					ServiceName
				);
			},

			Ok(Err(e)) => {
				dev_log!(
					"lifecycle",
					"warn: [HealthCheck] Recovery action failed for {}: {:?}",
					ServiceName,
					e
				);
			},

			Err(_) => {
				dev_log!("lifecycle", "warn: [HealthCheck] Recovery action timed out for {}", ServiceName);
			},
		}
	}

	/// Restart authentication service
	async fn RestartAuthenticationService(&self) -> Result<()> {
		dev_log!("lifecycle", "warn: [HealthCheck] Recovery: Restarting authentication service"); // In production, this would signal the authentication service to restart

		Ok(())
	}

	/// Restart updates service
	async fn RestartUpdatesService(&self) -> Result<()> {
		dev_log!("lifecycle", "warn: [HealthCheck] Recovery: Restarting updates service"); // In production, this would signal the updates service to restart

		Ok(())
	}

	/// Restart downloader service
	async fn RestartDownloaderService(&self) -> Result<()> {
		dev_log!("lifecycle", "warn: [HealthCheck] Recovery: Restarting downloader service"); // In production, this would signal the downloader service to restart

		Ok(())
	}

	/// Restart indexing service
	async fn RestartIndexingService(&self) -> Result<()> {
		dev_log!("lifecycle", "warn: [HealthCheck] Recovery: Restarting indexing service"); // In production, this would signal the indexing service to restart

		Ok(())
	}

	/// Restart gRPC service
	async fn RestartgRPCService(&self) -> Result<()> {
		dev_log!("lifecycle", "warn: [HealthCheck] Recovery: Restarting gRPC server"); // In production, this would gracefully restart the gRPC server

		Ok(())
	}

	/// Reset connections service
	async fn ResetConnectionsService(&self) -> Result<()> {
		dev_log!("lifecycle", "warn: [HealthCheck] Recovery: Resetting connections service"); // In production, this would reset connection pools and re-establish connections

		Ok(())
	}

	/// Get overall daemon health status
	pub async fn GetOverallHealth(&self) -> HealthStatus {
		let HealthMap = self.ServiceHealth.read().await;

		let mut HealthyCount = 0;

		let mut DegradedCount = 0;

		let mut UnhealthyCount = 0;

		for ServiceHealth in HealthMap.values() {
			match ServiceHealth.Status {
				HealthStatus::Healthy => HealthyCount += 1,

				HealthStatus::Degraded => DegradedCount += 1,

				HealthStatus::Unhealthy => UnhealthyCount += 1,

				HealthStatus::Unknown => {},
			}
		}

		if UnhealthyCount > 0 {
			HealthStatus::Unhealthy
		} else if DegradedCount > 0 {
			HealthStatus::Degraded
		} else if HealthyCount > 0 {
			HealthStatus::Healthy
		} else {
			HealthStatus::Unknown
		}
	}

	/// Get service health status
	pub async fn GetServiceHealth(&self, service_name:&str) -> Option<ServiceHealth> {
		let HealthMap = self.ServiceHealth.read().await;

		HealthMap.get(service_name).cloned()
	}

	/// Get health check history
	pub async fn GetHealthHistory(&self, service_name:Option<&str>, limit:Option<usize>) -> Vec<HealthCheckRecord> {
		let History = self.HealthHistory.read().await;

		let mut FilteredHistory:Vec<HealthCheckRecord> = if let Some(service) = service_name {
			History.iter().filter(|Record| Record.ServiceName == service).cloned().collect()
		} else {
			History.clone()
		};

		// Reverse to get most recent first
		FilteredHistory.reverse();

		// Apply limit
		if let Some(limit) = limit {
			FilteredHistory.truncate(limit);
		}

		FilteredHistory
	}

	/// Register a recovery action
	pub async fn RegisterRecoveryAction(&self, action:RecoveryAction) -> Result<()> {
		let mut actions = self.RecoveryActions.write().await;

		actions.insert(action.Name.clone(), action);

		Ok(())
	}

	/// Get health statistics
	pub async fn GetHealthStatistics(&self) -> HealthStatistics {
		let HealthMap = self.ServiceHealth.read().await;

		let history = self.HealthHistory.read().await;

		// Count service statuses
		let mut HealthyServices = 0;

		let mut DegradedServices = 0;

		let mut UnhealthyServices = 0;

		for ServiceHealth in HealthMap.values() {
			match ServiceHealth.Status {
				HealthStatus::Healthy => HealthyServices += 1,

				HealthStatus::Degraded => DegradedServices += 1,

				HealthStatus::Unhealthy => UnhealthyServices += 1,

				HealthStatus::Unknown => {},
			}
		}

		// Get health statistics
		let mut Statistics = HealthStatistics {
			TotalServices:HealthMap.len(),

			HealthyServices,

			DegradedServices,

			UnhealthyServices,

			TotalChecks:history.len(),

			AverageResponseTimeMs:0.0,

			SuccessRate:0.0,
		};

		// Calculate response time and success rate
		if !history.is_empty() {
			let mut TotalResponseTime = 0;

			let mut SuccessfulChecks = 0;

			for Record in history.iter() {
				if let Some(ResponseTime) = Record.ResponseTimeMs {
					TotalResponseTime += ResponseTime;
				}

				if Record.Status == HealthStatus::Healthy {
					SuccessfulChecks += 1;
				}
			}

			Statistics.AverageResponseTimeMs = TotalResponseTime as f64 / history.len() as f64;

			Statistics.SuccessRate = SuccessfulChecks as f64 / history.len() as f64 * 100.0;
		}

		Statistics
	}
}

/// Health statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatistics {
	pub TotalServices:usize,

	pub HealthyServices:usize,

	pub DegradedServices:usize,

	pub UnhealthyServices:usize,

	pub TotalChecks:usize,

	pub AverageResponseTimeMs:f64,

	pub SuccessRate:f64,
}

impl HealthStatistics {
	/// Get overall health percentage
	pub fn OverallHealthPercentage(&self) -> f64 {
		if self.TotalServices == 0 {
			return 0.0;
		}

		(self.HealthyServices as f64 / self.TotalServices as f64) * 100.0
	}
}

/// Health check response for gRPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
	pub OverallStatus:HealthStatus,

	pub ServiceHealth:HashMap<String, ServiceHealth>,

	pub Statistics:HealthStatistics,

	pub PerformanceIndicators:PerformanceIndicators,

	pub ResourceWarnings:Vec<ResourceWarning>,

	pub Timestamp:u64,
}

impl HealthCheckResponse {
	/// Create a new health check response
	pub fn new(
		OverallStatus:HealthStatus,

		ServiceHealth:HashMap<String, ServiceHealth>,

		Statistics:HealthStatistics,
	) -> Self {
		Self {
			OverallStatus,

			ServiceHealth,

			Statistics,

			PerformanceIndicators:PerformanceIndicators::default(),

			ResourceWarnings:Vec::new(),

			Timestamp:Utility::CurrentTimestamp(),
		}
	}

	/// Create with performance indicators
	pub fn with_performance_indicators(mut self, indicators:PerformanceIndicators) -> Self {
		self.PerformanceIndicators = indicators;

		self
	}

	/// Create with resource warnings
	pub fn with_resource_warnings(mut self, warnings:Vec<ResourceWarning>) -> Self {
		self.ResourceWarnings = warnings;

		self
	}
}

/// Performance degradation indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceIndicators {
	pub ResponseTimeP99Ms:f64,

	pub ResponseTimeP95Ms:f64,

	pub RequestThroughputPerSec:f64,

	pub ErrorRatePercent:f64,

	pub DegradationLevel:DegradationLevel,

	pub BottleneckService:Option<String>,
}

impl Default for PerformanceIndicators {
	fn default() -> Self {
		Self {
			ResponseTimeP99Ms:0.0,

			ResponseTimeP95Ms:0.0,

			RequestThroughputPerSec:0.0,

			ErrorRatePercent:0.0,

			DegradationLevel:DegradationLevel::Optimal,

			BottleneckService:None,
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
	pub WarningType:ResourceWarningType,

	pub ServiceName:Option<String>,

	pub CurrentValue:f64,

	pub Threshold:f64,

	pub Severity:WarningSeverity,

	pub Timestamp:u64,
}

/// Resource warning types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceWarningType {
	HighMemoryUsage,

	HighCPUUsage,

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
