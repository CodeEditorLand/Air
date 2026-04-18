//! # StartMonitoring
//!
//! ## File: Binary/Monitor/StartMonitoring.rs
//!
//! ## Role in Air Architecture
//!
//! Starts background monitoring tasks for resource usage, connection health,
//! and service health checks. These tasks run continuously while the daemon
//! is active.
//!
//! ## Primary Responsibility
//!
//! Start background monitoring tasks for daemon health and resources.
//!
//! ## Secondary Responsibilities
//!
//! - Monitor resource usage (CPU, memory, connections)
//! - Perform periodic health checks
//! - Update metrics with monitoring data
//! - Clean up stale connections
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `tokio::task` - Async task spawning
//! - `tokio::time` - Interval timers
//!
//! **Internal Modules:**
//! - `AirLibrary::ApplicationState` - Shared application state
//! - `AirLibrary::HealthCheck::HealthCheckManager` - Health check manager
//! - `AirLibrary::Metrics` - Metrics collection
//!
//! ## Dependents
//!
//! - `Binary::Binary::Main` - Starts monitoring after server initialization
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's monitoring in
//! `src/vs/platform/monitor/common/monitorService.ts`
//!
//! ## Security Considerations
//!
//! - Monitoring data doesn't contain sensitive information
//! - Health checks bypass auth (internal only)
//!
//! ## Performance Considerations
//!
//! - Tasks run on configurable intervals
//! - Lightweight operations to minimize overhead
//! - Async execution doesn't block main operations
//!
//! ## Error Handling Strategy
//!
//! - Individual monitoring failures logged but don't halt daemon
//! - Failed health checks recorded but continue monitoring
//! - Connection cleanup errors logged but don't stop task
//!
//! ## Thread Safety
//!
//! - Each monitor task runs independently
//! - Arc ensures thread-safe state sharing

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use std::{sync::Arc, time::Duration};

use tokio::time::interval;
use AirLibrary::{ApplicationState, HealthCheck::HealthCheckManager, Metrics};

use crate::dev_log;

/// Spawned monitoring task handles
///
/// Contains join handles for all monitoring background tasks.
pub struct MonitoringHandles {
	/// Connection monitor task handle
	pub ConnectionMonitor:tokio::task::JoinHandle<()>,
	/// Health monitor task handle
	pub HealthMonitor:tokio::task::JoinHandle<()>,
}

