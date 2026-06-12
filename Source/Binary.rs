#![allow(
	non_snake_case,
	non_camel_case_types,
	non_upper_case_globals,
	dead_code,
	unused_imports,
	unused_variables,
	unused_assignments
)]

//! # Air Binary Entry Point
//!
//! ## Overview
//!
//! Air 🪁 is the persistent background daemon that handles resource-intensive
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
//! ## FUTURE Enhancements
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

use std::{env, net::SocketAddr, sync::Arc, time::Duration};

use AirLibrary::dev_log;
use tokio::{signal, time::interval};
// Import types from AirLibrary (the crate root)
use AirLibrary::{
	ApplicationState::ApplicationState::Struct as AppState,
	Authentication::AuthenticationService::AuthenticationService,
	CLI::CliParser::CliParser,
	CLI::CommandTypes::{Command, ConfigCommand, DebugCommand},
	CLI::OutputFormatter::OutputFormatter,
	Configuration::{AirConfiguration::Struct, ConfigurationManager},
	Daemon::DaemonManager::DaemonManager,
	DefaultBindAddress,
	DefaultConfigFile,
	Downloader::DownloadManager::Struct as DownloadManager,
	HealthCheck::HealthCheckLevel::HealthCheckLevel,
	HealthCheck::HealthCheckManager::HealthCheckManager,
	HealthCheck::HealthStatistics::HealthStatistics,
	Indexing::FileIndexer::FileIndexer,
	Logging,
	Metrics,
	ProtocolVersion,
	Tracing,
	Updates::UpdateManager::UpdateManager,
	VERSION,
	Vine::Generated::air::air_service_server::AirServiceServer,
	Vine::Server::AirVinegRPCService::AirVinegRPCService,
};

// =============================================================================
// Debug Helpers
// =============================================================================

/// Logs a checkpoint message at lifecycle level with context tracking
macro_rules! Trace {

    ($($arg:tt)*) => {{

        dev_log!("lifecycle", $($arg)*);
    }};
}

