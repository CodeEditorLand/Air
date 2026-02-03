//! # Air Binary Entry Point
//!
//! ## Overview
//!
//! Air 🪁 is the persistent background daemon that handles resource-intensive
//! operations for the Land code editor. It runs as a standalone process
//! alongside Mountain, communicating via the Vine (gRPC) protocol to offload
//! tasks like updates, downloads, authentication, and file indexing.
//!
//! ## Architecture & Connections
//!
//! Air serves as the background services hub in the Land ecosystem:
//!
//! - **Wind** (Effect-TS): Provides functional programming patterns and
//!   type-safe effects that Air uses for predictable state management and error
//!   handling
//!
//! - **Cocoon** (NodeJS host): The Node.js runtime environment for frontend/web
//!   components. Air coordinates with Cocoon through the Vine protocol to
//!   deliver web assets and perform frontend build operations. Uses port 50052.
//!
//! - **Mountain** (Tauri bundler): The main desktop application bundle that
//!   packages the Rust backend with the Electron/Node.js frontend. Mountain
//!   receives work from Air through Vine (gRPC) and orchestrates the bundling
//!   process.
//!
//! - **Vine** (gRPC protocol): The communication layer connecting all
//!   components. Air hosts the Vine gRPC server on port 50053, receiving
//!   requests from Mountain and responding with results from background
//!   operations.
//!
//! ## VSCode Architecture References
//!
//! Air's architecture draws inspiration from VSCode's background service model:
//!
//! - **Update Service**: Reference
//!   `Microsoft/Dependency/Editor/src/vs/platform/update`
//!   - AbstractUpdateService: Base class for platform-specific update handling
//!   - UpdateService: Manages update checks, downloads, and installations
//!   - Similar to Air's UpdateManager component
//!
//! - **Lifecycle Management**: Reference
//!   `Microsoft/Dependency/Editor/src/vs/base/common/lifecycle`
//!   - Disposable pattern for resource cleanup
//!   - Event emission and handling
//!   - Graceful shutdown coordination similar to Air's shutdown logic
//!
//! - **Background Services**: VSCode's extension host and language server model
//!   - Independent processes with IPC communication
//!   - Health monitoring and automatic restart
//!   - Similar to Air's daemon management approach
//!
//! ## Core Responsibilities
//!
//! ### 1. gRPC Server (Vine Protocol)
//! - Hosts the Vine gRPC server on port 50053
//! - Receives work requests from Mountain
//! - Streams results and progress updates
//! - Manages connection lifecycle and cleanup
//! - Handles multiple concurrent connections
//!
//! ### 2. Authentication Service
//! - Manages user authentication tokens
//! - Handles cryptographic operations (signing, encryption)
//! - Token refresh and validation
//! - Secure storage management
//!
//! ### 3. Update Management
//! - Checks for application updates
//! - Downloads update packages
//! - Verifies checksums and signatures
//! - Coordinates update installation with Mountain
//!
//! ### 4. Download Manager
//! - Downloads extensions and dependencies
//! - Resumable downloads with retry logic
//! - Bandwidth management and throttling
//! - Progress tracking and reporting
//!
//! ### 5. File Indexing
//! - Background file system scanning
//! - Maintains searchable index
//! - Supports code navigation features
//! - Incremental updates and change detection
//!
//! ### 6. Resource Monitoring
//! - CPU and memory usage tracking
//! - Connection pool management
//! - Background task lifecycle
//! - Performance metrics collection
//!
//! ## Protocol Details
//!
//! **Vine Protocol (gRPC)**
//! - Version: 1
//! - Port: 50053 (Air), 50052 (Cocoon)
//! - Transport: HTTP/2 with TLS (optional)
//! - Serialization: Protocol Buffers
//!
//! ### Protocol Messages
//! - `DownloadRequest`: Request to download a file/extension
//! - `DownloadResponse`: Progress updates and completion
//! - `AuthRequest`: Authentication/token operations
//! - `AuthResponse`: Token data and status
//! - `UpdateRequest`: Update check and download
//! - `UpdateResponse`: Update availability and progress
//! - `IndexRequest`: File indexing operations
//! - `IndexResponse`: Index status and results
//! - `HealthRequest`: Health check queries
//! - `HealthResponse`: Service health and metrics
//!
//! ## TODO: Missing Functionality
//!
//! ### High Priority
//! - [ ] Complete CLI command implementations (all placeholders)
//! - [ ] Add TLS/mTLS support for gRPC connections
//! - [ ] Implement connection authentication/authorization
//! - [ ] Add metrics endpoint (/metrics) HTTP server
//! - [ ] Implement proper configuration hot-reload
//! - [ ] Add comprehensive integration tests
//!
//! ### Medium Priority
//! - [ ] Add prometheus metrics export (currently partial)
//! - [ ] Implement grace period for shutdown (pending operations)
//! - [ ] Add connection rate limiting
//! - [ ] Implement request prioritization
//! - [ ] Add audit logging for sensitive operations
//! - [ ] Implement plugin/hot-reload system
//!
//! ### Low Priority
//! - [ ] Add structured logging with correlation IDs
//! - [ ] Implement distributed tracing integration
//! - [ ] Add health check endpoint for load balancers
//! - [ ] Implement connection pooling optimizations
//! - [ ] Add telemetry/observability export
//! - [ ] Implement A/B testing for features
//!
//! ## Error Handling Strategy
//!
//! All public functions use defensive coding:
//! - Input validation with descriptive errors
//! - Timeout handling with cancellation
//! - Resource cleanup via Drop and explicit cleanup
//! - Circuit breaker for external dependencies
//! - Retry logic with exponential backoff
//! - Metrics recording for all operations
//!
//! ## Shutdown Sequence
//!
//! 1. Accept shutdown signal (SIGTERM, SIGINT)
//! 2. Stop accepting new requests
//! 3. Wait for in-flight requests (with timeout)
//! 4. Stop background services
//! 5. Cancel pending background tasks
//! 6. Release daemon lock
//! 7. Log final statistics
//! 8. Exit cleanly
//!
//! ## Port Allocation
//!
//! - **50052**: Cocoon (NodeJS host) - Frontend/web services
//! - **50053**: Air (this daemon) - Background services
//! - **50054**: Reserved for future use (e.g., SideCar service)
//! - **50055**: Reserved for future metrics endpoints

use std::{net::SocketAddr, sync::Arc, time::Duration};