/// Start background monitoring tasks
///
/// Spawns two monitoring tasks:
/// 1. Connection monitor (every 60 seconds) - Updates resource usage, cleans
///    connections
/// 2. Health monitor (every 30 seconds) - Performs health checks on services
///
/// # Arguments
///
/// * `AppState` - Shared application state
/// * `HealthManager` - Health check manager for service monitoring
///
/// # Returns
///
/// Returns `MonitoringHandles` containing task join handles.
///
/// # Monitoring Intervals
///
/// - **Connection monitor**: 60 seconds
/// - **Health monitor**: 30 seconds
///
/// # Monitoring Configuration
///
/// - **Connection monitor**: 60 seconds
/// - **Health monitor**: 30 seconds
///
/// # Future Enhancements
///
/// - Make monitoring intervals configurable
/// - Add restart logic for failed services
/// - Implement alert thresholds
///
/// # Thread Count Monitoring
///
/// The monitoring system reports active thread count. This is calculated using
/// a heuristic that counts the number of active tokio tasks, which approximates
/// the number of concurrent operations. Note that this is not an exact thread
/// count as tokio uses work-stealing scheduling with a limited worker pool.
pub async fn StartMonitoring(
	AppState:Arc<ApplicationState>,
	HealthManager:Arc<HealthCheckManager>,
) -> MonitoringHandles {
	dev_log!("lifecycle", "[Monitor] Starting background monitoring tasks...");
	// Start connection monitoring background task
	let ConnectionMonitorHandle = tokio::spawn({
		let AppState = AppState.clone();
		let HealthManager = HealthManager.clone();

		async move {
			let mut Tick = interval(Duration::from_secs(60)); // Check every minute

			loop {
				Tick.tick().await;

				// Update resource usage with error handling
				if let Err(Error) = AppState.UpdateResourceUsage().await {
					dev_log!(
						"lifecycle",
						"warn: [ConnectionMonitor] Failed to update resource usage: {}",
						Error
					);
				}

				// Get resource metrics
				let Resources = AppState.GetResourceUsage().await;

				// Calculate active thread count approximation
				// This estimates concurrent operations by querying internal task count
				let ActiveThreads = AppState.GetActiveTaskCount().await.unwrap_or(0);

				// Record metrics
				let MetricsCollector = Metrics::GetMetrics();
				MetricsCollector.UpdateResourceMetrics(
					Resources.MemoryUsageMb.saturating_mul(1024).saturating_mul(1024), // Convert MB to bytes
					Resources.CPUUsagePercent,
					AppState.GetActiveConnectionCount().await as u64,
					ActiveThreads,
				);

				dev_log!("lifecycle", "[ConnectionMonitor] Active threads (tasks): {}", ActiveThreads);
				// Clean up stale connections (5 minute timeout)
				if let Err(Error) = AppState.CleanupStaleConnections(300).await {
					dev_log!(
						"lifecycle",
						"warn: [ConnectionMonitor] Failed to cleanup stale connections: {}",
						Error
					);
				}

				// Perform health checks
				match HealthManager.CheckService("connections").await {
					Ok(_) => {},
					Err(Error) => {
						dev_log!("lifecycle", "warn: [ConnectionMonitor] Health check failed: {}", Error);
						// Record metrics for failed health check
						let MetricsCollector = Metrics::GetMetrics();
						MetricsCollector.RecordRequestFailure("health_check_failed", 0.0);
					},
				}

				dev_log!(
					"lifecycle",
					"[ConnectionMonitor] Active connections: {}",
					AppState.GetActiveConnectionCount().await
				);
			}
		}
	});

	// Register background task with error handling
	if let Err(Error) = AppState.RegisterBackgroundTask(ConnectionMonitorHandle.clone()).await {
		dev_log!("lifecycle", "warn: [Monitor] Failed to register connection monitor: {}", Error); // Non-fatal: continue without task tracking
	}

	// Start health monitoring background task
	let HealthMonitorHandle = tokio::spawn({
		async move {
			let mut Tick = interval(Duration::from_secs(30)); // Check every 30 seconds

			loop {
				Tick.tick().await;

				// Perform comprehensive health checks
				let Services = ["authentication", "updates", "downloader", "indexing", "grpc"];
				for Service in Services.iter() {
					if let Err(Error) = HealthManager.CheckService(Service).await {
						dev_log!(
							"lifecycle",
							"warn: [HealthMonitor] Health check failed for {}: {}",
							Service,
							Error
						);
					}
				}

				// Log overall health status
				let OverallHealth = HealthManager.GetOverallHealth().await;
				dev_log!("lifecycle", "[HealthMonitor] Overall health: {:?}", OverallHealth);
			}
		}
	});

	// Register health monitoring task with error handling
	if let Err(Error) = AppState.RegisterBackgroundTask(HealthMonitorHandle.clone()).await {
		dev_log!("lifecycle", "warn: [Monitor] Failed to register health monitor: {}", Error); // Non-fatal: continue monitoring may not be tracked
	}

	dev_log!("lifecycle", "[Monitor] Background monitoring tasks started");
	MonitoringHandles { ConnectionMonitor:ConnectionMonitorHandle, HealthMonitor:HealthMonitorHandle }
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	#[ignore] // Requires actual app state
	#[tokio::test]
	async fn TestStartMonitoring() {
		// This test requires proper app state and health manager setup
		// and is ignored for automated test runs.
	}
}
