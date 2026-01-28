//! # Air Binary Entry Point
//!
//! Air is the persistent background daemon that handles resource-intensive operations
//! for the Land code editor. It runs as a standalone process alongside Mountain,
//! communicating via gRPC/Vine protocol to offload tasks like updates, downloads,
//! authentication, and file indexing.
//!
//! Key Responsibilities:
//! - gRPC server hosting for Mountain communication
//! - Authentication and cryptographic operations
//! - Update management and application patching
//! - Background downloading of extensions and dependencies
//! - File indexing and processing
//! - Resource monitoring and optimization

#![allow(non_snake_case, non_camel_case_types)]

use std::{net::SocketAddr, sync::Arc, time::Duration};
use log::{debug, error, info, warn};
use tokio::signal;

use crate::{
    ApplicationState::ApplicationState,
    Authentication::AuthenticationService,
    Configuration::ConfigurationManager,
    Downloader::DownloadManager,
    Indexing::FileIndexer,
    Updates::UpdateManager,
    Vine::Server::AirVinegRPCService,
};

mod ApplicationState;
mod Authentication;
mod Configuration;
mod Downloader;
mod Indexing;
mod Updates;
mod Vine;

// =============================================================================
// Debug Helpers
// =============================================================================

/// Logs a checkpoint message at debug level
macro_rules! TraceStep {
    ($($arg:tt)*) => {{
        debug!($($arg)*);
    }};
}

/// Shutdown signal handler for graceful termination
async fn wait_for_shutdown_signal() {
    info!("[Shutdown] Waiting for termination signal...");
    
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        info!("[Shutdown] Received Ctrl+C signal");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
        info!("[Shutdown] Received SIGTERM signal");
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    
    info!("[Shutdown] Signal received, initiating graceful shutdown");
}

/// Initialize logging based on environment
fn initialize_logging() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            use std::io::Write;
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            writeln!(
                buf,
                "[{}] [{}] {}",
                timestamp,
                record.level(),
                record.args()
            )
        })
        .init();
    
    info!("[Boot] Logging initialized");
}

/// Parse command line arguments
fn parse_arguments() -> (Option<String>, Option<String>) {
    let args: Vec<String> = std::env::args().collect();
    
    let mut config_path = None;
    let mut bind_address = None;
    
    for i in 0..args.len() {
        match args[i].as_str() {
            "--config" | "-c" => {
                if i + 1 < args.len() {
                    config_path = Some(args[i + 1].clone());
                }
            },
            "--bind" | "-b" => {
                if i + 1 < args.len() {
                    bind_address = Some(args[i + 1].clone());
                }
            },
            _ => {}
        }
    }
    
    debug!("[Boot] CLI Args - config: {:?}, bind: {:?}", config_path, bind_address);
    
    (config_path, bind_address)
}

// =============================================================================
// Main Application Entry Point
// =============================================================================

/// The main asynchronous function that sets up and runs the Air daemon
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // -------------------------------------------------------------------------
    // [Boot] [Logging] Initialize logging system
    // -------------------------------------------------------------------------
    initialize_logging();
    
    info!("[Boot] Starting Air Daemon 🪁");
    info!("[Boot] Version: {} ({})", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_NAME"));
    
    // -------------------------------------------------------------------------
    // [Boot] [Args] Parse command line arguments
    // -------------------------------------------------------------------------
    TraceStep!("[Boot] [Args] Parsing command line arguments...");
    
    let (config_path, bind_address) = parse_arguments();
    
    // -------------------------------------------------------------------------
    // [Boot] [Configuration] Load configuration
    // -------------------------------------------------------------------------
    TraceStep!("[Boot] [Configuration] Loading configuration...");
    
    let config_manager = ConfigurationManager::new(config_path)?;
    let configuration = config_manager.load_configuration().await?;
    
    debug!("[Boot] [Configuration] Configuration loaded successfully");
    
    // -------------------------------------------------------------------------
    // [Boot] [State] Initialize application state
    // -------------------------------------------------------------------------
    TraceStep!("[Boot] [State] Initializing application state...");
    
    let app_state = Arc::new(ApplicationState::new(configuration.clone()).await?);
    
    info!("[Boot] [State] Application state initialized");
    
    // -------------------------------------------------------------------------
    // [Boot] [Services] Initialize core services
    // -------------------------------------------------------------------------
    TraceStep!("[Boot] [Services] Initializing core services...");
    
    // Authentication Service
    let auth_service = Arc::new(AuthenticationService::new(app_state.clone()).await?);
    
    // Update Manager
    let update_manager = Arc::new(UpdateManager::new(app_state.clone()).await?);
    
    // Download Manager
    let download_manager = Arc::new(DownloadManager::new(app_state.clone()).await?);
    
    // File Indexer
    let file_indexer = Arc::new(FileIndexer::new(app_state.clone()).await?);
    
    info!("[Boot] [Services] Core services initialized");
    
    // -------------------------------------------------------------------------
    // [Boot] [Vine] Initialize gRPC server
    // -------------------------------------------------------------------------
    TraceStep!("[Boot] [Vine] Initializing gRPC server...");
    
    let bind_addr: SocketAddr = bind_address
        .unwrap_or_else(|| "[::1]:50052".to_string())
        .parse()?;
    
    let vine_service = AirVinegRPCService::new(
        app_state.clone(),
        auth_service.clone(),
        update_manager.clone(),
        download_manager.clone(),
        file_indexer.clone(),
    );
    
    let server = tonic::transport::Server::builder()
        .add_service(AirServiceServer::new(vine_service))
    
    info!("[Boot] [Vine] gRPC server configured on {}", bind_addr);
    
    // -------------------------------------------------------------------------
    // [Boot] [Startup] Start services
    // -------------------------------------------------------------------------
    TraceStep!("[Boot] [Startup] Starting services...");
    
    // Start background services
    let auth_handle = auth_service.start_background_tasks().await?;
    let update_handle = update_manager.start_background_tasks().await?;
    let download_handle = download_manager.start_background_tasks().await?;
    let indexing_handle = file_indexer.start_background_tasks().await?;
    
    info!("[Boot] [Startup] All services started successfully");
    
    // -------------------------------------------------------------------------
    // [Runtime] Run server and wait for shutdown
    // -------------------------------------------------------------------------
    info!("[Runtime] Air Daemon 🪁 is now running");
    info!("[Runtime] Listening on {} for Mountain connections", bind_addr);
    
    // Combine server and signal handling
    tokio::select! {
        server_result = server => {
            if let Err(e) = server_result {
                error!("[Runtime] gRPC server error: {}", e);
            }
        },
        _ = wait_for_shutdown_signal() => {
            info!("[Runtime] Shutdown signal received");
        }
    }
    
    // -------------------------------------------------------------------------
    // [Shutdown] Graceful shutdown
    // -------------------------------------------------------------------------
    info!("[Shutdown] Initiating graceful shutdown...");
    
    // Stop background services
    auth_service.stop_background_tasks().await;
    update_manager.stop_background_tasks().await;
    download_manager.stop_background_tasks().await;
    file_indexer.stop_background_tasks().await;
    
    // Wait for services to stop
    let _ = tokio::join!(
        auth_handle,
        update_handle,
        download_handle,
        indexing_handle
    );
    
    info!("[Shutdown] All services stopped");
    info!("[Shutdown] Air Daemon 🪁 has shut down gracefully");
    
    Ok(())
}
