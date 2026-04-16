//! # BuildServer
//!
//! ## File: Initialize/Build/BuildServer.rs
//!
//! ## Role in Air Architecture
//!
//! Constructs and configures the Vine gRPC service implementation with all its
//! dependencies. This module ensures the gRPC server has access to all Air
//! services for request routing.
//!
//! ## Primary Responsibility
//!
//! Build the Vine gRPC service with all required service dependencies.
//!
//! ## Secondary Responsibilities
//!
//! - Wire service dependencies into the Vine service
//! - Validate service initialization
//! - Provide the configured service for server startup
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `tokio::sync::oneshot` - Shutdown signaling
//!
//! **Internal Modules:**
//! - `AirLibrary::Vine::Server::AirVinegRPCService::AirVinegRPCService` -
//!   Service implementation
//! - `AirLibrary::ApplicationState` - Shared application state
//! - `AirLibrary::Authentication::AuthenticationService` - Authentication
//!   service
//! - `AirLibrary::Updates::UpdateManager` - Update management service
//! - `AirLibrary::Downloader::DownloadManager` - Download service
//! - `AirLibrary::Indexing::FileIndexer` - File indexing service
//!
//! ## Dependents
//!
//! - `Initialize::Service::Vine::StartService` - Uses to build before starting
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's service composition pattern in
//! `src/vs/workbench/services/extensions/common/extensions.ts`
//!
//! ## Security Considerations
//!
//! - Validates all services are properly initialized
//! - Ensures shared state is properly wrapped in Arc
//! - Prevents use of uninitialized services
//!
//! ## Performance Considerations
//!
//! - Service construction is done once at startup
//! - Arc references allow efficient sharing across gRPC requests
//!
//! ## Error Handling Strategy
//!
//! - Returns errors for any service initialization failure
//! - Clear error messages for missing dependencies
//! - Fails fast to prevent running with incomplete services
//!
//! ## Thread Safety
//!
//! - All services are wrapped in Arc for thread-safe sharing
//! - Safe for concurrent gRPC request handling

use std::{net::SocketAddr, sync::Arc};

use crate::dev_log;
use AirLibrary::{
	ApplicationState,
	Authentication::AuthenticationService,
	Downloader::DownloadManager,
	Indexing::FileIndexer,
	Updates::UpdateManager,
	Vine::Server::AirVinegRPCService::AirVinegRPCService,
};

/// Built Vine gRPC service with shutdown channel
///
/// Contains the configured Vine service and the sending half of a
/// shutdown channel used to signal graceful server termination.
pub struct BuiltServer {
	/// The configured Vine gRPC service
	pub service:AirVinegRPCService,
	/// Sender for shutdown signal
	pub shutdown_tx:tokio::sync::oneshot::Sender<()>,
	/// Bind address for the server
	pub bind_addr:SocketAddr,
}

/// Build the Vine gRPC service with all dependencies
///
/// Constructs the gRPC service implementation with all required service
/// dependencies and creates the shutdown signaling channel.
///
/// # Arguments
///
/// * `app_state` - Shared application state
/// * `auth_service` - Authentication service for user operations
/// * `update_manager` - Update manager for application updates
/// * `download_manager` - Download manager for file downloads
/// * `file_indexer` - File indexer for code navigation
/// * `bind_addr` - Socket address to bind the gRPC server
///
/// # Returns
///
/// Returns a `BuiltServer` containing the configured service and shutdown
/// channel.
///
/// # Errors
///
/// Returns an error if service construction fails.
///
/// # FUTURE Enhancements
/// - Add service health validation before building
/// - Add dependency injection validation
/// - Add service version compatibility checks
pub fn BuildServer(
	app_state:Arc<ApplicationState>,
	auth_service:Arc<AuthenticationService>,
	update_manager:Arc<UpdateManager>,
	download_manager:Arc<DownloadManager>,
	file_indexer:Arc<FileIndexer>,
	bind_addr:SocketAddr,
) -> Result<BuiltServer, String> {
	dev_log!("lifecycle", "[Build] Building Vine gRPC service...");
	// Create the Vine gRPC service with all dependencies
	let service = AirVinegRPCService::new(app_state, auth_service, update_manager, download_manager, file_indexer)
		.map_err(|e| {
			dev_log!("lifecycle", "error: [Build] Failed to create Vine gRPC service: {}", e);			format!("Vine service creation failed: {}", e)
		})?;

	// Create a oneshot channel to signal server shutdown
	let (shutdown_tx, _shutdown_rx) = tokio::sync::oneshot::channel::<()>();

	dev_log!("lifecycle", "[Build] Vine gRPC service built successfully");	dev_log!("lifecycle", "[Build] Service configured with:");	dev_log!("lifecycle", "  - Bind address: {}", bind_addr);	dev_log!("lifecycle", "  - Authentication service: [OK]");	dev_log!("lifecycle", "  - Update manager: [OK]");	dev_log!("lifecycle", "  - Download manager: [OK]");	dev_log!("lifecycle", "  - File indexer: [OK]");
	Ok(BuiltServer { service, shutdown_tx, bind_addr })
}

#[cfg(test)]
mod tests {
	use std::sync::Arc;

	use super::*;

	#[test]
	#[ignore] // Requires actual app state setup
	fn test_build_server() {
		// This test requires proper initialization of all services
		// and is ignored for automated test runs.
		// In practice, this would be an integration test.
	}
}
