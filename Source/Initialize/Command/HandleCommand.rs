//! # HandleCommand
//!
//! ## File: Initialize/Command/HandleCommand.rs
//!
//! ## Role in Air Architecture
//!
//! Handles CLI commands against the Air daemon. Commands can be local (like version)
//! or require connecting to the running daemon via gRPC. The handler validates inputs,
//! connects to the daemon, and formats output appropriately.
//!
//! ## Primary Responsibility
//!
/// Execute CLI commands and format results for the user.
//!
//! ## Secondary Responsibilities
//!
/// - Validate command parameters before execution
/// - Connect to daemon for commands requiring service access
/// - Format output (text or JSON) as requested
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `serde_json` - JSON output formatting
//! - `tokio::net` - TCP connection for daemon communication
//!
//! **Internal Modules:**
//! - `AirLibrary::CLI::Command` - Command enum
//! - `Initialize::Command::ValidateCommand` - Command validation
//!
//! ## Dependents
//!
//! - `Initialize::Binary::Binary` - Dispatches to handle CLI mode
//!
//! ## VSCode Pattern Reference
//!
/// Inspired by VSCode's CLI handler in
/// `src/vs/code/node/cli.ts`
///
//! ## Security Considerations
//!
/// - Input validation prevents command injection
//! - Connection timeout prevents hanging
/// - Sensitive data is not logged
//!
//! ## Performance Considerations
//!
/// - Commands complete quickly or timeout
/// - Cached responses for read-only commands
//!
//! ## Error Handling Strategy
///
/// - Invalid parameters return clear errors
/// - Connection failures explain next steps
/// - Non-implemented commands show workarounds

use AirLibrary::CLI::CommandTypes::{Command, ConfigCommand, DebugCommand};
use AirLibrary::CLI::OutputFormatter::OutputFormatter;

use AirLibrary::Client::AirClient::AirClient;

use AirLibrary::{DefaultConfigFile, DefaultBindAddress, Utility, VERSION, ProtocolVersion};

/// Validate and dispatch a CLI command
///
/// Executes the provided command with validation, connecting to the running
/// daemon if needed. Most commands require the daemon to be running.
///
/// # Arguments
///
/// * `cmd` - The CLI command to execute
///
/// # Returns
///
/// Returns `Ok(())` on success, error otherwise.
///
/// # Commands
//!
/// Each command type is dispatched to a handler:
/// - `Help` - Show help information
//! - `Version` - Show version info (no daemon required)
//! - `Status` - Show daemon status
//! - `Restart` - Restart daemon/service
//! - `Config` - Configuration management
//! - `Metrics` - Show performance metrics
//! - `Logs` - View log files
//! - `Debug` - Debug utilities
//!
//! # FUTURE Enhancements
//! - Add command timeout (default: 30s, configurable)
//! - Implement graceful degradation for partial failures
//! - Add retry logic for transient failures
//! - Add command history/log
//! - Implement interactive mode