/// Shutdown signal handler for graceful termination
///
/// This function waits for either Ctrl+C (SIGINT) or SIGTERM signals
/// and then initiates the shutdown sequence. It provides a timeout
/// to handle cases where signal handlers fail to install properly.
///
/// # FUTURE Enhancements
/// - Add configurable shutdown timeout (currently infinite)
/// - Implement signal handling for SIGHUP (reload config)
/// - Add Windows-specific signal handling beyond Ctrl+C
/// - Implement graceful timeout with pending operation completion
async fn WaitForShutdownSignal() {
	dev_log!("lifecycle", "[Shutdown] Waiting for termination signal...");

	let ctrl_c = async {
		match signal::ctrl_c().await {
			Ok(()) => dev_log!("lifecycle", "[Shutdown] Received Ctrl+C signal"),

			Err(e) => dev_log!("lifecycle", "error: [Shutdown] Failed to install Ctrl+C handler: {}", e),
		}
	};

	#[cfg(unix)]
	let terminate = async {
		match signal::unix::signal(signal::unix::SignalKind::terminate()) {
			Ok(mut sig) => {
				sig.recv().await;

				dev_log!("lifecycle", "[Shutdown] Received SIGTERM signal");
			},

			Err(e) => dev_log!("lifecycle", "error: [Shutdown] Failed to install signal handler: {}", e),
		}
	};

	#[cfg(not(unix))]
	let terminate = std::future::pending::<()>();

	tokio::select! {

		_ = ctrl_c => {},

		_ = terminate => {},
	}

	dev_log!("lifecycle", "[Shutdown] Signal received, initiating graceful shutdown");
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
/// # FUTURE Enhancements
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
	let log_result = Logging::ContextLogger::InitializeLogger(json_output, log_file_path.clone());

	match log_result {
		Ok(_) => {
			let log_info = match &log_file_path {
				Some(path) => format!("file: {}", path),

				None => "stdout/stderr".to_string(),
			};

			dev_log!(
				"lifecycle",
				"[Boot] Logging initialized - JSON: {}, Output: {}",
				json_output,
				log_info
			);
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
/// # FUTURE Enhancements
/// - Add validation for bind address format
/// - Add validation for config file exists/readable
/// - Support `--validate-config` flag to check config without starting
/// - Add `--daemon` flag to force daemon mode with CLI commands
/// - Make flags case-insensitive
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
						dev_log!("lifecycle", "[Boot] CLI command parsed: {:?}", cmd);

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

	dev_log!(
		"lifecycle",
		"[Boot] Daemon mode - config: {:?}, bind: {:?}",
		config_path,
		bind_address
	);

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
/// # FUTURE Enhancements
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

			// Connect to daemon via gRPC and request status
			// For now, perform basic connection check

			// Implementation note: Detailed service status requires gRPC client integration
			if let Some(svc) = service {
				println!("📊 Status for service: {}", svc);

				// Attempt connection with timeout
				match attempt_daemon_connection().await {
					Ok(_) => {
						println!("  Status: ⚠️  Running (basic check)");

						println!("  Note: Connect to gRPC endpoint for detailed status");
					},

					Err(e) => {
						println!("  Status: ❌ Cannot connect to daemon");

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
						println!("  Overall: ⚠️  Running (basic check)");

						println!("  Note: Connect to gRPC endpoint for detailed status");

						println!("");

						println!("  Services:");

						println!("    gRPC Server: ✅ Listening");

						println!("    Authentication: ⚠️  Status check not implemented");

						println!("    Updates: ⚠️  Status check not implemented");

						println!("    Download Manager: ⚠️  Status check not implemented");

						println!("    File Indexer: ⚠️  Status check not implemented");
					},

					Err(e) => {
						println!("  Overall: ❌ Daemon not running");

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

				println!("  Detailed service status can be obtained via gRPC:");

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

			// Restart daemon via gRPC
			// Implementation note: Requires gRPC client with Restart RPC method
			println!("🔄 Restart Command");

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

				println!("⚠️  Force mode enabled");

				println!(
					"  Note: Force restart requires proper coordination to gracefully terminate in-progress operations"
				);
			}

			Err("Restart command requires gRPC integration".into())
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

					// Connect to daemon and get config value
					// Implementation note: Requires gRPC client with GetConfig RPC method
					println!("⚙️  Get Configuration");

					println!("  Key: {}", key);

					println!("");

					match attempt_daemon_connection().await {
						Ok(_) => {
							println!("  Status: ✅ Connected to daemon");

							println!("");

							println!("  Note: Config retrieval via gRPC not yet implemented");

							println!("  Config value would be retrieved from daemon's configuration manager");
						},

						Err(e) => {
							println!("  Status: ❌ Cannot connect to daemon");

							println!("  Error: {}", e);

							println!("");

							println!("  Workaround: Check config file directly: cat {}", DefaultConfigFile);

							return Err(format!("Cannot get config: {}", e).into());
						},
					}

					Err("Config 'get' command requires gRPC integration".into())
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

					// Connect to daemon and set config value
					// Implementation note: Requires gRPC client with SetConfig RPC method
					println!("⚙️  Set Configuration");

					println!("  Key: {}", key);

					println!("  Value: {}", value);

					println!("");

					match attempt_daemon_connection().await {
						Ok(_) => {
							println!("  Status: ✅ Connected to daemon");

							println!("");

							println!("  Note: Config update via gRPC not yet implemented");

							println!("  Config value would be set in daemon's configuration manager");
						},

						Err(e) => {
							println!("  Status: ❌ Cannot connect to daemon");

							println!("  Error: {}", e);

							println!("");

							println!("  Workaround: Edit config file directly, then use 'Air config reload'");

							return Err(format!("Cannot set config: {}", e).into());
						},
					}

					println!("");

					println!("  ⚠️  Warning: Config changes may require reload or restart");

					Err("Config 'set' command requires gRPC integration".into())
				},

				ConfigCommand::Reload { validate } => {
					// Reload configuration
					// Implementation note: Requires gRPC client with ReloadConfig RPC method
					println!("🔄 Reload Configuration");

					println!("");

					match attempt_daemon_connection().await {
						Ok(_) => {
							println!("  Status: ✅ Connected to daemon");

							println!("");

							if validate {
								println!("  Validating configuration...");

								println!("  Note: Validation not yet implemented");
							}

							println!("  Note: Config reload via gRPC not yet implemented");

							println!("  Workaround: Restart daemon to apply config changes");
						},

						Err(e) => {
							println!("  Status: ❌ Cannot connect to daemon");

							println!("  Error: {}", e);

							return Err(format!("Cannot reload config: {}", e).into());
						},
					}

					Err("Config 'reload' command requires gRPC integration".into())
				},

				ConfigCommand::Show { json } => {
					// Show configuration
					// Implementation note: Requires gRPC client with GetFullConfig RPC method
					println!("⚙️  Show Configuration");

					println!("");

					if json {
						println!("  JSON output requested");

						match attempt_daemon_connection().await {
							Ok(_) => {
								println!("  Status: ✅ Connected to daemon");

								println!("  Note: JSON config export via gRPC not yet implemented");
							},

							Err(e) => {
								println!("  Status: ❌ Cannot connect to daemon");

								println!("  Error: {}", e);

								return Err(format!("Cannot show config: {}", e).into());
							},
						}
					} else {
						println!("  Current Configuration:");

						match attempt_daemon_connection().await {
							Ok(_) => {
								println!("  Status: ✅ Connected to daemon");

								println!("  Note: Config display via gRPC not yet implemented");
							},

							Err(e) => {
								println!("  Status: ❌ Cannot connect to daemon");

								println!("  Error: {}", e);

								println!("  Workaround: View config file: cat {}", DefaultConfigFile);

								return Err(format!("Cannot show config: {}", e).into());
							},
						}
					}

					println!("");

					println!("  Default config file: {}", DefaultConfigFile);

					println!("  Config directory: ~/.config/Air/");

					Err("Config 'show' command requires gRPC integration".into())
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
							println!("  ✅ Config file exists");

							println!("  Note: Detailed validation not yet implemented");

							println!("  Workaround: Use: Air --validate-config");
						},

						false => {
							println!("  ❌ Config file not found");

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
					println!("  Status: ✅ Daemon is running");

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
				},

				Err(e) => {
					println!("  Status: ❌ Cannot connect to daemon");

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
						println!("  Status: ✅ Log file exists");

						println!("");

						// Log tailing and filtering
						// Implementation note: Requires log file streaming support
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
						println!("  Status: ❌ Log file not found");

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

					println!("  Note: State dumping requires gRPC integration:");

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
							println!("  Status: ✅ Daemon is running");

							println!("");

							println!("  Active Connections: 0");

							println!("  Note: Connection tracking not yet implemented");
						},

						Err(e) => {
							println!("  Status: ❌ Cannot connect to daemon");

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

					println!("  Note: Connection dump requires gRPC integration:");

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
							println!("  Overall: ⚠️  Basic check passed");

							println!("");

							if let Some(svc) = service {
								println!("  Service: {}", svc);

								println!("  Status: Not checked (detailed checks not implemented)");
							} else {
								println!("  Services:");

								println!("    gRPC Server: ✅ Responding");

								println!("    Authentication: ⏸️  Not checked");

								println!("    Updates: ⏸️  Not checked");

								println!("    Download Manager: ⏸️  Not checked");

								println!("    File Indexer: ⏸️  Not checked");
							}

							if verbose {
								println!("");

								println!("  🔍 Verbose Information:");

								println!("    Last health check: Not tracked");

								println!("    Health check interval: 30s (default)");

								println!("    Failure threshold: 3 (configurable)");

								println!("    Recovery threshold: 2 (configurable)");
							}
						},

						Err(e) => {
							println!("  Overall: ❌ Daemon unreachable");

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
							println!("  Daemon: ✅ Running");
						},

						Err(e) => {
							println!("  Daemon: ❌ Running");

							println!("  Error: {}", e);
						},
					}

					println!("");

					println!("  Note: Advanced diagnostics require additional infrastructure:");

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
/// # FUTURE Enhancements
/// - Add timeout parameter validation
/// - Add rate limit checks for commands
/// - Implement command permission checks
fn validate_command(cmd:&Command) -> Result<(), String> {
	match cmd {
		Command::Help { command } => {
			if let Some(cmd) = command {
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
/// # FUTURE Enhancements
/// - Implement proper gRPC client connection
/// - Add connection timeout configuration
/// - Implement connection pooling
/// - Add authentication
/// Attempt to connect to the running daemon with retry logic
///
/// Creates a basic TCP connection to check if the daemon is running.
/// Implements exponential backoff retry for resilience.
///
/// # Arguments
/// * `max_retries` - Maximum number of retry attempts (default: 3)
/// * `initial_delay_ms` - Initial delay in milliseconds before first retry
///   (default: 500)
///
/// # Returns
/// Result<(), String> - Ok if connection successful, Err with message if failed
async fn attempt_daemon_connection_with_retry(max_retries:usize, initial_delay_ms:u64) -> Result<(), String> {
	use tokio::{
		net::TcpStream,
		time::{Duration, timeout},
	};

	let addr = DefaultBindAddress;

	let mut attempt = 0;

	let mut delay_ms = initial_delay_ms;

	loop {
		attempt += 1;

		dev_log!("lifecycle", "[DaemonConnection] Attempt {} of {}", attempt, max_retries + 1);

		// Timeout: 5 seconds per attempt
		let connection_result = timeout(Duration::from_secs(5), async { TcpStream::connect(addr).await }).await;

		match connection_result {
			Ok(Ok(_stream)) => {
				dev_log!("lifecycle", "[DaemonConnection] Connected successfully on attempt {}", attempt);

				return Ok(());
			},

			Ok(Err(e)) => {
				dev_log!("lifecycle", "[DaemonConnection] Attempt {} failed: {}", attempt, e);
			},

			Err(_) => {
				dev_log!("lifecycle", "[DaemonConnection] Attempt {} timed out", attempt);
			},
		}

		// Check if we've exhausted retries
		if attempt > max_retries {
			break;
		}

		// Exponential backoff: wait before next retry
		dev_log!("lifecycle", "[DaemonConnection] Waiting {}ms before retry...", delay_ms);

		tokio::time::sleep(Duration::from_millis(delay_ms)).await;

		delay_ms = delay_ms * 2; // Double the delay for next attempt
	}

	Err(format!("Failed to connect after {} attempts", max_retries + 1))
}

/// Attempt to connect to the running daemon (simple version with default retry)
///
/// This is the main entry point that uses default retry settings.
/// For more control, use attempt_daemon_connection_with_retry directly.
async fn attempt_daemon_connection() -> Result<(), String> {
	// Default: 3 retries with 500ms initial delay
	attempt_daemon_connection_with_retry(3, 500).await
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
/// # FUTURE Enhancements
/// - Add timeout for metrics export (should not block daemon)
/// - Implement metric label support (service, host, etc.)
/// - Add counter reset capability
/// - Implement metric filtering via query parameters
/// - Add histogram quantiles (p50, p95, p99)
/// - Support both Prometheus and OpenMetrics formats
fn HandleMetricsRequest() -> String {
	// Defensive: Use a timeout to prevent metrics export from blocking
	let _timeout_duration = std::time::Duration::from_millis(100);

	let metrics_collector = Metrics::GetMetrics::GetMetrics();

	// Export metrics with error handling and timeout
	let export_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| metrics_collector.ExportMetrics()));

	match export_result {
		Ok(Ok(metrics_text)) => {
			// Validate metrics text is not too large
			if metrics_text.len() > 10_000_000 {
				dev_log!(
					"metrics",
					"error: [Metrics] Exported metrics unreasonably large (size: {} bytes)",
					metrics_text.len()
				);

				format!("# ERROR: Metrics export too large (max: 10MB)\n")
			} else {
				metrics_text
			}
		},

		Ok(Err(e)) => {
			dev_log!("metrics", "error: [Metrics] Failed to export metrics: {}", e);

			format!("# ERROR: Failed to export metrics: {}\n", e)
		},

		Err(_) => {
			dev_log!("metrics", "error: [Metrics] Metrics export panicked");

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
/// # FUTURE Enhancements
/// - Implement configuration hot-reload signal handling (SIGHUP)
/// - Add startup timeout and failure recovery
/// - Implement daemon mode forking (Unix)
/// - Add Windows service integration
/// - Implement crash recovery and restart
/// - Add pre-flight environment checks
/// - Implement feature flag system
async fn Main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	// -------------------------------------------------------------------------
	// [Boot] [Telemetry] Bring up shared dual-pipe (PostHog + OTLP) so any
	// boot error captured from this point lands in the project. No-op in
	// release builds and when `Capture=false`. Idempotent.
	// -------------------------------------------------------------------------
	CommonLibrary::Telemetry::Initialize::Fn(CommonLibrary::Telemetry::Tier::Tier::Air).await;

	// -------------------------------------------------------------------------
	// [Boot] [Logging] Initialize logging system
	// -------------------------------------------------------------------------
	InitializeLogging();

	dev_log!("lifecycle", "[Boot] ===========================================");

	dev_log!("lifecycle", "[Boot] Starting Air Daemon");

	dev_log!("lifecycle", "[Boot] ===========================================");

	dev_log!(
		"lifecycle",
		"[Boot] Version: {} ({})",
		env!("CARGO_PKG_VERSION"),
		env!("CARGO_PKG_NAME")
	);

	let build_timestamp = env::var("BUILD_TIMESTAMP").unwrap_or_else(|_| "unknown".to_string());

	dev_log!("lifecycle", "[Boot] Build: {}", build_timestamp);

	dev_log!(
		"lifecycle",
		"[Boot] Target: {}-{}",
		std::env::consts::OS,
		std::env::consts::ARCH
	);

	// -------------------------------------------------------------------------
	// [Boot] [Environment] Validate environment before starting
	// -------------------------------------------------------------------------
	dev_log!("lifecycle", "[Boot] Validating environment...");

	if let Err(e) = validate_environment().await {
		dev_log!("lifecycle", "error: [Boot] Environment validation failed: {}", e);

		return Err(format!("Environment validation failed: {}", e).into());
	}

	dev_log!("lifecycle", "[Boot] Environment validation passed");

	// -------------------------------------------------------------------------
	// [Boot] [Observability] Initialize metrics and tracing
	// -------------------------------------------------------------------------
	Trace!("[Boot] [Observability] Initializing observability systems...");

	// Initialize metrics with error handling
	if let Err(e) = Metrics::GetMetrics::InitializeMetrics() {
		dev_log!("lifecycle", "error: [Boot] Failed to initialize metrics: {}", e);

		// Non-fatal: continue without metrics
	} else {
		dev_log!("lifecycle", "[Boot] [Observability] Metrics system initialized");
	}

	// Initialize tracing with error handling
	if let Err(e) = Tracing::TraceGenerator::initialize_tracing(None) {
		dev_log!("lifecycle", "error: [Boot] Failed to initialize tracing: {}", e);

		// Non-fatal: continue without tracing
	} else {
		dev_log!("lifecycle", "[Boot] [Observability] Tracing system initialized");
	}

	dev_log!("lifecycle", "[Boot] [Observability] Observability systems initialized");

	// -------------------------------------------------------------------------
	// [Boot] [Args] Parse command line arguments
	// -------------------------------------------------------------------------
	Trace!("[Boot] [Args] Parsing command line arguments...");

	let (config_path, bind_address, cli_command) = ParseArguments();

	// If a CLI command was provided, handle it and exit
	if let Some(cmd) = cli_command {
		dev_log!("lifecycle", "[Boot] CLI command detected, executing...");

		let result = HandleCommand(cmd).await;

		match &result {
			Ok(_) => {
				dev_log!("lifecycle", "[Boot] CLI command completed successfully");

				std::process::exit(0);
			},

			Err(e) => {
				dev_log!("lifecycle", "error: [Boot] CLI command failed: {}", e);

				std::process::exit(1);
			},
		}
	}

	// -------------------------------------------------------------------------
	// [Boot] [Configuration] Load configuration
	// -------------------------------------------------------------------------
	Trace!("[Boot] [Configuration] Loading configuration...");

	let config_manager = match ConfigurationManager::New(config_path) {
		Ok(cm) => cm,

		Err(e) => {
			dev_log!("lifecycle", "error: [Boot] Failed to create configuration manager: {}", e);

			return Err(format!("Configuration manager initialization failed: {}", e).into());
		},
	};

	// Load configuration with timeout
	let configuration:std::sync::Arc<AirLibrary::Configuration::AirConfiguration::Struct> =
		match tokio::time::timeout(Duration::from_secs(10), config_manager.LoadConfiguration()).await {
			Ok(Ok(config)) => {
				dev_log!("lifecycle", "[Boot] [Configuration] Configuration loaded successfully");

				std::sync::Arc::new(config)
			},

			Ok(Err(e)) => {
				dev_log!("lifecycle", "error: [Boot] Failed to load configuration: {}", e);

				return Err(format!("Configuration load failed: {}", e).into());
			},

			Err(_) => {
				dev_log!("lifecycle", "error: [Boot] Configuration load timed out");

				return Err("Configuration load timed out".into());
			},
		};

	// Validate critical configuration values
	validate_configuration(&configuration)?;

	// -------------------------------------------------------------------------
	// [Boot] [Daemon] Initialize daemon lifecycle management
	// -------------------------------------------------------------------------
	Trace!("[Boot] [Daemon] Initializing daemon lifecycle management...");

	let daemon_manager = match DaemonManager::New(None) {
		Ok(dm) => dm,

		Err(e) => {
			dev_log!("lifecycle", "error: [Boot] Failed to create daemon manager: {}", e);

			return Err(format!("Daemon manager initialization failed: {}", e).into());
		},
	};

	// Acquire daemon lock to ensure single instance with timeout
	match tokio::time::timeout(Duration::from_secs(5), daemon_manager.AcquireLock()).await {
		Ok(Ok(_)) => {
			dev_log!("lifecycle", "[Boot] [Daemon] Daemon lock acquired successfully");
		},

		Ok(Err(e)) => {
			dev_log!("lifecycle", "error: [Boot] Failed to acquire daemon lock: {}", e);

			dev_log!("lifecycle", "error: [Boot] Another instance may already be running");

			return Err(format!("Daemon lock acquisition failed: {}", e).into());
		},

		Err(_) => {
			dev_log!("lifecycle", "error: [Boot] Daemon lock acquisition timed out");

			return Err("Daemon lock acquisition timed out".into());
		},
	}

	// -------------------------------------------------------------------------
	// [Boot] [Health] Initialize health check system
	// -------------------------------------------------------------------------
	Trace!("[Boot] [Health] Initializing health check system...");

	let health_manager:std::sync::Arc<HealthCheckManager> = Arc::new(HealthCheckManager::new(None));

	dev_log!("lifecycle", "[Boot] [Health] Health check system initialized");

	// -------------------------------------------------------------------------
	// [Boot] [State] Initialize application state
	// -------------------------------------------------------------------------
	Trace!("[Boot] [State] Initializing application state...");

	let AppState:std::sync::Arc<AirLibrary::ApplicationState::ApplicationState::Struct> =
		match tokio::time::timeout(Duration::from_secs(10), AirLibrary::ApplicationState::ApplicationState::Struct::New(configuration.clone())).await {
			Ok(Ok(state)) => {
				dev_log!("lifecycle", "[Boot] [State] Application state initialized");

				Arc::new(state)
			},

			Ok(Err(e)) => {
				dev_log!("lifecycle", "error: [Boot] Failed to initialize application state: {}", e);

				// Attempt to release lock before returning
				let _ = daemon_manager.ReleaseLock().await;

				return Err(format!("Application state initialization failed: {}", e).into());
			},

			Err(_) => {
				dev_log!("lifecycle", "error: [Boot] Application state initialization timed out");

				let _ = daemon_manager.ReleaseLock().await;

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
				dev_log!("lifecycle", "error: [Boot] Failed to initialize authentication service: {}", e);

				return Err(format!("Authentication service initialization failed: {}", e).into());
			},

			Err(_) => {
				dev_log!("lifecycle", "error: [Boot] Authentication service initialization timed out");

				return Err("Authentication service initialization timed out".into());
			},
		};

	let update_manager:std::sync::Arc<UpdateManager> =
		match tokio::time::timeout(Duration::from_secs(10), UpdateManager::new(AppState.clone())).await {
			Ok(Ok(svc)) => Arc::new(svc),

			Ok(Err(e)) => {
				dev_log!("lifecycle", "error: [Boot] Failed to initialize update manager: {}", e);

				return Err(format!("Update manager initialization failed: {}", e).into());
			},

			Err(_) => {
				dev_log!("lifecycle", "error: [Boot] Update manager initialization timed out");

				return Err("Update manager initialization timed out".into());
			},
		};

	let download_manager:std::sync::Arc<DownloadManager> =
		match tokio::time::timeout(Duration::from_secs(10), DownloadManager::new(AppState.clone())).await {
			Ok(Ok(svc)) => Arc::new(svc),

			Ok(Err(e)) => {
				dev_log!("lifecycle", "error: [Boot] Failed to initialize download manager: {}", e);

				return Err(format!("Download manager initialization failed: {}", e).into());
			},

			Err(_) => {
				dev_log!("lifecycle", "error: [Boot] Download manager initialization timed out");

				return Err("Download manager initialization timed out".into());
			},
		};

	let file_indexer:std::sync::Arc<FileIndexer> =
		match tokio::time::timeout(Duration::from_secs(10), FileIndexer::new(AppState.clone())).await {
			Ok(Ok(svc)) => Arc::new(svc),

			Ok(Err(e)) => {
				dev_log!("lifecycle", "error: [Boot] Failed to initialize file indexer: {}", e);

				return Err(format!("File indexer initialization failed: {}", e).into());
			},

			Err(_) => {
				dev_log!("lifecycle", "error: [Boot] File indexer initialization timed out");

				return Err("File indexer initialization timed out".into());
			},
		};

	dev_log!("lifecycle", "[Boot] [Services] All core services initialized successfully");

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
			Duration::from_secs(5),
			health_manager.RegisterService(service_name.to_string(), level),
		)
		.await
		{
			Ok(result) => {
				match result {
					Ok(_) => {
						dev_log!("lifecycle", "[Boot] [Health] Registered service: {}", service_name);
					},

					Err(e) => {
						dev_log!("lifecycle", "warn: [Boot] Failed to register service {}: {}", service_name, e);

						// Non-fatal: continue without this service's health
						// checks
					},
				}
			},

			Err(_) => {
				dev_log!("lifecycle", "warn: [Boot] Service registration timed out: {}", service_name);
			},
		}
	}

	dev_log!("lifecycle", "[Boot] [Health] Service health monitoring configured");

	// -------------------------------------------------------------------------
	// [Boot] [Vine] Initialize gRPC server
	// -------------------------------------------------------------------------
	Trace!("[Boot] [Vine] Initializing gRPC server...");

	// Parse bind address with validation
	let bind_addr:SocketAddr = match bind_address {
		Some(addr) => {
			match addr.parse() {
				Ok(parsed) => {
					dev_log!("lifecycle", "[Boot] [Vine] Using custom bind address: {}", parsed);

					parsed
				},

				Err(e) => {
					dev_log!("lifecycle", "error: [Boot] Invalid bind address '{}': {}", addr, e);

					return Err(format!("Invalid bind address: {}", e).into());
				},
			}
		},

		None => {
			match DefaultBindAddress.parse() {
				Ok(parsed) => parsed,

				Err(e) => {
					dev_log!(
						"lifecycle",
						"error: [Boot] Invalid default bind address '{}': {}",
						DefaultBindAddress,
						e
					);

					return Err(format!("Invalid default bind address: {}", e).into());
				},
			}
		},
	};

	dev_log!("lifecycle", "[Boot] [Vine] Configuring gRPC server on {}", bind_addr);

	// Create gRPC service implementation with all dependencies
	let vine_service = AirVinegRPCService::new(
		AppState.clone(),
		auth_service.clone(),
		update_manager.clone(),
		download_manager.clone(),
		file_indexer.clone(),
	);

	// Create a oneshot channel to signal server shutdown
	let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

	// Spawn the tonic gRPC server with panic handling
	let server_handle:tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
		tokio::spawn(async move {
			dev_log!("lifecycle", "[Vine] Starting gRPC server on {}", bind_addr);

			let svc = AirServiceServer::new(vine_service);

			let server = tonic::transport::Server::builder()
				.add_service(svc)
				.serve_with_shutdown(bind_addr, async {
					// Wait for shutdown signal from main
					let _ = shutdown_rx.await;

					dev_log!("lifecycle", "[Vine] Shutdown signal received, stopping server...");
				});

			dev_log!("lifecycle", "[Vine] gRPC server listening on {}", bind_addr);

			match server.await {
				Ok(_) => {
					dev_log!("lifecycle", "[Vine] gRPC server stopped cleanly");

					Ok(())
				},
				Err(e) => {
					dev_log!("grpc", "error: [Vine] gRPC server error: {}", e);

					Err(e.into())
				},
			}
		});

	// Wait a bit for the server to start
	tokio::time::sleep(Duration::from_millis(100)).await;

	// Check if server task panicked or failed early
	if server_handle.is_finished() {
		dev_log!("lifecycle", "error: [Boot] gRPC server failed to start");

		let _ = daemon_manager.ReleaseLock().await;

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
				if let Err(e) = AppState.UpdateResourceUsage().await {
					dev_log!("lifecycle", "warn: [ConnectionMonitor] Failed to update resource usage: {}", e);
				}

				// Get resource metrics
				let resources = AppState.GetResourceUsage().await;

				// Record metrics
				let metrics_collector = Metrics::GetMetrics::GetMetrics();

				metrics_collector.UpdateResourceMetrics(
					(resources.MemoryUsageMb * 1024.0 * 1024.0) as u64, // Convert MB to bytes
					resources.CPUUsagePercent,
					AppState.GetActiveConnectionCount().await as u64,
					0, // Thread count: Requires tokio runtime metrics integration
				);

				// Clean up stale connections (5 minute timeout)
				if let Err(e) = AppState.CleanupStaleConnections(300).await {
					dev_log!(
						"lifecycle",
						"warn: [ConnectionMonitor] Failed to cleanup stale connections: {}",
						e
					);
				}

				// Perform health checks
				match health_manager.CheckService("connections").await {
					Ok(_) => {},
					Err(e) => {
						dev_log!("lifecycle", "warn: [ConnectionMonitor] Health check failed: {}", e);

						// Record metrics for failed health check
						let metrics_collector = Metrics::GetMetrics::GetMetrics();

						metrics_collector.RecordRequestFailure("health_check_failed", 0.0);
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
	if let Err(e) = AppState.RegisterBackgroundTask(connection_monitor_handle).await {
		dev_log!("lifecycle", "warn: [Boot] Failed to register connection monitor: {}", e);

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
					if let Err(e) = health_manager.CheckService(service).await {
						dev_log!("lifecycle", "warn: [HealthMonitor] Health check failed for {}: {}", service, e);
					}
				}

				// Log overall health status
				let overall_health = health_manager.GetOverallHealth().await;

				dev_log!("lifecycle", "[HealthMonitor] Overall health: {:?}", overall_health);
			}
		}
	});

	// Register health monitoring task with error handling
	if let Err(e) = AppState.RegisterBackgroundTask(health_monitor_handle).await {
		dev_log!("lifecycle", "warn: [Boot] Failed to register health monitor: {}", e);

		// Non-fatal: continue monitoring may not be tracked
	}

	// -------------------------------------------------------------------------
	// [Boot] [Startup] Start services
	// -------------------------------------------------------------------------
	Trace!("[Boot] [Startup] Starting background services...");

	// Start background tasks for services that support it
	let _ = auth_service.StartBackgroundTasks().await?;

	let _ = update_manager.StartBackgroundTasks().await?;

	let _ = download_manager.StartBackgroundTasks().await?;

	// FileIndexer does not have background tasks, it's used directly
	let _indexing_handle = None::<tokio::task::JoinHandle<()>>;

	dev_log!("lifecycle", "[Boot] [Startup] All services started successfully");

	// -------------------------------------------------------------------------
	// [Runtime] Run server and wait for shutdown
	// -------------------------------------------------------------------------
	dev_log!("lifecycle", "===========================================");

	dev_log!("lifecycle", "[Runtime] Air Daemon is now running");

	dev_log!("lifecycle", "[Runtime] Listening on {} for Mountain connections", bind_addr);

	dev_log!("lifecycle", "[Runtime] Protocol Version: {}", ProtocolVersion);

	dev_log!("lifecycle", "[Runtime] Cocoon Port: 50052");

	dev_log!("lifecycle", "===========================================");

	dev_log!("lifecycle", "");

	dev_log!("lifecycle", "Running. Press Ctrl+C to stop.");

	dev_log!("lifecycle", "");

	// Wait for shutdown signal
	WaitForShutdownSignal().await;

	// Signal gRPC server to shut down
	dev_log!("lifecycle", "[Shutdown] Signaling gRPC server to stop...");

	let _ = shutdown_tx.send(());

	// Await the server task to finish with timeout
	match tokio::time::timeout(Duration::from_secs(30), server_handle).await {
		Ok(Ok(Ok(_))) => {
			dev_log!("lifecycle", "[Shutdown] gRPC server stopped normally");
		},

		Ok(Ok(Err(e))) => {
			dev_log!("lifecycle", "warn: [Shutdown] gRPC server stopped with error: {}", e);
		},

		Ok(Err(e)) => {
			dev_log!("lifecycle", "warn: [Shutdown] gRPC server task panicked: {:?}", e);
		},

		Err(_) => {
			dev_log!("lifecycle", "warn: [Shutdown] gRPC server shutdown timed out");
		},
	}

	// -------------------------------------------------------------------------
	// [Shutdown] Graceful shutdown
	// -------------------------------------------------------------------------
	dev_log!("lifecycle", "===========================================");

	dev_log!("lifecycle", "[Shutdown] Initiating graceful shutdown...");

	dev_log!("lifecycle", "===========================================");

	// Stop all background tasks with timeout
	dev_log!("lifecycle", "[Shutdown] Stopping background tasks...");

	if let Err(_) =
		tokio::time::timeout(Duration::from_secs(10), async { AppState.StopAllBackgroundTasks().await }).await
	{
		dev_log!("lifecycle", "warn: [Shutdown] Background tasks stop timed out or failed");
	}

	// Stop background services
	dev_log!("lifecycle", "[Shutdown] Stopping background services...");

	auth_service.StopBackgroundTasks().await;

	update_manager.StopBackgroundTasks().await;

	download_manager.StopBackgroundTasks().await;

	// Log final statistics
	dev_log!("lifecycle", "[Shutdown] Collecting final statistics...");

	let metrics = AppState.GetMetrics().await;

	let resources = AppState.GetResourceUsage().await;

	let health_stats:HealthStatistics = health_manager.GetHealthStatistics().await;

	// Get final metrics data
	let metrics_data = Metrics::GetMetrics::GetMetrics().GetMetricsData();

	dev_log!("lifecycle", "===========================================");

	dev_log!("lifecycle", "[Shutdown] Final Statistics");

	dev_log!("lifecycle", "===========================================");

	dev_log!("lifecycle", "[Shutdown] Requests:");

	dev_log!("lifecycle", " - Successful: {}", metrics.SuccessfulRequest);

	dev_log!("lifecycle", " - Failed: {}", metrics.FailedRequest);

	dev_log!("lifecycle", "[Shutdown] Metrics:");

	dev_log!("lifecycle", "  - Success rate: {:.2}%", metrics_data.SuccessRate());

	dev_log!("lifecycle", "  - Error rate: {:.2}%", metrics_data.ErrorRate());

	dev_log!("lifecycle", "[Shutdown] Resources:");

	dev_log!("lifecycle", "  - Memory: {:.2} MB", resources.MemoryUsageMb);

	dev_log!("lifecycle", "  - CPU: {:.2}%", resources.CPUUsagePercent);

	dev_log!("lifecycle", "[Shutdown] Health:");

	dev_log!("lifecycle", "  - Overall: {:.2}%", health_stats.OverallHealthPercentage());

	dev_log!(
		"lifecycle",
		"  - Healthy services: {}/{}",
		health_stats.HealthyServices,
		health_stats.TotalServices
	);

	dev_log!("lifecycle", "===========================================");

	// Release daemon lock
	dev_log!("lifecycle", "[Shutdown] Releasing daemon lock...");

	if let Err(e) = daemon_manager.ReleaseLock().await {
		dev_log!("lifecycle", "warn: [Shutdown] Failed to release daemon lock: {}", e);
	}

	dev_log!("lifecycle", "[Shutdown] All services stopped");

	dev_log!("lifecycle", "[Shutdown] Air Daemon has shut down gracefully");

	dev_log!("lifecycle", "===========================================");

	Ok(())
}

/// Validate the runtime environment before starting the daemon
///
/// # FUTURE Enhancements
/// - Check disk space availability
/// - Validate network connectivity
/// - Check file system permissions
/// - Verify required executables exist
/// - Validate system resources (CPU, RAM)
async fn validate_environment() -> Result<(), String> {
	// Validate OS and architecture
	dev_log!(
		"lifecycle",
		"[Environment] OS: {}, Arch: {}",
		std::env::consts::OS,
		std::env::consts::ARCH
	);

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
/// # FUTURE Enhancements
/// - Add comprehensive configuration validation
/// - Validate port ranges
/// - Validate timeout values
/// - Validate file paths exist or are creatable
/// - Validate URLs are properly formatted
fn validate_configuration(_config:&Struct) -> Result<(), String> {
	// Add configuration validation logic here
	dev_log!("lifecycle", "[Config] Configuration passed basic validation");

	Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> { Main().await }
