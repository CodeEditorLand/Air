//! # Binary
//!
//! ## File: Binary/Binary.rs
//!
//! ## Role in Air Architecture
//!
//! Main entry point for the Air daemon, orchestrating all initialization,
//! service startup, and graceful shutdown. This file is the primary bootstrap
//! for the Air background service.
//!
//! ## Primary Responsibility
//!
//! Orchestrate daemon initialization and manage main application lifecycle.
//!
//! ## Secondary Responsibilities
//!
//! - Coordinate service initialization order
//! - Initialize gRPC server (Vine protocol)
//! - Manage graceful shutdown sequence
//! - Provide CLI command handling
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `tokio` - Async runtime
//! - `log` - Logging facade
//! - `serde_json` - JSON output for CLI commands
//!
//! **Internal Modules:**
//! - `Initialize::Configure::Log::ConfigureLog` - Logging setup
//! - `Initialize::Configure::Port::SelectPort` - Port selection
//! - `Initialize::Build::BuildServer` - gRPC server building
//! - `Initialize::Service::*` - Service initialization
//! - `Initialize::Command::*` - CLI command handling
//! - `Binary::Shutdown::WaitForShutdownSignal` - Shutdown handling
//! - `Binary::Monitor::StartMonitoring` - Background monitoring
//! - `AirLibrary::*` - Core library modules
//!
//! ## Dependents
//!
//! - None (this is the entry point)
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's main entry point in
//! `src/vs/code/node/cli.ts`
//!
//! ## Security Considerations
//!
//! - Validates environment before starting services
//! - Daemon lock prevents multiple instances
//! - Configuration validation before applying
//!
//! ## Performance Considerations
//!
//! - Lazy initialization where possible
//! - Parallel service startup
//! - Minimal overhead for CLI commands
//!
//! ## Error Handling Strategy
//!
//! - Early validation to fail fast on errors
//! - Descriptive error messages for failures
//! - Graceful shutdown cleanup on errors
//!
//! ## Thread Safety
//!
//! - Async runtime manages threads
//! - Arc ensures thread-safe sharing
//!
//! ## Startup Sequence
//!
//! 1. Initialize logging
//! 2. Parse command-line arguments
//! 3. Validate environment
//! 4. Initialize observability (metrics, tracing)
//! 5. Load configuration
//! 6. Acquire daemon lock
//! 7. Initialize services
//! 8. Start gRPC server
//! 9. Start monitoring tasks
//! 10. Wait for shutdown signal
//! 11. Graceful shutdown

#![allow(non_snake_case)]

use std::sync::Arc;
use std::time::Duration;
use log::{error, info, warn};

use AirLibrary::{
    ApplicationState,
    Authentication::AuthenticationService,
    CLI::Command,
    Configuration::{AirConfiguration, ConfigurationManager, DefaultConfigFile},
    Daemon::DaemonManager,
    Downloader::DownloadManager,
    HealthCheck::{HealthCheckLevel, HealthCheckManager},
    Indexing::FileIndexer,
    Logging, Metrics, Tracing,
    ProtocolVersion, VERSION,
};

// Initialize module exports
mod Shutdown;
pub use Shutdown::WaitForShutdownSignal;

mod Monitor;
pub use Monitor::StartMonitoring;

// Initialize command parsing
// Note: The Initialize::* modules are in a separate crate location
// and would need to be imported via the AirLibrary