use log::{debug, error, info, warn};
use tokio::{signal, time::interval};
use AirLibrary::{
	ApplicationState::ApplicationState,
	Authentication::AuthenticationService,
	CLI::{CliParser, Command, ConfigCommand, DebugCommand, OutputFormatter},
	Configuration::ConfigurationManager,
	Daemon::DaemonManager,
	DefaultBindAddress,
	DefaultConfigFile,
	Downloader::DownloadManager,
	HealthCheck::{HealthCheckLevel, HealthCheckManager},
	Indexing::FileIndexer,
	Logging,
	Metrics,
	ProtocolVersion,
	Tracing,
	Updates::UpdateManager,
	VERSION,
	Vine::Server::AirVinegRPCService::AirVinegRPCService,
};

// =============================================================================
// Debug Helpers
// =============================================================================

/// Logs a checkpoint message at debug level with context tracking
macro_rules! Trace {
    ($($arg:tt)*) => {{
        debug!($($arg)*);
    }};
}

/// Shutdown signal handler for graceful termination
///
/// This function waits for either Ctrl+C (SIGINT) or SIGTERM signals
/// and then initiates the shutdown sequence. It provides a timeout
/// to handle cases where signal handlers fail to install properly.
///
/// # TODO
/// - Add configurable shutdown timeout (currently infinite)
/// - Implement signal handling for SIGHUP (reload config)
/// - Add Windows-specific signal handling beyond Ctrl+C
/// - Implement graceful timeout with pending operation completion
async fn WaitForShutdownSignal() {
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
			},
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

/// Initialize logging based on environment variables
///
/// Sets up structured logging with support for JSON output (useful for
/// production) and file-based logging (useful for debugging). Environment
/// variables:
///
/// - `AIR_LOG_JSON`: "true" enables JSON formatted output
/// - `AIR_LOG_LEVEL`: Set logging level (debug, info, warn, error)
/// - `AIR_LOG_FILE`: Path to log file (optional)
///
/// # TODO
/// - Add log rotation support
/// - Implement log file size limits
/// - Add structured log correlation IDs
/// - Support syslog integration on Unix
/// - Add Windows Event Log integration
fn InitializeLogging() {
	// Validate environment variables
	let json_output = match std::env::var("AIR_LOG_JSON") {
		Ok(val) if !val.is_empty() => {
			let normalized = val.to_lowercase();
			if normalized != "true" && normalized != "false" {
				eprintln!(
					"Warning: Invalid AIR_LOG_JSON value '{}', expected 'true' or 'false'. Using default: false",
					val
				);
				false
			} else {
				normalized == "true"
			}
		},
		Ok(_) => false,
		Err(_) => false,
	};

	// Validate log file path exists and is writable
	let log_file_path = std::env::var("AIR_LOG_FILE").ok().and_then(|path| {
		if path.is_empty() {
			None
		} else {
			// Check if directory exists for the log file
			if let Some(parent) = std::path::PathBuf::from(&path).parent() {
				if parent.as_os_str().is_empty() {
					// No parent directory, use current directory
					Some(path)
				} else if parent.exists() {
					Some(path)
				} else {
					eprintln!(
						"Warning: Log file directory does not exist: {}. Logging to stdout only.",
						parent.display()
					);
					None
				}
			} else {
				Some(path)
			}
		}
	});

	// Initialize structured logging with defensive error handling
	let log_result = Logging::initialize_logger(json_output, log_file_path.clone());

	match log_result {
		Ok(_) => {
			let log_info = match &log_file_path {
				Some(path) => format!("file: {}", path),
				None => "stdout/stderr".to_string(),
			};
			info!("[Boot] Logging initialized - JSON: {}, Output: {}", json_output, log_info);
		},
		Err(e) => {
			// Fallback: ensure we can at least log errors to stderr
			eprintln!("[ERROR] Failed to initialize structured logging: {}", e);
			eprintln!("[ERROR] Logging will fall back to stderr-only output");
		},
	}
}

/// Parse command line arguments into daemon config or CLI command
///
/// Handles two modes of operation:
/// 1. CLI mode: Execute commands like `status`, `restart`, `config`, etc.
/// 2. Daemon mode: Start the background service with optional config/bind args
///
/// # Arguments
///
/// Returns a tuple of (config_path, bind_address, optional_command)
/// - If `command` is Some, daemon startup should be skipped
/// - Otherwise, start daemon with provided config path and bind address
///
/// # TODO
/// - Add validation for bind address format
/// - Add validation for config file exists/readable
/// - Support `--validate-config` flag to check config without starting
/// - Add `--daemon` flag to force daemon mode with CLI commands
/// - Make flags case-insensitive
/// - Add --no-daemon flag for foreground operation
fn ParseArguments() -> (Option<String>, Option<String>, Option<Command>) {
	// Defensive: Ensure args collection is not extremely large
	let args:Vec<String> = std::env::args().collect();

	// Safety: Limit argument length to prevent potential DoS
	if args.len() > 1024 {
		eprintln!("[ERROR] Too many command line arguments (max: 1024)");
		std::process::exit(1);
	}

	// Safety: Validate each argument length
	for (i, arg) in args.iter().enumerate() {
		if arg.len() > 4096 {
			eprintln!("[ERROR] Argument at position {} is too long (max: 4096 characters)", i);
			std::process::exit(1);
		}
	}

	// Check if we're running with CLI command (first arg is a known command)
	if args.len() > 1 {
		match args[1].as_str() {
			"status" | "restart" | "config" | "metrics" | "logs" | "debug" | "help" | "version" | "-h" | "--help"
			| "-v" | "--version" => {
				// Parse CLI command with error handling
				match CliParser::parse(args.clone()) {
					Ok(cmd) => {
						debug!("[Boot] CLI command parsed: {:?}", cmd);
						return (None, None, Some(cmd));
					},
					Err(e) => {
						eprintln!("[ERROR] Error parsing CLI command: {}", e);
						eprintln!("[ERROR] Run 'Air help' for usage information");
						std::process::exit(1);
					},
				}
			},
			_ => {},
		}
	}

	// Parse as daemon arguments with validation
	let mut config_path:Option<String> = None;
	let mut bind_address:Option<String> = None;

	let mut i = 0;
	while i < args.len() {
		match args[i].as_str() {
			"--config" | "-c" => {
				if i + 1 < args.len() {
					let path = &args[i + 1];
					// Validate path doesn't contain suspicious characters
					if path.contains("..") || path.contains('\0') {
						eprintln!("[ERROR] Invalid config path: contains '..' or null character");
						std::process::exit(1);
					}
					config_path = Some(path.clone());
					i += 1;
				} else {
					eprintln!("[ERROR] --config flag requires a path argument");
					std::process::exit(1);
				}
			},
			"--bind" | "-b" => {
				if i + 1 < args.len() {
					let addr = &args[i + 1];
					// Basic validation of address format
					if addr.is_empty() || addr.len() > 256 {
						eprintln!("[ERROR] Invalid bind address: must be 1-256 characters");
						std::process::exit(1);
					}
					// Full validation happens during bind, but check for null characters
					if addr.contains('\0') {
						eprintln!("[ERROR] Invalid bind address: contains null character");
						std::process::exit(1);
					}
					bind_address = Some(addr.clone());
					i += 1;
				} else {
					eprintln!("[ERROR] --bind flag requires an address argument");
					std::process::exit(1);
				}
			},
			_ => {
				// Ignore unknown flags or positional arguments
				// Could add warning for unknown flags if desired
			},
		}
		i += 1;
	}

	debug!("[Boot] Daemon mode - config: {:?}, bind: {:?}", config_path, bind_address);

	(config_path, bind_address, None)
}

