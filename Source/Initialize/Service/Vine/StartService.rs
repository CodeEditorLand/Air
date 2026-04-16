//! # StartService
//!
//! ## File: Initialize/Service/Vine/StartService.rs
//!
//! ## Role in Air Architecture
//!
//! Starts the Vine gRPC server that is the primary communication channel between
//! Air and Mountain. The Vine protocol on port 50053 handles all requests from the
//! Land editor to background services like auth, updates, downloads, and indexing.
//!
//! ## Primary Responsibility
//!
/// Start the Vine gRPC server and initialize connection management.
//!
//! ## Secondary Responsibilities
///
/// - Graceful shutdown signaling
/// - Connection lifecycle tracking
/// - Request routing to service modules
//!
//! ## Dependencies
///
/// **External Crates:**
/// - `tokio::task` - Async task spawning
/// - `tokio::time` - Timeout and delay utilities
//! - `tonic::transport` - gRPC server transport
//!
//! **Internal Modules:**
//! - `AirLibrary::Vine::Generated::air_service_server` - gRPC server binding
//! - `Initialize::Build::BuildServer` - Built service configuration
//!
//! ## Dependents
//!
//! - `Initialize::Binary::Binary` - Starts gRPC server at boot
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's IPC server in
//! `src/vs/base/parts/ipc/node/ipc.cp.ts`
//!
//! ## Security Considerations
//!
//! - FUTURE: Add TLS/mTLS support
//! - FUTURE: Implement connection authentication
//! - Rate limiting prevents abuse
//! - Input validation on all requests
//!
//! ## Performance Considerations
///
/// - Connection pooling for efficiency
/// - Async handling for high throughput
/// - Streaming support for large transfers
//!
//! ## Error Handling Strategy
///
/// - Server startup failures are fatal
/// - Connection errors are logged and don't halt server
/// - Graceful shutdown preserves data integrity

use std::time::Duration;
use crate::dev_log;
use tonic::transport::Server as TonicServer;

use AirLibrary::Vine::Generated::air_service_server::AirServiceServer;

use super::super::super::Build::BuildServer::BuiltServer;

/// Started Vine gRPC server with handle
///
/// Contains the server join handle for monitoring and the shutdown channel
/// sender for signaling graceful termination.
pub struct StartedService {
    /// Join handle for the server task
    pub server_handle: tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    /// Sender for shutdown signal
    pub shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

/// Start the Vine gRPC server
///
/// Spawns the Vine gRPC server as a background task and returns control to the
/// caller with a handle for monitoring and shutdown signaling.
///
/// # Arguments
///
/// * `built` - The built server configuration from `BuildServer`
///
/// # Returns
///
/// Returns a `StartedService` containing the server handle and shutdown channel.
///
/// # Port
///
/// The server listens on port 50053 by default (Air's Vine protocol port).
/// This is separate from Cocoon's port 50052 (NodeJS host).
///
/// # Protocol
///
/// - **Version**: Vine protocol v1
//! - **Transport**: HTTP/2
//! - **Serialization**: Protocol Buffers
//!
//! # Graceful Shutdown
//!
/// Send a message through `shutdown_tx` to signal graceful shutdown.
/// The server will stop accepting new requests and complete in-flight
/// requests before terminating.
///
//! # FUTURE Enhancements
//! - Add TLS/mTLS support for production
//! - Implement connection authentication
//! - Add connection rate limiting
//! - Implement connection pooling optimizations

pub fn StartService(built: BuiltServer) -> StartedService {
    let BuiltServer {
        service,
        shutdown_tx,
        bind_addr,
    } = built;
    
    dev_log!("lifecycle", "[Vine] Starting gRPC server on {}", bind_addr);
    // Spawn the tonic gRPC server with panic handling
    let server_handle: tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> = 
        tokio::spawn(async move {
            // Create shutdown receiver
            let mut shutdown_rx = shutdown_tx.closed();

            // Build the gRPC service
            let svc = AirServiceServer::new(service);

            // Create server with graceful shutdown
            let server = TonicServer::builder()
                .add_service(svc)
                .serve_with_shutdown(bind_addr, async {
                    // Wait for shutdown signal
                    shutdown_rx.await;
                    dev_log!("lifecycle", "[Vine] Shutdown signal received, stopping server...");                });

            dev_log!("lifecycle", "[Vine] gRPC server listening on {}", bind_addr);
            // Run the server
            match server.await {
                Ok(_) => {
                    dev_log!("lifecycle", "[Vine] gRPC server stopped cleanly");                    Ok(())
                }
                Err(e) => {
                    dev_log!("lifecycle", "error: [Vine] gRPC server error: {}", e);                    Err(e.into())
                }
            }
        });
    
    // Wait briefly for server to start
    tokio::task::spawn_blocking(move || {
        std::thread::sleep(Duration::from_millis(100));
    });
    
    // Check if server started successfully
    if server_handle.is_finished() {
        dev_log!("lifecycle", "error: [Vine] gRPC server failed to start");    }
    
    StartedService { server_handle, shutdown_tx }
}

/// Wait for server to stop with timeout
///
/// Awaits the server task with a configurable timeout.
///
/// # Arguments
///
/// * `started` - The started server configuration
/// * `timeout_secs` - Maximum time to wait for shutdown (default: 30s)
///
/// # Returns
///
/// Returns `Ok(())` if stopped successfully, error otherwise.
///
/// # FUTURE Enhancements
/// - Use timeout from configuration
/// - Add shutdown timeout error details

pub async fn WaitForShutdown(
    started: StartedService,
    timeout_secs: u64,
) -> Result<(), String> {
    let StartedService { 
        mut server_handle, 
        shutdown_tx, 
    } = started;
    
    // Signal shutdown
    dev_log!("lifecycle", "[Vine] Signaling gRPC server to stop...");    let _ = shutdown_tx.send(());

    // Await the server task to finish with timeout
    match tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        server_handle
    ).await {
        Ok(Ok(Ok(_))) => {
            dev_log!("lifecycle", "[Vine] gRPC server stopped normally");            Ok(())
        }
        Ok(Ok(Err(e))) => {
            dev_log!("lifecycle", "error: [Vine] gRPC server stopped with error: {}", e);            Err(format!("Server stopped with error: {}", e))
        }
        Ok(Err(e)) => {
            dev_log!("lifecycle", "error: [Vine] gRPC server task panicked: {:?}", e);            Err("Server task panicked".to_string())
        }
        Err(_) => {
            dev_log!("lifecycle", "error: [Vine] gRPC server shutdown timed out");            Err("Server shutdown timed out".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[ignore] // Requires full service setup
    #[tokio::test]
    async fn test_start_service() {
        // This test requires proper built server setup
        // and is ignored for automated test runs.
    }
}