/// The main asynchronous function that sets up and runs the Air daemon
///
/// This is the primary entry point for the Air background service. It coordinates
/// all initialization, starts the gRPC server, manages the daemon lifecycle, and
/// handles graceful shutdown.
///
/// # Startup Sequence
///
/// 1. Initialize logging and observability (metrics, tracing)
/// 2. Parse command-line arguments (for CLI commands or daemon config)
/// 3. Load configuration (with validation)
/// 4. Acquire daemon lock (ensure single instance)
/// 5. Initialize application state
/// 6. Create and register core services
/// 7. Start gRPC server (Vine protocol on port 50053)
/// 8. Start background tasks and monitoring
/// 9. Wait for shutdown signal
/// 10. Graceful shutdown sequence
///
/// # CLI Mode
///
/// If a CLI command is provided (status, version, config, etc.), the command
/// is executed and the process exits without starting the daemon.
///
/// # Daemon Mode
///
/// Starts the background service with:
/// - gRPC server on port 50053 (Vine protocol)
/// - All background services (auth, updates, downloads, indexing)
/// - Health monitoring
/// - Resource monitoring
///
/// # TODO
/// - Implement configuration hot-reload signal handling (SIGHUP)
/// - Add startup timeout and failure recovery
//! - Implement daemon mode forking (Unix)
//! - Add Windows service integration
//! - Implement crash recovery and restart