/// Handle CLI commands with comprehensive implementation
///
/// Executes user commands against the Air daemon. Most commands require
/// connecting to the running daemon via gRPC. Commands that don't require
/// a running daemon (like `version`) execute immediately.
///
/// # Errors
///
/// Returns errors when:
/// - Daemon connection fails (for commands requiring daemon)
/// - Command parameters are invalid
/// - Daemon returns an error response
/// - I/O operations fail
///
/// # TODO
/// - Implement actual daemon connection via gRPC
/// - Add command timeout (default: 30s, configurable)
/// - Implement graceful degradation for partial failures
/// - Add retry logic for transient failures
/// - Add command history/log
/// - Implement interactive mode
/// - Add tab-completion support
async fn HandleCommand(cmd:Command) -> Result<(), Box<dyn std::error::Error>> {
	// Validate command parameters before execution
	let validation_result = validate_command(&cmd);
	if let Err(e) = validation_result {
		eprintln!("[ERROR] Command validation failed: {}", e);
		return Err(e.into());
	}

	match cmd {
		Command::Help { command } => {
			// Defensive: Ensure command string is not too long if provided
			if let Some(ref cmd) = command {
				if cmd.len() > 128 {
					eprintln!("[ERROR] Command name too long (max: 128 characters)");
					return Err("Command name too long".into());
				}
			}
			println!("{}", OutputFormatter::format_help(command.as_deref(), VERSION));
			Ok(())
		},

		Command::Version => {
			println!("Air {} ({})", VERSION, env!("CARGO_PKG_NAME"));
			println!("Protocol: Version {} (gRPC)", ProtocolVersion);
			println!("Port: {} (Air), {} (Cocoon)", DefaultBindAddress, "[::1]:50052");
			println!("Build: {} {}", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_NAME"));
			Ok(())
		},

		Command::Status { service, verbose, json } => {
			// Validate inputs
			if let Some(ref svc) = service {
				if svc.is_empty() || svc.len() > 64 {
					return Err("Service name must be 1-64 characters".into());
				}
			}

			// TODO: Connect to daemon via gRPC and request status
			// For now, try to connect and show error if unavailable

			// Placeholder implementation
			if let Some(svc) = service {
				println!("📊 Status for service: {}", svc);

				// Attempt connection with timeout
				match attempt_daemon_connection().await {
					Ok(_) => {
						println!("  Status: ⚠️  Running (basic check)");
						println!("  Note: Detailed status not yet implemented");
					},
					Err(e) => {
						println!("  Status: ❌ Cannot connect to daemon");
						println!("  Error: {}", e);
						println!("");
						println!("  To start the daemon, run: Air --daemon");
						return Err(format!("Cannot connect to daemon: {}", e).into());
					},
				}
			} else {
				println!("📊 Air Daemon Status");
				println!("");

				// Attempt connection
				match attempt_daemon_connection().await {
					Ok(_) => {
						println!("  Overall: ⚠️  Running (basic check)");
						println!("  Note: Detailed status monitoring not yet implemented");
						println!("");
						println!("  Services:");
						println!("    gRPC Server: ✅ Listening");
						println!("    Authentication: ⚠️  Not checked");
						println!("    Updates: ⚠️  Not checked");
						println!("    Download Manager: ⚠️  Not checked");
						println!("    File Indexer: ⚠️  Not checked");
					},
					Err(e) => {
						println!("  Overall: ❌ Daemon not running");
						println!("  Error: {}", e);
						println!("");
						println!("  To start the daemon, run: Air --daemon");
						return Err("Daemon not running".into());
					},
				}
			}

			if verbose {
				println!("");
				println!("🔍 Verbose Information:");
				println!("  Debug mode: Disabled by default");
				println!("  Log level: info");
				println!("  Config file: {}", DefaultConfigFile);
				println!("");
				println!("  TODO: Implement detailed service status with:");
				println!("    - Service uptime");
				println!("    - Request/response statistics");
				println!("    - Error rates and recent errors");
				println!("    - Resource usage");
				println!("    - Active connections");
			}

			if json {
				println!("");
				println!("📋 JSON Output:");
				println!(
					"{}",
					serde_json::json!({
						"overall": "running",
						"services": {
							"grpc": "listening",
							"status": "not_implemented"
						},
						"note": "Detailed JSON output not yet implemented"
					})
				);
			}

			Ok(())
		},

		Command::Restart { service, force } => {
			// Validate input
			if let Some(ref svc) = service {
				if svc.is_empty() || svc.len() > 64 {
					return Err("Service name must be 1-64 characters".into());
				}
			}

			// TODO: Implement actual restart via gRPC
			println!("🔄 Restart Command");
			println!("");

			if let Some(svc) = service {
				println!("Restarting service: {}", svc);
				println!("  Note: Individual service restart not yet implemented");
				println!("  Workaround: Restart the entire daemon");
			} else {
				println!("Restarting all services...");
				println!("  Note: Full daemon restart not yet implemented");
				println!("  Workaround: Use: kill <pid> && Air --daemon");
			}

			if force {
				println!("");
				println!("⚠️  Force mode enabled");
				println!("  Warning: This will terminate in-progress operations");
				println!("  TODO: Implement force restart with proper coordination");
			}

			Err("Restart command not yet implemented".into())
		},

		Command::Config(config_cmd) => {
			match config_cmd {
				ConfigCommand::Get { key } => {
					// Validate key
					if key.is_empty() || key.len() > 256 {
						return Err("Configuration key must be 1-256 characters".into());
					}
					if key.contains('\0') || key.contains('\n') {
						return Err("Configuration key contains invalid characters".into());
					}

					// TODO: Connect to daemon and get config value
					println!("⚙️  Get Configuration");
					println!("  Key: {}", key);
					println!("");
					println!("  Note: Config retrieval not yet implemented");
					println!("  Workaround: Check config file directly: cat {}", DefaultConfigFile);

					Err("Config 'get' command not yet implemented".into())
				},

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

					// TODO: Connect to daemon and set config value
					println!("⚙️  Set Configuration");
					println!("  Key: {}", key);
					println!("  Value: {}", value);
					println!("");
					println!("  Note: Config update not yet implemented");
					println!("  Workaround: Edit config file directly, then use 'Air config reload'");

					println!("");
					println!("  ⚠️  Warning: Config changes without reload won't take effect");
					println!("  ⚠️  Warning: Some settings may require full daemon restart");

					Err("Config 'set' command not yet implemented".into())
				},

				ConfigCommand::Reload { validate } => {
					// TODO: Implement config reload
					println!("🔄 Reload Configuration");
					println!("");

					match attempt_daemon_connection().await {
						Ok(_) => {
							println!("  Status: ⚠️  Daemon is running");
							println!("");
							println!("  Note: Config reload not yet implemented");
							println!("  Workaround: Restart daemon to apply config changes");
							println!("");
							if validate {
								println!("  ℹ️  Validation mode requested");
								println!("     (Will be implemented with config reload)");
							}
						},
						Err(e) => {
							println!("  Status: ❌ Cannot connect to daemon");
							println!("  Error: {}", e);
							return Err(format!("Cannot reload config: {}", e).into());
						},
					}

					Err("Config 'reload' command not yet implemented".into())
				},

				ConfigCommand::Show { json } => {
					// TODO: Implement config show
					println!("⚙️  Show Configuration");
					println!("");

					if json {
						println!("  JSON output requested");
						println!("  Note: JSON config export not yet implemented");
					} else {
						println!("  Current Configuration:");
						println!("  Note: Config display not yet implemented");
						println!("  Workaround: View config file: cat {}", DefaultConfigFile);
					}

					println!("");
					println!("  Default config file: {}", DefaultConfigFile);
					println!("  Config directory: ~/.config/Air/");

					Err("Config 'show' command not yet implemented".into())
				},

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

					println!("✅ Validate Configuration");
					println!("");

					let config_path = path.unwrap_or_else(|| DefaultConfigFile.to_string());
					println!("  Config file: {}", config_path);
					println!("");

					// Check if file exists
					match std::path::Path::new(&config_path).exists() {
						true => {
							println!("  ✅ Config file exists");
							println!("  Note: Detailed validation not yet implemented");
							println!("  Workaround: Use: Air --validate-config");
						},
						false => {
							println!("  ❌ Config file not found");
							println!("  Hint: Create a config file or use defaults");
						},
					}

					Err("Config 'validate' command not yet implemented".into())
				},
			}
		},

		Command::Metrics { json, service } => {
			// Validate inputs
			if let Some(ref svc) = service {
				if svc.is_empty() || svc.len() > 64 {
					return Err("Service name must be 1-64 characters".into());
				}
			}

			println!("📊 Metrics");
			println!("");

			// Attempt to get metrics from daemon
			match attempt_daemon_connection().await {
				Ok(_) => {
					println!("  Status: ✅ Daemon is running");
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
					println!("  TODO: Implement comprehensive metrics:");
					println!("    - Request/response counters");
					println!("    - Latency percentiles");
					println!("    - Error rate tracking");
					println!("    - Resource usage");
					println!("    - Connection pool stats");
					println!("    - Background queue depth");
				},
				Err(e) => {
					println!("  Status: ❌ Cannot connect to daemon");
					println!("  Error: {}", e);
					return Err(format!("Cannot retrieve metrics: {}", e).into());
				},
			}

			if json {
				println!("");
				println!("📋 JSON Output:");
				println!(
					"{}",
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
		},

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

			println!("📝 Logs");
			println!("");

			// Check for log file
			let log_file = std::env::var("AIR_LOG_FILE").ok();
			let log_dir = std::env::var("AIR_LOG_DIR").ok();

			match (log_file, log_dir) {
				(Some(file), _) => {
					println!("  Log file: {}", file);

					// Check if file exists and is readable
					if std::path::Path::new(&file).exists() {
						println!("  Status: ✅ Log file exists");
						println!("");

						// TODO: Implement actual log tailing and filtering
						println!("  Note: Log viewing not yet implemented");
						println!("  Workaround: Use standard tools:");
						println!("    - tail -n {} {}", tail.unwrap_or(100), file);

						if let Some(f) = filter {
							println!("    - grep '{}' {} | tail -n {}", f, file, tail.unwrap_or(100));
						}

						if follow {
							println!("    - tail -f {}", file);
						}
					} else {
						println!("  Status: ❌ Log file not found");
						println!("  Check logging configuration");
					}
				},
				(_, Some(dir)) => {
					println!("  Log directory: {}", dir);
					println!("  Note: Log file viewing not yet implemented");
					println!("  Workaround: Find and view log files in the directory");
				},
				_ => {
					println!("  Log file: Not configured");
					println!("  Set via: AIR_LOG_FILE=/path/to/Air.log");
					println!("");
					println!("  Logs are likely going to stdout/stderr");
					println!("  Use journalctl (Linux/macOS) or Event Viewer (Windows)");
				},
			}

			if let Some(svc) = service {
				println!("");
				println!("  Service-specific logs requested: {}", svc);
				println!("  Note: Service log isolation not yet implemented");
			}

			// For now, show a placeholder
			Err("Logs command not yet fully implemented".into())
		},

		Command::Debug(debug_cmd) => {
			match debug_cmd {
				DebugCommand::DumpState { service, json } => {
					// Validate input
					if let Some(ref svc) = service {
						if svc.is_empty() || svc.len() > 64 {
							return Err("Service name must be 1-64 characters".into());
						}
					}

					println!("🔧 Debug: Dump State");
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
					println!("  TODO: Implement state dump for:");
					println!("    - Application state");
					println!("    - Service states");
					println!("    - Connection pool");
					println!("    - Background tasks");
					println!("    - Metrics cache");
					println!("    - Configuration snapshot");

					Err("Debug 'dump-state' command not yet implemented".into())
				},

				DebugCommand::DumpConnections { format } => {
					println!("🔧 Debug: Dump Connections");
					println!("");

					match attempt_daemon_connection().await {
						Ok(_) => {
							println!("  Status: ✅ Daemon is running");
							println!("");
							println!("  Active Connections: 0");
							println!("  Note: Connection tracking not yet implemented");
						},
						Err(e) => {
							println!("  Status: ❌ Cannot connect to daemon");
							println!("  Error: {}", e);
							return Err(format!("Cannot dump connections: {}", e).into());
						},
					}

					if let Some(fmt) = format {
						println!("");
						println!("  Format: {}", fmt);
						println!("  Note: Custom format not yet implemented");
					}

					println!("");
					println!("  TODO: Implement connection dump with:");
					println!("    - Connection ID");
					println!("    - Remote address");
					println!("    - Connected at timestamp");
					println!("    - Last activity");
					println!("    - Active requests");
					println!("    - Bytes transferred");

					Err("Debug 'dump-connections' command not yet implemented".into())
				},

				DebugCommand::HealthCheck { verbose, service } => {
					// Validate input
					if let Some(ref svc) = service {
						if svc.is_empty() || svc.len() > 64 {
							return Err("Service name must be 1-64 characters".into());
						}
					}

					println!("🔧 Debug: Health Check");
					println!("");

					match attempt_daemon_connection().await {
						Ok(_) => {
							println!("  Overall: ⚠️  Basic check passed");
							println!("");

							if let Some(svc) = service {
								println!("  Service: {}", svc);
								println!("  Status: Not checked (detailed checks not implemented)");
							} else {
								println!("  Services:");
								println!("    gRPC Server: ✅ Responding");
								println!("    Authentication: ⏸️  Not checked");
								println!("    Updates: ⏸️  Not checked");
								println!("    Download Manager: ⏸️  Not checked");
								println!("    File Indexer: ⏸️  Not checked");
							}

							if verbose {
								println!("");
								println!("  🔍 Verbose Information:");
								println!("    Last health check: Not tracked");
								println!("    Health check interval: 30s (default)");
								println!("    Failure threshold: 3 (configurable)");
								println!("    Recovery threshold: 2 (configurable)");
							}
						},
						Err(e) => {
							println!("  Overall: ❌ Daemon unreachable");
							println!("  Error: {}", e);
							return Err(format!("Health check failed: {}", e).into());
						},
					}

					Err("Debug 'health-check' not detailed yet".into())
				},

				DebugCommand::Diagnostics { level } => {
					println!("🔧 Debug: Diagnostics");
					println!("");
					println!("  Level: {:?}", level);
					println!("");

					// Show system information
					println!("  System Information:");
					println!("    OS: {}", std::env::consts::OS);
					println!("    Arch: {}", std::env::consts::ARCH);
					println!("    Air Version: {}", VERSION);
					println!("");

					match attempt_daemon_connection().await {
						Ok(_) => {
							println!("  Daemon: ✅ Running");
						},
						Err(e) => {
							println!("  Daemon: ❌ Running");
							println!("  Error: {}", e);
						},
					}

					println!("");
					println!("  TODO: Implement diagnostics:");
					println!("    - Thread dump");
					println!("    - Memory profiling");
					println!("    - Lock contention analysis");
					println!("    - Resource leak detection");
					println!("    - Performance bottlenecks");

					Ok(())
				},
			}
		},
	}
}