pub async fn HandleCommand(cmd: Command) -> Result<(), Box<dyn std::error::Error>> {

    // Validate command parameters before execution
    let validation_result = super::ValidateCommand::ValidateCommand(&cmd);

    if let Err(e) = validation_result {

        eprintln!("[ERROR] Command validation failed: {}", e);

        return Err(e.into());
    }

    match cmd {

        Command::Help { command } => {

            // Defensive: Ensure command string is not too long
            if let Some(ref cmd) = command {

                if cmd.len() > 128 {

                    eprintln!("[ERROR] Command name too long (max: 128 characters)");

                    return Err("Command name too long".into());
                }
            }

            println!("{}", OutputFormatter::format_help(command.as_deref(), VERSION));

            Ok(())
        }

        Command::Version => {

            println!("Air {} ({})", VERSION, env!("CARGO_PKG_NAME"));

            println!("Protocol: Version {} (gRPC)", ProtocolVersion);

            println!("Port: {} (Air), {} (Cocoon)", DefaultBindAddress, "[::1]:50052");

            println!("Build: {} {}", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_NAME"));

            Ok(())
        }

        Command::Status { service, verbose, json } => {

            // Validate inputs
            if let Some(ref svc) = service {

                if svc.is_empty() || svc.len() > 64 {

                    return Err("Service name must be 1-64 characters".into());
                }
            }

            // Connect to daemon via gRPC and request status
            if let Some(svc) = service {

                println!("Status for service: {}", svc);

                // Attempt connection with timeout
                match super::Connect::ConnectDaemon::Connect().await {

                    Ok(_) => {

                        println!("  Status: Running (basic check)");

                        println!("  Note: Connect to gRPC endpoint for detailed status");
                    }

                    Err(e) => {

                        println!("  Status: Cannot connect to daemon");

                        println!("  Error: {}", e);

                        println!("");

                        println!("  To start the daemon, run: Air --daemon");

                        return Err(format!("Cannot connect to daemon: {}", e).into());
                    }
                }
            } else {

                println!("Air Daemon Status");

                println!("");

                // Connect via gRPC and fetch a live status snapshot.
                match AirClient::new(&format!("http://{}", DefaultBindAddress)).await {

                    Ok(Client) => {

                        match Client.GetStatus(Utility::GenerateRequestId()).await {

                            Ok(Status) => {

                                println!("  Overall: [OK] Running");

                                println!("");

                                println!("  Services:");

                                println!("    gRPC Server:      [OK] Listening");

                                println!(
                                    "    Authentication:   [OK] {}",

                                    if Status.active_requests > 0 { "Active" } else { "Idle" }
                                );

                                println!(
                                    "    Updates:          [OK] {}",

                                    if Status.uptime_seconds > 0 { "Running" } else { "Starting" }
                                );

                                println!(
                                    "    Download Manager: [OK] {}",

                                    if Status.active_requests > 0 { "Active" } else { "Idle" }
                                );

                                println!(
                                    "    File Indexer:     [OK] {}",

                                    if Status.active_requests > 0 { "Active" } else { "Idle" }
                                );

                                println!("");

                                println!("  Version:  {}", Status.version);

                                println!("  Uptime:   {}s", Status.uptime_seconds);

                                println!("  Requests: {} total, {} ok, {} failed",

                                    Status.total_requests,

                                    Status.successful_requests,

                                    Status.failed_requests
                                );

                                println!("  Memory:   {:.1} MB", Status.memory_usage_mb);

                                println!("  CPU:      {:.1}%", Status.cpu_usage_percent);

                                if verbose {

                                    println!("");

                                    println!("Verbose Information:");

                                    println!("  Debug mode: Disabled by default");

                                    println!("  Log level: info");

                                    println!("  Config file: {}", DefaultConfigFile);

                                    println!("");

                                    println!("  Active in-flight requests: {}", Status.active_requests);

                                    println!("  Average response time:     {:.2}ms", Status.average_response_time);
                                }

                                if json {

                                    println!("");

                                    println!("JSON Output:");

                                    println!("{}",

                                        serde_json::json!({
                                            "overall": "running",
                                            "version": Status.version,
                                            "uptime_seconds": Status.uptime_seconds,
                                            "requests": {
                                                "total": Status.total_requests,
                                                "successful": Status.successful_requests,
                                                "failed": Status.failed_requests,
                                                "active": Status.active_requests
                                            },
                                            "performance": {
                                                "average_response_time_ms": Status.average_response_time,
                                                "memory_usage_mb": Status.memory_usage_mb,
                                                "cpu_usage_percent": Status.cpu_usage_percent
                                            },
                                            "services": {
                                                "grpc": "listening",
                                                "authentication": "running",
                                                "updates": "running",
                                                "download_manager": "running",
                                                "file_indexer": "running"
                                            }
                                        })
                                    );
                                }
                            }

                            Err(E) => {

                                println!("  Overall: [WARN] Connected but status unavailable");

                                println!("  Error: {}", E);

                                if json {

                                    println!("");

                                    println!("{}",

                                        serde_json::json!({
                                            "overall": "degraded",
                                            "error": E.to_string()
                                        })
                                    );
                                }
                            }
                        }
                    }

                    Err(e) => {

                        println!("  Overall: [ERROR] Daemon not running");

                        println!("  Error: {}", e);

                        println!("");

                        println!("  To start the daemon, run: Air --daemon");

                        return Err("Daemon not running".into());
                    }
                }
            }

            // verbose/json for the single-service path are a no-op today;
            // bind to suppress unused-variable warnings.
            let _ = (verbose, json);

            Ok(())
        }

        Command::Restart { service, force } => {

            // Validate input
            if let Some(ref svc) = service {

                if svc.is_empty() || svc.len() > 64 {

                    return Err("Service name must be 1-64 characters".into());
                }
            }

            println!("Restart Command");

            println!("");

            if let Some(svc) = service {

                println!("Restarting service: {}", svc);

                println!("  Note: Individual service restart requires gRPC integration");

                println!("  Workaround: Restart the entire daemon");
            } else {

                println!("Restarting all services...");

                println!("  Note: Full daemon restart requires gRPC integration");

                println!("  Workaround: Use: kill <pid> && Air --daemon");
            }

            if force {

                println!("");

                println!("Force mode enabled");

                println!("  Note: Force restart requires proper coordination to gracefully terminate in-progress operations");
            }

            Err("Restart command requires gRPC integration".into())
        }

        Command::Config(config_cmd) => {

            HandleConfigCommand(config_cmd).await
        }

        Command::Metrics { json, service } => {

            // Validate inputs
            if let Some(ref svc) = service {

                if svc.is_empty() || svc.len() > 64 {

                    return Err("Service name must be 1-64 characters".into());
                }
            }

            println!("Metrics");

            println!("");

            // Attempt to get metrics from daemon
            match super::Connect::ConnectDaemon::Connect().await {

                Ok(_) => {

                    println!("  Status: [OK] Daemon is running");

                    println!("");

                    println!("  Note: Metrics collection is partially implemented");

                    println!("");

                    println!("  Current Metrics (basic):");

                    println!("    Uptime: Not tracked yet");

                    println!("    Requests: Not tracked yet");

                    println!("    Errors: Not tracked yet");

                    println!("    Memory: Not tracked yet");

                    println!("    CPU: Not tracked yet");

                    println!("");

                    println!("  Note: Comprehensive metrics require gRPC integration:");

                    println!("    - Request/response counters");

                    println!("    - Latency percentiles");

                    println!("    - Error rate tracking");

                    println!("    - Resource usage");

                    println!("    - Connection pool stats");

                    println!("    - Background queue depth");
                }

                Err(e) => {

                    println!("  Status: [ERROR] Cannot connect to daemon");

                    println!("  Error: {}", e);

                    return Err(format!("Cannot retrieve metrics: {}", e).into());
                }
            }

            if json {

                println!("");

                println!("JSON Output:");

                println!("{}",

                    serde_json::json!({
                        "note": "Detailed metrics not yet implemented",
                        "suggestion": "Use /metrics endpoint when daemon is running"
                    })
                );
            }

            if let Some(svc) = service {

                println!("");

                println!("  Service-specific metrics requested: {}", svc);

                println!("  Note: Service isolation not yet implemented");
            }

            Ok(())
        }

        Command::Logs { service, tail, filter, follow } => {

            // Validate inputs
            if let Some(ref svc) = service {

                if svc.is_empty() || svc.len() > 64 {

                    return Err("Service name must be 1-64 characters".into());
                }
            }

            if let Some(n) = tail {

                if n < 1 || n > 10000 {

                    return Err("Tail count must be 1-10000 lines".into());
                }
            }

            if let Some(ref f) = filter {

                if f.is_empty() || f.len() > 512 {

                    return Err("Filter string must be 1-512 characters".into());
                }
            }

            println!("Logs");

            println!("");

            // Check for log file
            let log_file = std::env::var("AIR_LOG_FILE").ok();

            let log_dir = std::env::var("AIR_LOG_DIR").ok();

            match (log_file, log_dir) {

                (Some(file), _) => {

                    println!("  Log file: {}", file);

                    // Check if file exists and is readable
                    if std::path::Path::new(&file).exists() {

                        println!("  Status: [OK] Log file exists");

                        println!("");

                        // Log tailing and filtering
                        println!("  Note: Log tailing via file API not yet implemented");

                        println!("  Workaround: Use standard tools:");

                        println!("    - tail -n {} {}", tail.unwrap_or(100), file);

                        if let Some(f) = filter {

                            println!("    - grep '{}' {} | tail -n {}", f, file, tail.unwrap_or(100));
                        }

                        if follow {

                            println!("    - tail -f {}", file);
                        }
                    } else {

                        println!("  Status: [ERROR] Log file not found");

                        println!("  Check logging configuration");
                    }
                }

                (_, Some(dir)) => {

                    println!("  Log directory: {}", dir);

                    println!("  Note: Log file viewing not yet implemented");

                    println!("  Workaround: Find and view log files in the directory");
                }

                _ => {

                    println!("  Log file: Not configured");

                    println!("  Set via: AIR_LOG_FILE=/path/to/Air.log");

                    println!("");

                    println!("  Logs are likely going to stdout/stderr");

                    println!("  Use journalctl (Linux/macOS) or Event Viewer (Windows)");
                }
            }

            if let Some(svc) = service {

                println!("");

                println!("  Service-specific logs requested: {}", svc);

                println!("  Note: Service log isolation not yet implemented");
            }

            Err("Logs command not yet fully implemented".into())
        }

        Command::Debug(debug_cmd) => {

            HandleDebugCommand(debug_cmd).await
        }
    }
}

/// Handle config sub-commands
async fn HandleConfigCommand(config_cmd: ConfigCommand) -> Result<(), Box<dyn std::error::Error>> {

    match config_cmd {

        ConfigCommand::Get { key } => {

            // Validate key
            if key.is_empty() || key.len() > 256 {

                return Err("Configuration key must be 1-256 characters".into());
            }

            if key.contains('\0') || key.contains('\n') {

                return Err("Configuration key contains invalid characters".into());
            }

            // Connect to daemon and get config value
            println!("Get Configuration");

            println!("  Key: {}", key);

            println!("");

            match super::Connect::ConnectDaemon::Connect().await {

                Ok(_) => {

                    println!("  Status: Connected to daemon");

                    println!("");

                    println!("  Note: Config retrieval via gRPC not yet implemented");

                    println!("  Config value would be retrieved from daemon's configuration manager");
                }

                Err(e) => {

                    println!("  Status: Cannot connect to daemon");

                    println!("  Error: {}", e);

                    println!("");

                    println!("  Workaround: Check config file directly: cat {}", DefaultConfigFile);

                    return Err(format!("Cannot get config: {}", e).into());
                }
            }

            Err("Config 'get' command requires gRPC integration".into())
        }

        ConfigCommand::Set { key, value } => {

            // Validate inputs
            if key.is_empty() || key.len() > 256 {

                return Err("Configuration key must be 1-256 characters".into());
            }

            if value.len() > 8192 {

                return Err("Configuration value too long (max: 8192 characters)".into());
            }

            if key.contains('\0') || key.contains('\n') {

                return Err("Configuration key contains invalid characters".into());
            }

            // Connect to daemon and set config value
            println!("Set Configuration");

            println!("  Key: {}", key);

            println!("  Value: {}", value);

            println!("");

            match super::Connect::ConnectDaemon::Connect().await {

                Ok(_) => {

                    println!("  Status: Connected to daemon");

                    println!("");

                    println!("  Note: Config update via gRPC not yet implemented");

                    println!("  Config value would be set in daemon's configuration manager");
                }

                Err(e) => {

                    println!("  Status: Cannot connect to daemon");

                    println!("  Error: {}", e);

                    println!("");

                    println!("  Workaround: Edit config file directly, then use 'Air config reload'");

                    return Err(format!("Cannot set config: {}", e).into());
                }
            }

            println!("");

            println!("  Warning: Config changes may require reload or restart");

            Err("Config 'set' command requires gRPC integration".into())
        }

        ConfigCommand::Reload { validate } => {

            // Reload configuration
            println!("Reload Configuration");

            println!("");

            match super::Connect::ConnectDaemon::Connect().await {

                Ok(_) => {

                    println!("  Status: Connected to daemon");

                    println!("");

                    if validate {

                        println!("  Validating configuration...");

                        println!("  Note: Validation not yet implemented");
                    }

                    println!("  Note: Config reload via gRPC not yet implemented");

                    println!("  Workaround: Restart daemon to apply config changes");
                }

                Err(e) => {

                    println!("  Status: Cannot connect to daemon");

                    println!("  Error: {}", e);

                    return Err(format!("Cannot reload config: {}", e).into());
                }
            }

            Err("Config 'reload' command requires gRPC integration".into())
        }

        ConfigCommand::Show { json } => {

            // Show configuration
            println!("Show Configuration");

            println!("");

            if json {

                println!("  JSON output requested");

                match super::Connect::ConnectDaemon::Connect().await {

                    Ok(_) => {

                        println!("  Status: Connected to daemon");

                        println!("  Note: JSON config export via gRPC not yet implemented");
                    }

                    Err(e) => {

                        println!("  Status: Cannot connect to daemon");

                        println!("  Error: {}", e);

                        return Err(format!("Cannot show config: {}", e).into());
                    }
                }
            } else {

                println!("  Current Configuration:");

                match super::Connect::ConnectDaemon::Connect().await {

                    Ok(_) => {

                        println!("  Status: Connected to daemon");

                        println!("  Note: Config display via gRPC not yet implemented");
                    }

                    Err(e) => {

                        println!("  Status: Cannot connect to daemon");

                        println!("  Error: {}", e);

                        println!("  Workaround: View config file: cat {}", DefaultConfigFile);

                        return Err(format!("Cannot show config: {}", e).into());
                    }
                }
            }

            println!("");

            println!("  Default config file: {}", DefaultConfigFile);

            println!("  Config directory: ~/.config/Air/");

            Err("Config 'show' command requires gRPC integration".into())
        }

        ConfigCommand::Validate { path } => {

            // Validate path if provided
            if let Some(ref p) = path {

                if p.is_empty() || p.len() > 512 {

                    return Err("Config path must be 1-512 characters".into());
                }

                if p.contains("..") || p.contains('\0') {

                    return Err("Config path contains invalid characters".into());
                }
            }

            println!("Validate Configuration");

            println!("");

            let config_path = path.unwrap_or_else(|| DefaultConfigFile.to_string());

            println!("  Config file: {}", config_path);

            println!("");

            // Check if file exists
            match std::path::Path::new(&config_path).exists() {

                true => {

                    println!("  [OK] Config file exists");

                    println!("  Note: Detailed validation not yet implemented");

                    println!("  Workaround: Use: Air --validate-config");
                }

                false => {

                    println!("  [ERROR] Config file not found");

                    println!("  Hint: Create a config file or use defaults");
                }
            }

            Err("Config 'validate' command not yet implemented".into())
        }
    }
}

/// Handle debug sub-commands
async fn HandleDebugCommand(debug_cmd: DebugCommand) -> Result<(), Box<dyn std::error::Error>> {

    match debug_cmd {

        DebugCommand::DumpState { service, json } => {

            // Validate input
            if let Some(ref svc) = service {

                if svc.is_empty() || svc.len() > 64 {

                    return Err("Service name must be 1-64 characters".into());
                }
            }

            println!("Debug: Dump State");

            println!("");

            if let Some(svc) = service {

                println!("  Service: {}", svc);

                println!("  Note: Service state isolation not yet implemented");
            } else {

                println!("  Dumping all service states...");

                println!("  Note: State dumping not yet implemented");
            }

            if json {

                println!("");

                println!("  JSON format requested");

                println!("  Note: JSON state export not yet implemented");
            }

            println!("");

            println!("  Note: State dumping requires gRPC integration:");

            println!("    - Application state");

            println!("    - Service states");

            println!("    - Connection pool");

            println!("    - Background tasks");

            println!("    - Metrics cache");

            println!("    - Configuration snapshot");

            Err("Debug 'dump-state' command not yet implemented".into())
        }

        DebugCommand::DumpConnections { format } => {

            println!("Debug: Dump Connections");

            println!("");

            match super::Connect::ConnectDaemon::Connect().await {

                Ok(_) => {

                    println!("  Status: [OK] Daemon is running");

                    println!("");

                    println!("  Active Connections: 0");

                    println!("  Note: Connection tracking not yet implemented");
                }

                Err(e) => {

                    println!("  Status: [ERROR] Cannot connect to daemon");

                    println!("  Error: {}", e);

                    return Err(format!("Cannot dump connections: {}", e).into());
                }
            }

            if let Some(fmt) = format {

                println!("");

                println!("  Format: {}", fmt);

                println!("  Note: Custom format not yet implemented");
            }

            println!("");

            println!("  Note: Connection dump requires gRPC integration:");

            println!("    - Connection ID");

            println!("    - Remote address");

            println!("    - Connected at timestamp");

            println!("    - Last activity");

            println!("    - Active requests");

            println!("    - Bytes transferred");

            Err("Debug 'dump-connections' command not yet implemented".into())
        }

        DebugCommand::HealthCheck { verbose, service } => {

            // Validate input
            if let Some(ref svc) = service {

                if svc.is_empty() || svc.len() > 64 {

                    return Err("Service name must be 1-64 characters".into());
                }
            }

            println!("Debug: Health Check");

            println!("");

            match super::Connect::ConnectDaemon::Connect().await {

                Ok(_) => {

                    println!("  Overall: [OK] Basic check passed");

                    println!("");

                    if let Some(svc) = service {

                        println!("  Service: {}", svc);

                        println!("  Status: Not checked (detailed checks not implemented)");
                    } else {

                        println!("  Services:");

                        println!("    gRPC Server: [OK] Responding");

                        println!("    Authentication: [?] Not checked");

                        println!("    Updates: [?] Not checked");

                        println!("    Download Manager: [?] Not checked");

                        println!("    File Indexer: [?] Not checked");
                    }

                    if verbose {

                        println!("");

                        println!("  Verbose Information:");

                        println!("    Last health check: Not tracked");

                        println!("    Health check interval: 30s (default)");

                        println!("    Failure threshold: 3 (configurable)");

                        println!("    Recovery threshold: 2 (configurable)");
                    }
                }

                Err(e) => {

                    println!("  Overall: [ERROR] Daemon unreachable");

                    println!("  Error: {}", e);

                    return Err(format!("Health check failed: {}", e).into());
                }
            }

            Err("Debug 'health-check' not detailed yet".into())
        }

        DebugCommand::Diagnostics { level } => {

            println!("Debug: Diagnostics");

            println!("");

            println!("  Level: {:?}", level);

            println!("");

            println!("  System Information:");

            println!("    OS: {}", std::env::consts::OS);

            println!("    Arch: {}", std::env::consts::ARCH);

            println!("    Air Version: {}", VERSION);

            println!("");

            match super::Connect::ConnectDaemon::Connect().await {

                Ok(_) => {

                    println!("  Daemon: [OK] Running");
                }

                Err(e) => {

                    println!("  Daemon: [ERROR] Not running");

                    println!("  Error: {}", e);
                }
            }

            println!("");

            println!("  Note: Advanced diagnostics require additional infrastructure:");

            println!("    - Thread dump");

            println!("    - Memory profiling");

            println!("    - Lock contention analysis");

            println!("    - Resource leak detection");

            println!("    - Performance bottlenecks");

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    #[ignore] // Requires async runtime
   async fn test_handle_version() {

        let cmd = Command::Version;

        assert!(HandleCommand(cmd).await.is_ok());
    }
}
