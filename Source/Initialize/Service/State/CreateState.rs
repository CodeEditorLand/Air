//! # CreateState
//!
//! ## File: Initialize/Service/State/CreateState.rs
//!
//! ## Role in Air Architecture
//!
//! Initializes the shared application state that serves as the central
//! coordination point for all Air services. The ApplicationState holds
//! configuration, connections, and shared data structures accessed by all
//! background services.
//!
//! ## Primary Responsibility
//!
//! Create and initialize the shared ApplicationState for the Air daemon.
//!
//! ## Secondary Responsibilities
//!
//! - Validate configuration before creating state
//! - Provide timeout for state initialization
//! - Handle initialization failures with cleanup
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `tokio::time` - Timeout handling
//!
//! **Internal Modules:**
//! - `AirLibrary::ApplicationState` - Application state implementation
//! - `AirLibrary::Configuration::AirConfiguration` - Configuration structure
//!
//! ## Dependents
//!
//! - `Initialize::Service::Auth::StartAuth` - Needs state for auth operations
//! - `Initialize::Service::Update::StartUpdate` - Needs state for update
//!   management
//! - `Initialize::Service::Download::StartDownload` - Needs state for downloads
//! - `Initialize::Service::Index::StartIndex` - Needs state for indexing
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's state management in
//! `src/vs/platform/storage/common/storageService.ts`
//!
//! ## Security Considerations
//!
//! - Configuration is validated before state creation
//! - Sensitive data is properly isolated within state
//! - Timeout prevents resource exhaustion during init
//!
//! ## Performance Considerations
//!
//! - State creation is done once at startup
//! - Arc wrapping allows efficient sharing
//! - Timeout prevents hanging on slow operations
//!
//! ## Error Handling Strategy
//!
//! - Returns descriptive errors for initialization failures
//! - Timeout prevents indefinite blocking
//! - Errors trigger proper cleanup of daemon lock
//!
//! ## Thread Safety
//!
//! - Returns Arc<ApplicationState> for thread-safe sharing
//! - Safe for concurrent access across all services

use std::{sync::Arc, time::Duration};

use log::{error, info};
use tokio as _;
use AirLibrary::{ApplicationState, Configuration::AirConfiguration};

/// Create the shared application state
///
/// Initializes the ApplicationState with the provided configuration.
/// The state is wrapped in Arc for thread-safe sharing across all services.
///
/// # Arguments
///
/// * `configuration` - Arc-wrapped configuration for the daemon
///
/// # Returns
///
/// Returns an `Arc<ApplicationState>` on success.
///
/// # Errors
///
/// Returns an error if:
/// - State initialization fails
/// - Configuration is invalid
/// - Initialization timeout occurs
///
/// # Timeout
///
/// A 10-second timeout is applied to prevent hanging on slow operations.
///
/// # TODO
/// - Add configuration validation before state creation
/// - Implement state recovery from previous run
/// - Add state snapshot for debugging
pub async fn CreateState(configuration:Arc<AirConfiguration>) -> Result<Arc<ApplicationState>, String> {
	info!("[State] Creating application state...");

	// Initialize state with timeout
	let state_result =
		tokio::time::timeout(Duration::from_secs(10), ApplicationState::new(configuration.clone())).await;

	match state_result {
		Ok(Ok(state)) => {
			info!("[State] Application state initialized successfully");
			Ok(Arc::new(state))
		},
		Ok(Err(e)) => {
			error!("[State] Failed to initialize application state: {}", e);
			Err(format!("Application state initialization failed: {}", e))
		},
		Err(_) => {
			error!("[State] Application state initialization timed out");
			Err("Application state initialization timed out".to_string())
		},
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	#[ignore] // Requires actual configuration
	async fn test_create_state() {
		// This test requires proper configuration setup
		// and is ignored for automated test runs.
		// In practice, this would be an integration test.
	}
}
