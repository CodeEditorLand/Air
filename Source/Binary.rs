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
use tokio::{signal, time::interval};

use Air::{ApplicationState::ApplicationState, Authentication::AuthenticationService, Configuration::ConfigurationManager, Daemon::DaemonManager, Downloader::DownloadManager, HealthCheck::{HealthCheckManager, HealthCheckLevel}, Indexing::FileIndexer, Logging, Metrics, Tracing, Updates::UpdateManager, CLI::{CliParser, Command, ConfigCommand, DebugCommand, OutputFormatter}, VERSION};

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
        match signal::ctrl_c().await {
            Ok(()) => info!("[Shutdown] Received Ctrl+C signal"),
            Err(e) => error!("[Shutdown] Failed to install Ctrl+C handler: {}", e),
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
                info!("[Shutdown] Received SIGTERM signal");
            }
            Err(e) => error!("[Shutdown] Failed to install signal handler: {}", e),
        }
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
    // Initialize structured logging with JSON output support
    let json_output = std::env::var("AIR_LOG_JSON")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false);
    
    let log_file_path = std::env::var("AIR_LOG_FILE").ok();
    
    if let Err(e) = Logging::initialize_logger(json_output, log_file_path) {
        eprintln!("Failed to initialize structured logging: {}", e);
    }
    
    info!("[Boot] Logging initialized - JSON output: {}, Log file: {:?}", json_output, std::env::var("AIR_LOG_FILE").ok());
}