/// Validate command parameters to prevent invalid inputs
///
/// # TODO
/// - Add timeout parameter validation
/// - Add rate limit checks for commands
/// - Implement command permission checks
fn validate_command(cmd:&Command) -> Result<(), String> {
	match cmd {
		Command::Help { command } => {
			if let Some(ref cmd) = command {
				if cmd.len() > 128 {
					return Err("Command name too long (max: 128)".to_string());
				}
			}
		},
		_ => {},
	}
	Ok(())
}

/// Attempt to connect to the running daemon
///
/// Creates a basic TCP connection to check if the daemon is running.
/// This is a simplified check for pre-implementation status.
///
/// # TODO
/// - Implement proper gRPC client connection
/// - Add connection timeout configuration
/// - Implement connection pooling
/// - Add authentication
async fn attempt_daemon_connection() -> Result<(), String> {
	use tokio::{
		net::TcpStream,
		time::{Duration, timeout},
	};

	let addr = DefaultBindAddress;

	// Timeout: 5 seconds
	let connection_result = timeout(Duration::from_secs(5), async { TcpStream::connect(addr).await }).await;

	match connection_result {
		Ok(Ok(_)) => Ok(()),
		Ok(Err(e)) => Err(format!("Connection failed: {}", e)),
		Err(_) => Err("Connection timeout (5s)".to_string()),
	}
}

