//! # StartHealthCheck
//!
//! ## File: Initialize/Service/Health/StartHealthCheck.rs
//!
//! ## Role in Air Architecture
//!
//! Initializes the health check system that monitors all Air service health.
//! The health check system provides visibility into service status and enables
//! automatic detection of service failures.
//!
//! ## Primary Responsibility
//!
//! Create and initialize the health check manager for monitoring Air services.
//!
//! ## Secondary Responsibilities
//!
//! - Configure health check intervals
//! - Initialize health level thresholds
//! - Prepare service registration infrastructure
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `std::sync::Arc` - Thread-safe reference counting
//!
//! **Internal Modules:**
//! - `AirLibrary::HealthCheck::HealthCheckManager` - Health check manager
//! - `AirLibrary::HealthCheck::HealthCheckLevel` - Health check levels
//!
//! ## Dependents
//!
//! - `Initialize::Binary::Binary` - Initializes health monitoring at boot
//! - `Initialize::Service::Vine::StartService` - Registers services for
//!   monitoring
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's health monitoring in
//! `src/vs/workbench/services/health/common/healthService.ts`
//!
//! ## Security Considerations
//!
//! - Health check data does not contain sensitive information
//! - No authentication required for internal health checks
//!
//! ## Performance Considerations
//!
//! - Health checks are lightweight and performant
//! - Configurable intervals prevent excessive CPU usage
//! - Non-blocking async checks
//!
//! ## Error Handling Strategy
/// - Health check failures are logged but non-fatal
/// - Service registration failures logged but don't halt boot
///
/// # FUTURE Enhancements
/// - Add custom health check interval configuration
/// - Implement health check aggregation for dashboard
/// - Add external health check endpoint
use std::sync::Arc;

use AirLibrary::HealthCheck::{HealthCheckLevel, HealthCheckManager};

use crate::dev_log;

/// Start the health check system
///
/// Creates the health check manager that monitors the status of all Air
/// services. Services can be registered for monitoring with configurable health
/// check levels.
///
/// # Returns
///
/// Returns an `Arc<HealthCheckManager>` for registering and checking service
/// health.
///
/// # Health Check Levels
///
/// - `Alive`: Basic check that the service is running
/// - `Responsive`: Service is responding within timeout
/// - `Functional`: Service is functioning correctly
///
/// # Examples
///
/// ```rust
/// let health_manager = StartHealthCheck().await?;
///
/// // Register a service
/// health_manager
/// 	.register_service("authentication".to_string(), HealthCheckLevel::Functional)
/// 	.await?;
///
/// // Check health
/// let health = health_manager.check_service("authentication").await?;
/// ```
///
/// # FUTURE Enhancements
/// - Add service health history tracking
/// - Implement health-based auto-recovery
/// - Add health notification hooks
pub async fn StartHealthCheck() -> Arc<HealthCheckManager> {

	dev_log!("lifecycle", "[Health] Starting health check system...");

	// Create health check manager
	let health_manager = Arc::new(HealthCheckManager::new(None));

	dev_log!("lifecycle", "[Health] Health check system initialized");

	health_manager
}

#[cfg(test)]
mod tests {

	use super::*;

	#[test]
	#[ignore] // Async test
	async fn test_start_health_check() {
		let health_manager = StartHealthCheck().await;

		assert_eq!(Arc::strong_count(&health_manager), 1);
	}
}