/// Parse command line arguments
fn parse_arguments() -> (Option<String>, Option<String>, Option<Command>) {
    let args: Vec<String> = std::env::args().collect();
    
    // Check if we're running with CLI command (first arg is a known command)
    if args.len() > 1 {
        match args[1].as_str() {
            "status" | "restart" | "config" | "metrics" | "logs" | "debug" | "help" | "version" | "-h" | "--help" | "-v" | "--version" => {
                match CliParser::parse(args.clone()) {
                    Ok(cmd) => return (None, None, Some(cmd)),
                    Err(e) => {
                        eprintln!("Error parsing CLI command: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            _ => {}
        }
    }
    
    // Parse as daemon arguments
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
    
    (config_path, bind_address, None)
}

/// Handle CLI commands
async fn handle_cli_command(cmd: Command) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        Command::Help { command } => {
            println!("{}", OutputFormatter::format_help(command.as_deref(), VERSION));
            Ok(())
        }
        Command::Version => {
            println!("Air {} ({})", VERSION, env!("CARGO_PKG_NAME"));
            Ok(())
        }
        Command::Status { service, verbose, json } => {
            // To be implemented - would require connecting to daemon
            println!("Status command placeholder");
            if let Some(s) = service {
                println!("Service: {}", s);
            }
            if verbose {
                println!("Verbose output requested");
            }
            if json {
                println!("JSON output requested");
            }
            Ok(())
        }
        Command::Restart { service, force } => {
            println!("Restart command placeholder");
            if let Some(s) = service {
                println!("Restarting service: {}", s);
            }
            if force {
                println!("Force restart requested");
            }
            Ok(())
        }
        Command::Config(config_cmd) => {
            match config_cmd {
                ConfigCommand::Get { key } => {
                    println!("Get config value: {}", key);
                }
                ConfigCommand::Set { key, value } => {
                    println!("Set {} = {}", key, value);
                }
                ConfigCommand::Reload { validate } => {
                    println!("Reloading configuration{}", if validate { " with validation" } else { "" });
                }
                ConfigCommand::Show { json } => {
                    println!("Show configuration{}", if json { " as JSON" } else { "" });
                }
                ConfigCommand::Validate { path } => {
                    println!("Validating configuration{}", path.map(|p| format!(": {}", p)).unwrap_or_default());
                }
            }
            Ok(())
        }
        Command::Metrics { json: _, service } => {
            println!("Metrics command placeholder");
            if let Some(s) = service {
                println!("Service: {}", s);
            }
            Ok(())
        }
        Command::Logs { service, tail, filter, follow } => {
            println!("Logs command placeholder");
            if let Some(s) = service {
                println!("Service: {}", s);
            }
            if let Some(n) = tail {
                println!("Last {} lines", n);
            }
            if let Some(f) = filter {
                println!("Filter: {}", f);
            }
            if follow {
                println!("Following logs");
            }
            Ok(())
        }
        Command::Debug(debug_cmd) => {
            match debug_cmd {
                DebugCommand::DumpState { service, json: _ } => {
                    println!("Dump state command placeholder");
                    if let Some(s) = service {
                        println!("Service: {}", s);
                    }
                }
                DebugCommand::DumpConnections { format } => {
                    println!("Dump connections placeholder");
                    if let Some(f) = format {
                        println!("Format: {}", f);
                    }
                }
                DebugCommand::HealthCheck { verbose, service } => {
                    println!("Health check placeholder");
                    if verbose {
                        println!("Verbose output");
                    }
                    if let Some(s) = service {
                        println!("Service: {}", s);
                    }
                }
                DebugCommand::Diagnostics { level } => {
                    println!("Diagnostics placeholder (level: {:?})", level);
                }
            }
            Ok(())
        }
    }
}

/// Handler for /metrics endpoint - returns Prometheus format metrics
fn handle_metrics_request() -> String {
    let metrics_collector = Metrics::get_metrics();
    match metrics_collector.export_metrics() {
        Ok(metrics_text) => metrics_text,
        Err(e) => {
            error!("[Metrics] Failed to export metrics: {}", e);
            format!("# ERROR: Failed to export metrics: {}\n", e)
        }
    }
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
    // [Boot] [Observability] Initialize metrics and tracing
    // -------------------------------------------------------------------------
    TraceStep!("[Boot] [Observability] Initializing observability systems...");
    
    if let Err(e) = Metrics::initialize_metrics() {
        error!("[Boot] Failed to initialize metrics: {}", e);
    }
    
    if let Err(e) = Tracing::initialize_tracing() {
        error!("[Boot] Failed to initialize tracing: {}", e);
    }
    
    info!("[Boot] [Observability] Metrics and tracing systems initialized");
    
    // -------------------------------------------------------------------------
    // [Boot] [Args] Parse command line arguments
    // -------------------------------------------------------------------------
    TraceStep!("[Boot] [Args] Parsing command line arguments...");
    
    let (config_path, bind_address, cli_command) = parse_arguments();
    
    // If a CLI command was provided, handle it and exit
    if let Some(cmd) = cli_command {
        return handle_cli_command(cmd).await;
    }
    
    // -------------------------------------------------------------------------
    // [Boot] [Configuration] Load configuration
    // -------------------------------------------------------------------------
    TraceStep!("[Boot] [Configuration] Loading configuration...");
    
    let config_manager = ConfigurationManager::new(config_path)?;
    let configuration: std::sync::Arc<Air::Configuration::AirConfiguration> = std::sync::Arc::new(config_manager.load_configuration().await?);
    
    debug!("[Boot] [Configuration] Configuration loaded successfully");
    
    // -------------------------------------------------------------------------
    // [Boot] [Daemon] Initialize daemon lifecycle management
    // -------------------------------------------------------------------------
    TraceStep!("[Boot] [Daemon] Initializing daemon lifecycle management...");
    
    let daemon_manager = DaemonManager::new(None)?;
    
    // Acquire daemon lock to ensure single instance
    daemon_manager.acquire_lock().await?;
    
    info!("[Boot] [Daemon] Daemon lock acquired");
    
    // -------------------------------------------------------------------------
    // [Boot] [Health] Initialize health check system
    // -------------------------------------------------------------------------
    TraceStep!("[Boot] [Health] Initializing health check system...");
    
    let health_manager: std::sync::Arc<HealthCheckManager> = Arc::new(HealthCheckManager::new(None));
    
    info!("[Boot] [Health] Health check system initialized");
    
    // -------------------------------------------------------------------------
    // [Boot] [State] Initialize application state
    // -------------------------------------------------------------------------
    TraceStep!("[Boot] [State] Initializing application state...");
    
    let app_state: std::sync::Arc<ApplicationState> = Arc::new(ApplicationState::new(configuration.clone()).await?);
    
    info!("[Boot] [State] Application state initialized");
    
    // -------------------------------------------------------------------------
    // [Boot] [Services] Initialize core services
    // -------------------------------------------------------------------------
    TraceStep!("[Boot] [Services] Initializing core services...");
    
    // Authentication Service
    let auth_service: std::sync::Arc<AuthenticationService> = Arc::new(AuthenticationService::new(app_state.clone()).await?);
    
    // Update Manager
    let update_manager: std::sync::Arc<UpdateManager> = Arc::new(UpdateManager::new(app_state.clone()).await?);
    
    // Download Manager
    let download_manager: std::sync::Arc<DownloadManager> = Arc::new(DownloadManager::new(app_state.clone()).await?);
    
    // File Indexer
    let file_indexer: std::sync::Arc<FileIndexer> = Arc::new(FileIndexer::new(app_state.clone()).await?);
    
    info!("[Boot] [Services] Core services initialized");
    
    // -------------------------------------------------------------------------
    // [Boot] [Health] Register services for health monitoring
    // -------------------------------------------------------------------------
    TraceStep!("[Boot] [Health] Registering services for health monitoring...");
    
    health_manager.register_service("authentication".to_string(), HealthCheckLevel::Functional).await?;
    health_manager.register_service("updates".to_string(), HealthCheckLevel::Functional).await?;
    health_manager.register_service("downloader".to_string(), HealthCheckLevel::Functional).await?;
    health_manager.register_service("indexing".to_string(), HealthCheckLevel::Functional).await?;
    health_manager.register_service("grpc".to_string(), HealthCheckLevel::Responsive).await?;
    health_manager.register_service("connections".to_string(), HealthCheckLevel::Alive).await?;
    
    info!("[Boot] [Health] Services registered for health monitoring");
    
    // -------------------------------------------------------------------------
    // [Boot] [Vine] Initialize gRPC server (temporarily disabled)
    // -------------------------------------------------------------------------
    TraceStep!("[Boot] [Vine] Initializing gRPC server...");
    
    let bind_addr: SocketAddr = bind_address
        .unwrap_or_else(|| "[::1]:50053".to_string())
        .parse()?;
    
    info!("[Boot] [Vine] gRPC server would be configured on {} (currently disabled)", bind_addr);
    
    // Create a dummy future that never completes (placeholder for gRPC server)
    let _server = std::future::pending::<Result<(), tonic::transport::Error>>();
    
    // Start connection monitoring background task
    let connection_monitor_handle: tokio::task::JoinHandle<()> = tokio::spawn({
        let app_state = app_state.clone();
        let health_manager = health_manager.clone();
        async move {
            let mut interval = interval(Duration::from_secs(60)); // Check every minute
            loop {
                interval.tick().await;
                
                // Update resource usage
                if let Err(e) = app_state.update_resource_usage().await {
                    warn!("[ConnectionMonitor] Failed to update resource usage: {}", e);
                }
                
                // Get resource metrics
                let resources = app_state.get_resource_usage().await;
                
                // Record metrics
                let metrics_collector = Metrics::get_metrics();
                metrics_collector.update_resource_metrics(
                    resources.memory_usage_mb as u64 * 1024 * 1024,
                    resources.cpu_usage_percent,
                    app_state.get_active_connection_count().await as u64,
                    0, // Active threads - would need to be computed from system
                );
                
                // Clean up stale connections (5 minute timeout)
                if let Err(e) = app_state.cleanup_stale_connections(300).await {
                    warn!("[ConnectionMonitor] Failed to cleanup stale connections: {}", e);
                }
                
                // Perform health checks
                if let Err(e) = health_manager.check_service("connections").await {
                    warn!("[ConnectionMonitor] Health check failed: {}", e);
                    
                    // Record metrics for failed health check
                    let metrics_collector = Metrics::get_metrics();
                    metrics_collector.record_request_failure("health_check_failed", 0.0);
                }
                
                debug!("[ConnectionMonitor] Active connections: {}", app_state.get_active_connection_count().await);
            }
        }
    });
    
    // Register background task
    app_state.register_background_task(connection_monitor_handle).await
        .map_err(|e| format!("Failed to register connection monitor: {}", e))?;
    
    // Start health monitoring background task
    let health_monitor_handle: tokio::task::JoinHandle<()> = tokio::spawn({
        let health_manager = health_manager.clone();
        async move {
            let mut interval = interval(Duration::from_secs(30)); // Check every 30 seconds
            loop {
                interval.tick().await;
                
                // Perform comprehensive health checks
                let services = ["authentication", "updates", "downloader", "indexing", "grpc"];
                for service in services.iter() {
                    if let Err(e) = health_manager.check_service(service).await {
                        warn!("[HealthMonitor] Health check failed for {}: {}", service, e);
                    }
                }
                
                // Log overall health status
                let overall_health = health_manager.get_overall_health().await;
                debug!("[HealthMonitor] Overall health: {:?}", overall_health);
            }
        }
    });
    
    // Register health monitoring task
    app_state.register_background_task(health_monitor_handle).await
        .map_err(|e| format!("Failed to register health monitor: {}", e))?;
    
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
    
    // Wait for shutdown signal (gRPC server temporarily disabled)
    wait_for_shutdown_signal().await;
    
    // -------------------------------------------------------------------------
    // [Shutdown] Graceful shutdown
    // -------------------------------------------------------------------------
    info!("[Shutdown] Initiating graceful shutdown...");
    
    // Stop all background tasks
    if let Err(e) = app_state.stop_all_background_tasks().await {
        error!("[Shutdown] Failed to stop background tasks: {}", e);
    }
    
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
    
    // Log final statistics
    let metrics = app_state.get_metrics().await;
    let resources = app_state.get_resource_usage().await;
    let health_stats = health_manager.get_health_statistics().await;
    
    // Get final metrics data
    let metrics_data = Metrics::get_metrics().get_metrics_data();
    
    info!("[Shutdown] Final statistics - Requests: {} successful, {} failed", 
          metrics.successful_requests, metrics.failed_requests);
    info!("[Shutdown] Final metrics - Success rate: {:.1}%, Error rate: {:.1}%",
          metrics_data.success_rate(), metrics_data.error_rate());
    info!("[Shutdown] Final resource usage - Memory: {:.1}MB, CPU: {:.1}%", 
          resources.memory_usage_mb, resources.cpu_usage_percent);
    info!("[Shutdown] Final health - Overall: {:.1}%, Services: {}/{} healthy", 
          health_stats.overall_health_percentage(), health_stats.healthy_services, health_stats.total_services);
    
    // Release daemon lock
    if let Err(e) = daemon_manager.release_lock().await {
        error!("[Shutdown] Failed to release daemon lock: {}", e);
    }
    
    info!("[Shutdown] All services stopped");
    info!("[Shutdown] Air Daemon 🪁 has shut down gracefully");
    
    Ok(())
}