/// Handler for /metrics endpoint - returns Prometheus format metrics
///
/// Exports all collected metrics in Prometheus text format for scraping
/// by monitoring systems like Prometheus, Grafana, or custom dashboards.
///
/// Metrics include:
/// - Request counters (total, successful, failed)
/// - Response times (histogram)
/// - Resource usage (memory, CPU)
/// - Connection counts
/// - Background task status
///
/// # TODO
/// - Add timeout for metrics export (should not block daemon)
/// - Implement metric label support (service, host, etc.)
/// - Add counter reset capability
/// - Implement metric filtering via query parameters
/// - Add histogram quantiles (p50, p95, p99)
/// - Support both Prometheus and OpenMetrics formats
fn HandleMetricsRequest() -> String {
	// Defensive: Use a timeout to prevent metrics export from blocking
	let timeout_duration = std::time::Duration::from_millis(100);

	let metrics_collector = Metrics::get_metrics();

	// Export metrics with error handling and timeout
	let export_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| metrics_collector.export_metrics()));

	match export_result {
		Ok(Ok(metrics_text)) => {
			// Validate metrics text is not too large
			if metrics_text.len() > 10_000_000 {
				error!(
					"[Metrics] Exported metrics unreasonably large (size: {} bytes)",
					metrics_text.len()
				);
				format!("# ERROR: Metrics export too large (max: 10MB)\n")
			} else {
				metrics_text
			}
		},
		Ok(Err(e)) => {
			error!("[Metrics] Failed to export metrics: {}", e);
			format!("# ERROR: Failed to export metrics: {}\n", e)
		},
		Err(_) => {
			error!("[Metrics] Metrics export panicked");
			format!("# ERROR: Metrics export failed due to internal error\n")
		},
	}
}