#[tokio::main]
async fn Main() -> Result<(), Box<dyn std::error::Error>> {
    // -------------------------------------------------------------------------
    // [Boot] [Logging] Initialize logging system
    // -------------------------------------------------------------------------
    AirLibrary::Logging::initialize_logging();
    
    info!("[Boot] ===========================================");
    info!("[Boot] Starting Air Daemon 🪁");
    info!("[Boot] ===========================================");
    info!("[Boot] Version: {} ({})", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_NAME"));
    info!("[Boot] Build: {}", env!("BUILD_TIMESTAMP").unwrap_or("unknown".to_string()));
    info!("[Boot] Target: {}-{}", std::env::consts::OS, std::env::consts::ARCH);
    
    // -------------------------------------------------------------------------
    // [Boot] [Environment] Validate environment before starting
    // -------------------------------------------------------------------------
    info!("[Boot] Validating environment...");
    
    if let Err(e) = validate_environment().await {
        error!("[Boot] Environment validation failed: {}", e);
        return Err(format!("Environment validation failed: {}", e).into());
    }
    
    info!("[Boot] Environment validation passed");
    
    // -------------------------------------------------------------------------
    // [Boot] [Observability] Initialize metrics and tracing
    // -------------------------------------------------------------------------
    info!("[Boot] [Observability] Initializing observability systems...");
    
    // Initialize metrics with error handling
    if let Err(e) = Metrics::initialize_metrics() {
        error!("[Boot] Failed to initialize metrics: {}", e);
        // Non-fatal: continue without metrics
    } else {
        info!("[Boot] [Observability] Metrics system initialized");
    }
    
    // Initialize tracing with error handling
    if let Err(e) = Tracing::initialize_tracing() {
        error!("[Boot] Failed to initialize tracing: {}", e);
        // Non-fatal: continue without tracing
    } else {
        info!("[Boot] [Observability] Tracing system initialized");
    }
    
    info!("[Boot] [Observability] Observability systems initialized");
    
    // -------------------------------------------------------------------------
    // [Boot] [Args] Parse command line arguments
    // -------------------------------------------------------------------------
    info!("[Boot] [Args] Parsing command line arguments...");
    
    // Parse arguments and check for CLI command
    let args: Vec<String> = std::env::args().collect();
    
    // Check if we're running with CLI command
    if args.len() > 1 {
        match args[1].as_str() {
            "status" | "restart" | "config" | "metrics" | "logs" | "debug" |
            "help" | "version" | "-h" | "--help" | "-v" | "--version" => {
                // CLI mode - Handle command and exit
                info!("[Boot] CLI command detected, executing...");
                
                if let Ok(cmd) = AirLibrary::CLI::CliParser::parse(args) {
                    let result = handle_cli_command(cmd).await;
                    match &result {
                        Ok(_) => {
                            info!("[Boot] CLI command completed successfully");
                            std::process::exit(0);
                        }
                        Err(e) => {
                            error!("[Boot] CLI command failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    error!("[Boot] Failed to parse CLI command");
                    std::process::exit(1);
                }
            }
            _ => {}
        }
    }
    
    // Parse daemon arguments
    let mut config_path: Option<String> = None;
    let mut bind_address: Option<String> = None;
    
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--config" | "-c" => {
                if i + 1 < args.len() {
                    config_path = Some(args[i + 1].clone());
                    i += 1;
                }
            },
            "--bind" | "-b" => {
                if i + 1 < args.len() {
                    bind_address = Some(args[i + 1].clone());
                    i += 1;
                }
            },
            _ => {}
        }
        i += 1;
    }
    
    // -------------------------------------------------------------------------
    // [Boot] [Configuration] Load configuration
    // -------------------------------------------------------------------------
    info!("[Boot] [Configuration] Loading configuration...");
    
    let config_manager = match ConfigurationManager::new(config_path) {
        Ok(cm) => cm,
        Err(e) => {
            error!("[Boot] Failed to create configuration manager: {}", e);
            return Err(format!("Configuration manager initialization failed: {}", e).into());
        }
    };
    
    // Load configuration with timeout
    let configuration: Arc<AirConfiguration> = match tokio::time::timeout(
        Duration::from_secs(10),
        config_manager.load_configuration()
    ).await {
        Ok(Ok(config)) => {
            info!("[Boot] [Configuration] Configuration loaded successfully");
            Arc::new(config)
        }
        Ok(Err(e)) => {
            error!("[Boot] Failed to load configuration: {}", e);
            return Err(format!("Configuration load failed: {}", e).into());
        }
        Err(_) => {
            error!("[Boot] Configuration load timed out");
            return Err("Configuration load timed out".into());
        }
    };
    
    // -------------------------------------------------------------------------
    // [Boot] [Daemon] Initialize daemon lifecycle management
    // -------------------------------------------------------------------------
    info!("[Boot] [Daemon] Initializing daemon lifecycle management...");
    
    let daemon_manager = match DaemonManager::new(None) {
        Ok(dm) => dm,
        Err(e) => {
            error!("[Boot] Failed to create daemon manager: {}", e);
            return Err(format!("Daemon manager initialization failed: {}", e).into());
        }
    };
    
    // Acquire daemon lock to ensure single instance with timeout
    match tokio::time::timeout(
        Duration::from_secs(5),
        daemon_manager.acquire_lock()
    ).await {
        Ok(Ok(_)) => {
            info!("[Boot] [Daemon] Daemon lock acquired successfully");
        }
        Ok(Err(e)) => {
            error!("[Boot] Failed to acquire daemon lock: {}", e);
            error!("[Boot] Another instance may already be running");
            return Err(format!("Daemon lock acquisition failed: {}", e).into());
        }
        Err(_) => {
            error!("[Boot] Daemon lock acquisition timed out");
            return Err("Daemon lock acquisition timed out".into());
        }
    }
    
    // -------------------------------------------------------------------------
    // [Boot] [Health] Initialize health check system
    // -------------------------------------------------------------------------
    info!("[Boot] [Health] Initializing health check system...");
    
    let health_manager: Arc<HealthCheckManager> = Arc::new(HealthCheckManager::new(None));
    
    info!("[Boot] [Health] Health check system initialized");
    
    // -------------------------------------------------------------------------
    // [Boot] [State] Initialize application state
    // -------------------------------------------------------------------------
    info!("[Boot] [State] Initializing application state...");
    
    let app_state: Arc<ApplicationState> = match tokio::time::timeout(
        Duration::from_secs(10),
        ApplicationState::new(configuration.clone())
    ).await {
        Ok(Ok(state)) => {
            info!("[Boot] [State] Application state initialized");
            Arc::new(state)
        }
        Ok(Err(e)) => {
            error!("[Boot] Failed to initialize application state: {}", e);
            let _ = daemon_manager.release_lock().await;
            return Err(format!("Application state initialization failed: {}", e).into());
        }
        Err(_) => {
            error!("[Boot] Application state initialization timed out");
            let _ = daemon_manager.release_lock().await;
            return Err("Application state initialization timed out".into());
        }
    };
    
    // -------------------------------------------------------------------------
    // [Boot] [Services] Initialize core services
    // -------------------------------------------------------------------------
    info!("[Boot] [Services] Initializing core services...");
    
    // Initialize each service with error handling
    let auth_service: Arc<AuthenticationService> = match tokio::time::timeout(
        Duration::from_secs(10),
        AuthenticationService::new(app_state.clone())
    ).await {
        Ok(Ok(svc)) => Arc::new(svc),
        Ok(Err(e)) => {
            error!("[Boot] Failed to initialize authentication service: {}", e);
            return Err(format!("Authentication service initialization failed: {}", e).into());
        }
        Err(_) => {
            error!("[Boot] Authentication service initialization timed out");
            return Err("Authentication service initialization timed out".into());
        }
    };
    
    let update_manager: Arc<UpdateManager> = match tokio::time::timeout(
        Duration::from_secs(10),
        UpdateManager::new(app_state.clone())
    ).await {
        Ok(Ok(svc)) => Arc::new(svc),
        Ok(Err(e)) => {
            error!("[Boot] Failed to initialize update manager: {}", e);
            return Err(format!("Update manager initialization failed: {}", e).into());
        }
        Err(_) => {
            error!("[Boot] Update manager initialization timed out");
            return Err("Update manager initialization timed out".into());
        }
    };
    
    let download_manager: Arc<DownloadManager> = match tokio::time::timeout(
        Duration::from_secs(10),
        DownloadManager::new(app_state.clone())
    ).await {
        Ok(Ok(svc)) => Arc::new(svc),
        Ok(Err(e)) => {
            error!("[Boot] Failed to initialize download manager: {}", e);
            return Err(format!("Download manager initialization failed: {}", e).into());
        }
        Err(_) => {
            error!("[Boot] Download manager initialization timed out");
            return Err("Download manager initialization timed out".into());
        }
    };
    
    let file_indexer: Arc<FileIndexer> = match tokio::time::timeout(
        Duration::from_secs(10),
        FileIndexer::new(app_state.clone())
    ).await {
        Ok(Ok(svc)) => Arc::new(svc),
        Ok(Err(e)) => {
            error!("[Boot] Failed to initialize file indexer: {}", e);
            return Err(format!("File indexer initialization failed: {}", e).into());
        }
        Err(_) => {
            error!("[Boot] File indexer initialization timed out");
            return Err("File indexer initialization timed out".into());
        }
    };
    
    info!("[Boot] [Services] All core services initialized successfully");
    
    // -------------------------------------------------------------------------
    // [Boot] [Health] Register services for health monitoring
    // -------------------------------------------------------------------------
    info!("[Boot] [Health] Registering services for health monitoring...");
    
    // Register each service with validation
    let service_registrations = vec![
        ("authentication", HealthCheckLevel::Functional),
        ("updates", HealthCheckLevel::Functional),
        ("downloader", HealthCheckLevel::Functional),
        ("indexing", HealthCheckLevel::Functional),
        ("grpc", HealthCheckLevel::Responsive),
        ("connections", HealthCheckLevel::Alive),
    ];
    
    for (service_name, level) in service_registrations {
        match tokio::time::timeout(
            Duration::from_secs(5),
            health_manager.register_service(service_name.to_string(), level)
        ).await {
            Ok(Ok(_)) => {
                log::debug!("[Boot] [Health] Registered service: {}", service_name);
            }
            Ok(Err(e)) => {
                warn!("[Boot] Failed to register service {}: {}", service_name, e);
            }
            Err(_) => {
                warn!("[Boot] Service registration timed out: {}", service_name);
            }
        }
    }
    
    info!("[Boot] [Health] Service health monitoring configured");
    
    // -------------------------------------------------------------------------
    // [Boot] [Vine] Initialize gRPC server
    // -------------------------------------------------------------------------
    info!("[Boot] [Vine] Initializing gRPC server...");

    // Parse bind address
    let default_bind = std::env::var("AIR_BIND_ADDR")
        .unwrap_or_else(|_| "[::1]:50053".to_string());
    
    let bind_addr_str = bind_address.unwrap_or(default_bind);
    
    let bind_addr: std::net::SocketAddr = match bind_addr_str.parse() {
        Ok(addr) => addr,
        Err(e) => {
            error!("[Boot] Invalid bind address '{}': {}", bind_addr_str, e);
            return Err(format!("Invalid bind address: {}", e).into());
        }
    };

    info!("[Boot] [Vine] Configuring gRPC server on {}", bind_addr);

    // Create gRPC service implementation
    let vine_service = match AirLibrary::Vine::Server::AirVinegRPCService::AirVinegRPCService::new(
        app_state.clone(),
        auth_service.clone(),
        update_manager.clone(),
        download_manager.clone(),
        file_indexer.clone(),
    ) {
        Ok(svc) => svc,
        Err(e) => {
            error!("[Boot] Failed to create Vine gRPC service: {}", e);
            return Err(format!("Vine service creation failed: {}", e).into());
        }
    };

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Spawn the gRPC server
    let server_handle: tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> = 
        tokio::spawn(async move {
            info!("[Vine] Starting gRPC server on {}", bind_addr);

            let svc = AirLibrary::Vine::Generated::air_service_server::AirServiceServer::new(vine_service);

            let server = tonic::transport::Server::builder()
                .add_service(svc)
                .serve_with_shutdown(bind_addr, async {
                    let _ = shutdown_rx.await;
                    info!("[Vine] Shutdown signal received, stopping server...");
                });

            info!("[Vine] gRPC server listening on {}", bind_addr);

            match server.await {
                Ok(_) => {
                    info!("[Vine] gRPC server stopped cleanly");
                    Ok(())
                }
                Err(e) => {
                    error!("[Vine] gRPC server error: {}", e);
                    Err(e.into())
                }
            }
        });
    
    // Wait briefly for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    if server_handle.is_finished() {
        error!("[Boot] gRPC server failed to start");
        let _ = daemon_manager.release_lock().await;
        return Err("gRPC server failed to start".into());
    }
    
    // -------------------------------------------------------------------------
    // [Boot] [Monitoring] Start background monitoring tasks
    // -------------------------------------------------------------------------
    info!("[Boot] [Monitor] Starting background monitoring tasks...");
    
    let monitoring_handles = StartMonitoring(app_state.clone(), health_manager.clone()).await;
    
    // -------------------------------------------------------------------------
    // [Boot] [Services] Start background tasks for services
    // -------------------------------------------------------------------------
    info!("[Boot] [Startup] Starting background service tasks...");
    
    // Start each service
    let auth_handle = auth_service.start_background_tasks().await?;
    let update_handle = update_manager.start_background_tasks().await?;
    let download_handle = download_manager.start_background_tasks().await?;
    let indexing_handle = file_indexer.start_background_tasks().await?;
    
    info!("[Boot] [Startup] All services started successfully");
    
    // -------------------------------------------------------------------------
    // [Runtime] Run server and wait for shutdown
    // -------------------------------------------------------------------------
    info!("===========================================");
    info!("[Runtime] Air Daemon 🪁 is now running");
    info!("[Runtime] Listening on {} for Mountain connections", bind_addr);
    info!("[Runtime] Protocol Version: {}", ProtocolVersion);
    info!("[Runtime] Cocoon Port: 50052");
    info!("===========================================");
    info!("Running. Press Ctrl+C to stop.");
    info!("");
    
    // Wait for shutdown signal
    WaitForShutdownSignal().await;

    // Signal gRPC server to shut down
    info!("[Shutdown] Signaling gRPC server to stop...");
    let _ = shutdown_tx.send(());

    // -------------------------------------------------------------------------
    // [Shutdown] Graceful shutdown
    // -------------------------------------------------------------------------
    info!("===========================================");
    info!("[Shutdown] Initiating graceful shutdown...");
    info!("===========================================");
    
    // Stop gRPC server with timeout
    match tokio::time::timeout(Duration::from_secs(30), server_handle).await {
        Ok(Ok(Ok(_))) => {
            info!("[Shutdown] gRPC server stopped normally");
        }
        Ok(Ok(Err(e))) => {
            warn!("[Shutdown] gRPC server stopped with error: {}", e);
        }
        Ok(Err(e)) => {
            warn!("[Shutdown] gRPC server task panicked: {:?}", e);
        }
        Err(_) => {
            warn!("[Shutdown] gRPC server shutdown timed out");
        }
    }
    
    // Stop background services
    info!("[Shutdown] Stopping background services...");
    auth_service.stop_background_tasks().await;
    update_manager.stop_background_tasks().await;
    download_manager.stop_background_tasks().await;
    file_indexer.stop_background_tasks().await;
    
    // Wait for services to stop
    info!("[Shutdown] Waiting for services to complete...");
    let _ = tokio::time::timeout(
        Duration::from_secs(10),
        tokio::join!(auth_handle, update_handle, download_handle, indexing_handle)
    ).await;
    
    // Stop monitoring tasks
    monitoring_handles.connection_monitor.abort();
    monitoring_handles.health_monitor.abort();
    
    // Stop all background tasks
    info!("[Shutdown] Stopping background tasks...");
    if let Err(e) = tokio::time::timeout(
        Duration::from_secs(10),
        app_state.stop_all_background_tasks()
    ).await {
        match e {
            Ok(inner) => warn!("[Shutdown] Failed to stop background tasks: {}", inner),
            Err(_) => warn!("[Shutdown] Background tasks stop timed out"),
        }
    }
    
    // Log final statistics
    info!("[Shutdown] Collecting final statistics...");
    let metrics = app_state.get_metrics().await;
    let resources = app_state.get_resource_usage().await;
    let health_stats = health_manager.get_health_statistics().await;
    
    let metrics_data = Metrics::get_metrics().get_metrics_data();
    
    info!("===========================================");
    info!("[Shutdown] Final Statistics");
    info!("===========================================");
    info!("[Shutdown] Requests:");
    info!("  - Successful: {}", metrics.successful_requests);
    info!("  - Failed: {}", metrics.failed_requests);
    info!("[Shutdown] Metrics:");
    info!("  - Success rate: {:.2}%", metrics_data.success_rate());
    info!("  - Error rate: {:.2}%", metrics_data.error_rate());
    info!("[Shutdown] Resources:");
    info!("  - Memory: {:.2} MB", resources.memory_usage_mb);
    info!("  - CPU: {:.2}%", resources.cpu_usage_percent);
    info!("[Shutdown] Health:");
    info!("  - Overall: {:.2}%", health_stats.overall_health_percentage());
    info!("  - Healthy services: {}/{}", health_stats.healthy_services, health_stats.total_services);
    info!("===========================================");
    
    // Release daemon lock
    info!("[Shutdown] Releasing daemon lock...");
    if let Err(e) = daemon_manager.release_lock().await {
        warn!("[Shutdown] Failed to release daemon lock: {}", e);
    }
    
    info!("[Shutdown] All services stopped");
    info!("[Shutdown] Air Daemon 🪁 has shut down gracefully");
    info!("===========================================");
    
    Ok(())
}

/// Validate the runtime environment before starting the daemon
///
/// # TODO
/// - Check disk space availability
/// - Validate network connectivity
/// - Check file system permissions
/// - Verify required executables exist
/// - Validate system resources (CPU, RAM)
async fn validate_environment() -> Result<(), String> {
    info!("[Environment] OS: {}, Arch: {}", std::env::consts::OS, std::env::consts::ARCH);
    
    // Verify we can create lock files
    let lock_path = "/tmp/air-test-lock.tmp";
    if std::fs::write(lock_path, b"test").is_err() {
        return Err("Cannot write to /tmp directory".to_string());
    }
    let _ = std::fs::remove_file(lock_path);
    
    Ok(())
}

/// Handle CLI command for non-daemon mode
async fn handle_cli_command(cmd: Command) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Implement actual CLI command handling
    match cmd {
        Command::Version => {
            println!("Air {} ({})", VERSION, env!("CARGO_PKG_NAME"));
            println!("Protocol: Version {} (gRPC)", ProtocolVersion);
            Ok(())
        }
        _ => {
            println!("CLI command handling not yet fully implemented");
            Ok(())
        }
    }
}