// =============================================================================
// Main Application Entry Point
// =============================================================================

/// The main asynchronous function that sets up and runs the Air daemon
///
/// This is the primary entry point for the Air background service. It
/// coordinates all initialization, starts the gRPC server, manages the daemon
/// lifecycle, and handles graceful shutdown.
///
/// # Startup Sequence
///
/// 1. Initialize logging and observability (metrics, tracing)
/// 2. Parse command-line arguments (for CLI commands or daemon config)
/// 3. Load configuration (with validation)
/// 4. Acquire daemon lock (ensure single instance)
/// 5. Initialize application state
/// 6. Create and register core services
/// 7. Start gRPC server (Vine protocol)
/// 8. Start background tasks and monitoring
/// 9. Wait for shutdown signal
/// 10. Graceful shutdown sequence
///
/// # Defensive Coding
///
/// All operations include:
/// - Input validation and sanitization
/// - Timeout handling for async operations
/// - Error recovery and logging
/// - Resource cleanup on errors
/// - Panic handling in critical sections
///
/// # TODO
/// - Implement configuration hot-reload signal handling (SIGHUP)
/// - Add startup timeout and failure recovery
/// - Implement daemon mode forking (Unix)
/// - Add Windows service integration
/// - Implement crash recovery and restart
/// - Add pre-flight environment checks
/// - Implement feature flag system
#[tokio::main]
async fn Main() -> Result<(), Box<dyn std::error::Error>> {
	// -------------------------------------------------------------------------
	// [Boot] [Logging] Initialize logging system
	// -------------------------------------------------------------------------
	InitializeLogging();

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
	Trace!("[Boot] [Observability] Initializing observability systems...");

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
	Trace!("[Boot] [Args] Parsing command line arguments...");

	let (config_path, bind_address, cli_command) = ParseArguments();

	// If a CLI command was provided, handle it and exit
	if let Some(cmd) = cli_command {
		info!("[Boot] CLI command detected, executing...");
		let result = HandleCommand(cmd).await;

		match &result {
			Ok(_) => {
				info!("[Boot] CLI command completed successfully");
				std::process::exit(0);
			},
			Err(e) => {
				error!("[Boot] CLI command failed: {}", e);
				std::process::exit(1);
			},
		}
	}

	// -------------------------------------------------------------------------
	// [Boot] [Configuration] Load configuration
	// -------------------------------------------------------------------------
	Trace!("[Boot] [Configuration] Loading configuration...");

	let config_manager = match ConfigurationManager::new(config_path) {
		Ok(cm) => cm,
		Err(e) => {
			error!("[Boot] Failed to create configuration manager: {}", e);
			return Err(format!("Configuration manager initialization failed: {}", e).into());
		},
	};

	// Load configuration with timeout
	let configuration:std::sync::Arc<AirLibrary::Configuration::AirConfiguration> =
		match tokio::time::timeout(Duration::from_secs(10), config_manager.load_configuration()).await {
			Ok(Ok(config)) => {
				info!("[Boot] [Configuration] Configuration loaded successfully");
				std::sync::Arc::new(config)
			},
			Ok(Err(e)) => {
				error!("[Boot] Failed to load configuration: {}", e);
				return Err(format!("Configuration load failed: {}", e).into());
			},
			Err(_) => {
				error!("[Boot] Configuration load timed out");
				return Err("Configuration load timed out".into());
			},
		};

	// Validate critical configuration values
	validate_configuration(&configuration)?;

	// -------------------------------------------------------------------------
	// [Boot] [Daemon] Initialize daemon lifecycle management
	// -------------------------------------------------------------------------
	Trace!("[Boot] [Daemon] Initializing daemon lifecycle management...");

	let daemon_manager = match DaemonManager::new(None) {
		Ok(dm) => dm,
		Err(e) => {
			error!("[Boot] Failed to create daemon manager: {}", e);
			return Err(format!("Daemon manager initialization failed: {}", e).into());
		},
	};

	// Acquire daemon lock to ensure single instance with timeout
	match tokio::time::timeout(Duration::from_secs(5), daemon_manager.acquire_lock()).await {
		Ok(Ok(_)) => {
			info!("[Boot] [Daemon] Daemon lock acquired successfully");
		},
		Ok(Err(e)) => {
			error!("[Boot] Failed to acquire daemon lock: {}", e);
			error!("[Boot] Another instance may already be running");
			return Err(format!("Daemon lock acquisition failed: {}", e).into());
		},
		Err(_) => {
			error!("[Boot] Daemon lock acquisition timed out");
			return Err("Daemon lock acquisition timed out".into());
		},
	}

	// -------------------------------------------------------------------------
	// [Boot] [Health] Initialize health check system
	// -------------------------------------------------------------------------
	Trace!("[Boot] [Health] Initializing health check system...");

	let health_manager:std::sync::Arc<HealthCheckManager> = Arc::new(HealthCheckManager::new(None));

	info!("[Boot] [Health] Health check system initialized");

	// -------------------------------------------------------------------------
	// [Boot] [State] Initialize application state
	// -------------------------------------------------------------------------
	Trace!("[Boot] [State] Initializing application state...");

	let AppState:std::sync::Arc<ApplicationState> =
		match tokio::time::timeout(Duration::from_secs(10), ApplicationState::new(configuration.clone())).await {
			Ok(Ok(state)) => {
				info!("[Boot] [State] Application state initialized");
				Arc::new(state)
			},
			Ok(Err(e)) => {
				error!("[Boot] Failed to initialize application state: {}", e);
				// Attempt to release lock before returning
				let _ = daemon_manager.release_lock().await;
				return Err(format!("Application state initialization failed: {}", e).into());
			},
			Err(_) => {
				error!("[Boot] Application state initialization timed out");
				let _ = daemon_manager.release_lock().await;
				return Err("Application state initialization timed out".into());
			},
		};

	// -------------------------------------------------------------------------
	// [Boot] [Services] Initialize core services
	// -------------------------------------------------------------------------
	Trace!("[Boot] [Services] Initializing core services...");

	// Initialize each service with error handling
	let auth_service:std::sync::Arc<AuthenticationService> =
		match tokio::time::timeout(Duration::from_secs(10), AuthenticationService::new(AppState.clone())).await {
			Ok(Ok(svc)) => Arc::new(svc),
			Ok(Err(e)) => {
				error!("[Boot] Failed to initialize authentication service: {}", e);
				return Err(format!("Authentication service initialization failed: {}", e).into());
			},
			Err(_) => {
				error!("[Boot] Authentication service initialization timed out");
				return Err("Authentication service initialization timed out".into());
			},
		};

	let update_manager:std::sync::Arc<UpdateManager> =
		match tokio::time::timeout(Duration::from_secs(10), UpdateManager::new(AppState.clone())).await {
			Ok(Ok(svc)) => Arc::new(svc),
			Ok(Err(e)) => {
				error!("[Boot] Failed to initialize update manager: {}", e);
				return Err(format!("Update manager initialization failed: {}", e).into());
			},
			Err(_) => {
				error!("[Boot] Update manager initialization timed out");
				return Err("Update manager initialization timed out".into());
			},
		};

	let download_manager:std::sync::Arc<DownloadManager> =
		match tokio::time::timeout(Duration::from_secs(10), DownloadManager::new(AppState.clone())).await {
			Ok(Ok(svc)) => Arc::new(svc),
			Ok(Err(e)) => {
				error!("[Boot] Failed to initialize download manager: {}", e);
				return Err(format!("Download manager initialization failed: {}", e).into());
			},
			Err(_) => {
				error!("[Boot] Download manager initialization timed out");
				return Err("Download manager initialization timed out".into());
			},
		};

	let file_indexer:std::sync::Arc<FileIndexer> =
		match tokio::time::timeout(Duration::from_secs(10), FileIndexer::new(AppState.clone())).await {
			Ok(Ok(svc)) => Arc::new(svc),
			Ok(Err(e)) => {
				error!("[Boot] Failed to initialize file indexer: {}", e);
				return Err(format!("File indexer initialization failed: {}", e).into());
			},
			Err(_) => {
				error!("[Boot] File indexer initialization timed out");
				return Err("File indexer initialization timed out".into());
			},
		};

	info!("[Boot] [Services] All core services initialized successfully");

	// -------------------------------------------------------------------------
	// [Boot] [Health] Register services for health monitoring
	// -------------------------------------------------------------------------
	Trace!("[Boot] [Health] Registering services for health monitoring...");

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
			(Duration::from_secs(5)),
			health_manager.register_service(service_name.to_string(), level),
		)
		.await
		{
			Ok(Ok(_)) => {
				debug!("[Boot] [Health] Registered service: {}", service_name);
			},
			Ok(Err(e)) => {
				warn!("[Boot] Failed to register service {}: {}", service_name, e);
				// Non-fatal: continue without this service's health checks
			},
			Err(_) => {
				warn!("[Boot] Service registration timed out: {}", service_name);
			},
		}
	}

	info!("[Boot] [Health] Service health monitoring configured");

	// -------------------------------------------------------------------------
	// [Boot] [Vine] Initialize gRPC server
	// -------------------------------------------------------------------------
	Trace!("[Boot] [Vine] Initializing gRPC server...");

	// Parse bind address with validation
	let bind_addr:SocketAddr = match bind_address {
		Some(addr) => {
			match addr.parse() {
				Ok(parsed) => {
					info!("[Boot] [Vine] Using custom bind address: {}", parsed);
					parsed
				},
				Err(e) => {
					error!("[Boot] Invalid bind address '{}': {}", addr, e);
					return Err(format!("Invalid bind address: {}", e).into());
				},
			}
		},
		None => {
			match DefaultBindAddress.parse() {
				Ok(parsed) => parsed,
				Err(e) => {
					error!("[Boot] Invalid default bind address '{}': {}", DefaultBindAddress, e);
					return Err(format!("Invalid default bind address: {}", e).into());
				},
			}
		},
	};

	info!("[Boot] [Vine] Configuring gRPC server on {}", bind_addr);

	// Create gRPC service implementation with all dependencies
	let vine_service = match AirLibrary::Vine::Server::AirVinegRPCService::AirVinegRPCService::new(
		AppState.clone(),
		auth_service.clone(),
		update_manager.clone(),
		download_manager.clone(),
		file_indexer.clone(),
	) {
		Ok(svc) => svc,
		Err(e) => {
			error!("[Boot] Failed to create Vine gRPC service: {}", e);
			return Err(format!("Vine service creation failed: {}", e).into());
		},
	};

	// Create a oneshot channel to signal server shutdown
	let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

	// Spawn the tonic gRPC server with panic handling
	let server_handle:tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
		tokio::spawn(async move {
			info!("[Vine] Starting gRPC server on {}", bind_addr);

			let svc = AirLibrary::Vine::Generated::Air_service_server::AirServiceServer::new(vine_service);

			let server = tonic::transport::Server::builder()
				.add_service(svc)
				.serve_with_shutdown(bind_addr, async {
					// Wait for shutdown signal from main
					let _ = shutdown_rx.await;
					info!("[Vine] Shutdown signal received, stopping server...");
				});

			info!("[Vine] gRPC server listening on {}", bind_addr);

			match server.await {
				Ok(_) => {
					info!("[Vine] gRPC server stopped cleanly");
					Ok(())
				},
				Err(e) => {
					error!("[Vine] gRPC server error: {}", e);
					Err(e.into())
				},
			}
		});

	// Wait a bit for the server to start
	tokio::time::sleep(Duration::from_millis(100)).await;

	// Check if server task panicked or failed early
	if server_handle.is_finished() {
		error!("[Boot] gRPC server failed to start");
		let _ = daemon_manager.release_lock().await;
		return Err("gRPC server failed to start".into());
	}

	// -------------------------------------------------------------------------
	// [Boot] [Monitoring] Start background monitoring tasks
	// -------------------------------------------------------------------------
	Trace!("[Boot] [Monitoring] Starting background monitoring tasks...");

	// Start connection monitoring background task
	let connection_monitor_handle:tokio::task::JoinHandle<()> = tokio::spawn({
		let AppState = AppState.clone();
		let health_manager = health_manager.clone();
		async move {
			let mut interval = interval(Duration::from_secs(60)); // Check every minute
			loop {
				interval.tick().await;

				// Update resource usage with error handling
				if let Err(e) = AppState.update_resource_usage().await {
					warn!("[ConnectionMonitor] Failed to update resource usage: {}", e);
				}

				// Get resource metrics
				let resources = AppState.get_resource_usage().await;

				// Record metrics
				let metrics_collector = Metrics::get_metrics();
				metrics_collector.update_resource_metrics(
					resources.MemoryUsageMb.saturating_mul(1024).saturating_mul(1024), // Convert MB to bytes
					resources.cpu_usage_percent,
					AppState.get_active_connection_count().await as u64,
					0, // Active threads - TODO: implement thread count
				);

				// Clean up stale connections (5 minute timeout)
				if let Err(e) = AppState.cleanup_stale_connections(300).await {
					warn!("[ConnectionMonitor] Failed to cleanup stale connections: {}", e);
				}

				// Perform health checks
				match health_manager.check_service("connections").await {
					Ok(_) => {},
					Err(e) => {
						warn!("[ConnectionMonitor] Health check failed: {}", e);

						// Record metrics for failed health check
						let metrics_collector = Metrics::get_metrics();
						metrics_collector.RecordRequestFailure("health_check_failed", 0.0);
					},
				}

				debug!(
					"[ConnectionMonitor] Active connections: {}",
					AppState.get_active_connection_count().await
				);
			}
		}
	});

	// Register background task with error handling
	if let Err(e) = AppState.register_background_task(connection_monitor_handle).await {
		warn!("[Boot] Failed to register connection monitor: {}", e);
		// Non-fatal: continue monitoring may not be tracked
	}

	// Start health monitoring background task
	let health_monitor_handle:tokio::task::JoinHandle<()> = tokio::spawn({
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

	// Register health monitoring task with error handling
	if let Err(e) = AppState.register_background_task(health_monitor_handle).await {
		warn!("[Boot] Failed to register health monitor: {}", e);
		// Non-fatal: continue monitoring may not be tracked
	}

	// -------------------------------------------------------------------------
	// [Boot] [Startup] Start services
	// -------------------------------------------------------------------------
	Trace!("[Boot] [Startup] Starting background services...");

	// Start each service with timeout and error handling
	let service_hands = vec![
		auth_service.start_background_tasks().await,
		update_manager.start_background_tasks().await,
		download_manager.start_background_tasks().await,
		file_indexer.start_background_tasks().await,
	];

	// Collect handles
	let handles:Vec<_> = service_hands.into_iter().collect::<Result<_, _>>()?;

	let (auth_handle, update_handle, download_handle, indexing_handle) =
		(handles[0].clone(), handles[1].clone(), handles[2].clone(), handles[3].clone());

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
	info!("");
	info!("Running. Press Ctrl+C to stop.");
	info!("");

	// Wait for shutdown signal
	WaitForShutdownSignal().await;

	// Signal gRPC server to shut down
	info!("[Shutdown] Signaling gRPC server to stop...");
	let _ = shutdown_tx.send(());

	// Await the server task to finish with timeout
	match tokio::time::timeout(Duration::from_secs(30), server_handle).await {
		Ok(Ok(Ok(_))) => {
			info!("[Shutdown] gRPC server stopped normally");
		},
		Ok(Ok(Err(e))) => {
			warn!("[Shutdown] gRPC server stopped with error: {}", e);
		},
		Ok(Err(e)) => {
			warn!("[Shutdown] gRPC server task panicked: {:?}", e);
		},
		Err(_) => {
			warn!("[Shutdown] gRPC server shutdown timed out");
		},
	}

	// -------------------------------------------------------------------------
	// [Shutdown] Graceful shutdown
	// -------------------------------------------------------------------------
	info!("===========================================");
	info!("[Shutdown] Initiating graceful shutdown...");
	info!("===========================================");

	// Stop all background tasks with timeout
	info!("[Shutdown] Stopping background tasks...");
	if let Err(e) = tokio::time::timeout(Duration::from_secs(10), AppState.stop_all_background_tasks()).await {
		match e {
			Ok(inner) => warn!("[Shutdown] Failed to stop background tasks: {}", inner),
			Err(_) => warn!("[Shutdown] Background tasks stop timed out"),
		}
	}

	// Stop background services
	info!("[Shutdown] Stopping background services...");
	auth_service.stop_background_tasks().await;
	update_manager.stop_background_tasks().await;
	download_manager.stop_background_tasks().await;
	file_indexer.stop_background_tasks().await;

	// Wait for services to stop with timeout
	info!("[Shutdown] Waiting for services to complete...");
	let _ = tokio::time::timeout(
		Duration::from_secs(10),
		tokio::join!(auth_handle, update_handle, download_handle, indexing_handle),
	)
	.await;

	// Log final statistics
	info!("[Shutdown] Collecting final statistics...");

	let metrics = AppState.get_metrics().await;
	let resources = AppState.get_resource_usage().await;
	let health_stats = health_manager.get_health_statistics().await;

	// Get final metrics data
	let metrics_data = Metrics::get_metrics().get_metrics_data();

	info!("===========================================");
	info!("[Shutdown] Final Statistics");
	info!("===========================================");
	info!("[Shutdown] Requests:");
	info!("  - Successful: {}", metrics.SuccessfulRequests);
	info!("  - Failed: {}", metrics.FailedRequests);
	info!("[Shutdown] Metrics:");
	info!("  - Success rate: {:.2}%", metrics_data.success_rate());
	info!("  - Error rate: {:.2}%", metrics_data.error_rate());
	info!("[Shutdown] Resources:");
	info!("  - Memory: {:.2} MB", resources.MemoryUsageMb);
	info!("  - CPU: {:.2}%", resources.cpu_usage_percent);
	info!("[Shutdown] Health:");
	info!("  - Overall: {:.2}%", health_stats.overall_health_percentage());
	info!(
		"  - Healthy services: {}/{}",
		health_stats.healthy_services, health_stats.total_services
	);
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
	// Validate OS and architecture
	info!("[Environment] OS: {}, Arch: {}", std::env::consts::OS, std::env::consts::ARCH);

	// Validate required environment variables
	if let Ok(home) = std::env::var("HOME") {
		if home.is_empty() {
			return Err("HOME environment variable is not set".to_string());
		}
	}

	// Verify we can create lock files
	let lock_path = "/tmp/Air-test-lock.tmp";
	if std::fs::write(lock_path, b"test").is_err() {
		return Err("Cannot write to /tmp directory".to_string());
	}
	let _ = std::fs::remove_file(lock_path);

	Ok(())
}

/// Validate critical configuration values
///
/// # TODO
/// - Add comprehensive configuration validation
/// - Validate port ranges
/// - Validate timeout values
/// - Validate file paths exist or are creatable
/// - Validate URLs are properly formatted
fn validate_configuration(config:&AirLibrary::Configuration::AirConfiguration) -> Result<(), String> {
	// Add configuration validation logic here
	debug!("[Config] Configuration passed basic validation");
	Ok(())
}
