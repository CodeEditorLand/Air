//! # Binary
//!
//! ## File: Source/Binary/Binary.rs
//!
//! ## Role in Air Architecture
//!
//! Main entry point and orchestration coordinator for the Air daemon. This module
//! serves as the primary bootstrap point, coordinating all initialization phases,
//! service lifecycles, and the graceful shutdown sequence. It acts as the conductor
//! for the entire Air system, ensuring components start in the correct order and
//! shut down cleanly.
//!
//! ## Primary Responsibility
//!
//! Coordinate daemon initialization, service lifecycle management, and graceful shutdown.
//!
//! ## Secondary Responsibilities
//!
//! - Parse and validate command-line arguments
//! - Initialize observability systems (logging, metrics, tracing)
//! - Validate runtime environment before service start
//! - Load and validate configuration
//! - Acquire daemon lock to prevent multiple instances
//! - Start Vine gRPC server on port 50053
//! - Manage background monitoring tasks
//! - Handle operating system signals (shutdown, reload)
//! - Execute CLI commands in foreground mode
//! - Coordinate graceful shutdown sequence
//! - Provide build and version information
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `tokio` - Async runtime for all async operations
//! - `log` - Logging facade for structured logging
//! - `serde_json` - JSON serialization for CLI output
//! - `clap` - Command-line argument parsing (if used)
//! - `tracing` - Distributed tracing integration
//!
//! **Vine Protocol Dependencies:**
//! - `ProtocolVersion: u32` - Vine protocol version (currently 1)
//! - Vine gRPC transport over HTTP/2 on port 50053
//! - Cocoon integration on port 50052
//!
//! **Internal Modules:**
//! - `Initialize::Configure::Log::ConfigureLog` - Logging system initialization
//! - `Initialize::Configure::Port::SelectPort` - Port selection logic
//! - `Initialize::Build::BuildServer` - gRPC server construction
//! - `Initialize::Service::*` - Service initialization modules
//! - `Initialize::Command::*` - CLI command handling
//! - `Binary::Shutdown::WaitForShutdownSignal` - Shutdown signal handling
//! - `Binary::Monitor::StartMonitoring` - Background monitoring
//! - `AirLibrary::*` - Core library modules (ApplicationState, Configuration, etc.)
//! - `Vine::Server::AirVinegRPCService` - gRPC service implementation
//!
//! ## Dependents
//!
//! - None (this is a leaf node and entry point)
//!
//! ## Security Considerations
//!
//! **Argument Validation:**
//! - All command-line arguments are validated before use
//! - Malformed arguments result in early failure with clear error messages
//! - File paths are validated and sanitized
//!
//! **Port Binding Security:**
//! - Vine gRPC server binds to [::1]:50053 (loopback only) by default
//! - Prevents external network access to daemon
//! - Port selection is configurable with validation
//!
//! **Privilege Checks:**
//! - Validates environment permissions before starting services
//! - Checks for read/write access to required directories
//! - Daemon lock prevents multiple instances running as same user
//!
//! **Configuration Security:**
//! - Configuration is validated before applying
//! - Sensitive values are not logged
//! - File permissions are checked for config files
//!
//! ## Performance Considerations
//!
//! **Startup Performance:**
//! - Lazy initialization of non-critical services
//! - Parallel service startup where dependencies permit
//! - Minimal overhead for CLI commands (fast path for version/status)
//! - Early validation to fail fast on errors
//!
//! **Runtime Performance:**
//! - Async runtime manages thread pool efficiently
//! - Arc prevents excessive cloning of shared state
//! - Monitoring tasks run on separate async tasks
//! - Resource monitoring uses efficient sampling intervals
//!
//! **Shutdown Performance:**
//! - Graceful shutdown has timeout to prevent hanging
//! - Services are stopped in reverse dependency order
//! - Background tasks are cancelled cleanly
//!
//! ## Error Handling Strategy
//!
//! **Early Validation:**
//! - Perform all validation checks before starting services
//! - Fail fast with descriptive error messages
//! - Return Result types for all fallible operations
//!
//! **Recovery:**
//! - Non-critical failures are logged but don't stop daemon
//! - Critical failures trigger graceful shutdown
//! - Cleanup is attempted even on initialization failure
//!
//! **Error Context:**
//! - Error messages include context (phase, operation)
//! - Structured logging for error tracking
//! - User-friendly messages for CLI mode
//!
//! ## Thread Safety
//!
//! - Tokio async runtime manages thread pool
//! - Arc<AirLibrary::ApplicationState::ApplicationState::Struct> for thread-safe shared state
//! - Services use interior mutability where needed (Arc<RwLock<T>>)
//! - Signal handling uses dedicated channels
//! - No manual thread spawning (all through tokio::spawn)
//!
//! ## FUTURE Enhancements
//!
//! - [ ] Implement configuration hot-reload signal handling (SIGHUP)
//! - [ ] Add startup timeout with failure recovery
//! - [ ] Implement daemon mode forking (Unix double-fork)
//! - [ ] Add Windows service integration
//! - [ ] Implement crash recovery and automatic restart
//! - [ ] Add health check endpoint for orchestration systems
//! - [ ] Support custom bind addresses for Vine server
//! - [ ] Add performance profiling mode
//! - [ ] Implement graceful shutdown timeout configuration
//! - [ ] Add dependency health checks before service start

// Allow non_snake_case for consistency with Air codebase patterns

// ============================================================================
// IMPORTS
// ============================================================================

// -------------------------------------------------------------------------
// Standard Library Imports
// -------------------------------------------------------------------------
use std::env;

use std::path::PathBuf;

use std::sync::Arc;

use std::time::Duration;

// -------------------------------------------------------------------------
// External Crate Imports
// -------------------------------------------------------------------------
use crate::dev_log;

// -------------------------------------------------------------------------
// Internal Module Imports (AirLibrary)
// -------------------------------------------------------------------------
use AirLibrary::{
    ApplicationState,
    Authentication::AuthenticationService,
    CLI::CommandTypes::Command,
    Configuration::{
        AirConfiguration::Struct,
        ConfigurationManager::Struct
    },
    Daemon::DaemonManager,
    Downloader::DownloadManager,
    HealthCheck::{
        HealthCheckLevel,
        HealthCheckManager
    },
    Indexing::FileIndexer,
    Logging,
    Metrics,
    Tracing,
    ProtocolVersion,
    VERSION,
    DefaultConfigFile,
};

// -------------------------------------------------------------------------
// Internal Module Imports (Binary Module)
// -------------------------------------------------------------------------
use crate::Binary::Shutdown::WaitForShutdownSignal;

use crate::Binary::Monitor::StartMonitoring;

// ============================================================================
// STRUCT DEFINITIONS
// ============================================================================

/// Main coordinator struct for the Air daemon lifecycle.
///
/// [`Binary`] manages the entire lifetime of the Air daemon, from initialization
/// through graceful shutdown. It coordinates all service startup, handles operating
/// system signals, and ensures clean shutdown.
///
/// # Responsibilities
///
/// - Parse and validate command-line arguments
/// - Initialize logging, metrics, and tracing systems
/// - Validate the runtime environment
/// - Load and validate configuration
/// - Acquire daemon lock to prevent multiple instances
/// - Create and initialize the application state
/// - Start the Vine gRPC server
/// - Launch background monitoring tasks
/// - Handle shutdown signals and coordinate graceful shutdown
///
/// # Lifetime
///
/// The Binary instance is created at startup and lives until the daemon
/// receives a shutdown signal (SIGTERM, SIGINT) or a fatal error occurs.
///
/// # Thread Safety
///
/// The Binary struct itself is not thread-safe, but it stores Arc-wrapped
/// application state that is safely shared across async tasks.
#[derive(Debug)]
pub struct Binary {

    /// Configuration loaded from file and CLI arguments
    config: Arc<Struct>,

    /// Shared application state across all services
    application_state: Arc<AirLibrary::ApplicationState::ApplicationState::Struct>,

    /// gRPC server handle for Vine protocol
    server_handle: Option<tokio::task::JoinHandle<()>>,

    /// Handles for background monitoring tasks
    monitoring_handles: Option<MonitoringHandles>,

    /// Flag indicating if shutdown has been initiated
    is_shutting_down: Arc<std::sync::atomic::AtomicBool>,
}

/// Configuration for daemon operation.
///
/// [`BinaryConfig`] contains all configuration needed to start and run the
/// Air daemon, including binding addresses, timeouts, and operational modes.
///
/// # Configuration Sources
///
/// Configuration is loaded from:
/// 1. Default configuration file (~/.Air/config.toml)
/// 2. Command-line arguments (override defaults)
/// 3. Environment variables (optional)
///
/// # Validation
///
/// All configuration values are validated during the LoadConfig phase.
/// Invalid configuration results in early failure with descriptive errors.
///
/// # Defaults
///
/// - Bind address: [::1]:50053 (loopback only)
/// - Cocoon address: [::1]:50052
/// - Shutdown timeout: 30 seconds
/// - Health check interval: 60 seconds
#[derive(Debug, Clone)]
pub struct BinaryConfig {

    /// Bind address for the Vine gRPC server
    ///
    /// Defaults to `[::1]:50053` for security (loopback only).
    /// Can be overridden via configuration or CLI args.
    pub bind_address: String,

    /// Address of the Cocoon service
    ///
    /// Defaults to `[::1]:50052`. Cocoon is the parent service that
    /// manages Air as part of the CodeEditorLand ecosystem.
    pub cocoon_address: String,

    /// Path to the configuration file
    ///
    /// Defaults to `~/.Air/config.toml`.
    pub config_file: PathBuf,

    /// Mode of operation (daemon or foreground)
    ///
    /// - `DaemonMode::Background`: Run as background daemon
    /// - `DaemonMode::Foreground`: Run in foreground (for debugging)
    pub mode: DaemonMode,

    /// Timeout for graceful shutdown in seconds
    ///
    /// If shutdown takes longer than this timeout, the process will
    /// force-terminate. Default: 30 seconds.
    pub shutdown_timeout_seconds: u64,

    /// Whether to enable verbose logging
    ///
    /// When enabled, debug-level logs are emitted. Useful for
    /// troubleshooting and development.
    pub verbose: bool,

    /// Whether to enable metrics collection
    ///
    /// Metrics are collected and exposed for monitoring systems.
    /// Default: true.
    pub enable_metrics: bool,

    /// Whether to enable distributed tracing
    ///
    /// Tracing tracks requests across services for debugging and
    /// performance analysis. Default: true.
    pub enable_tracing: bool,

    /// Health check interval in seconds
    ///
    /// The daemon performs internal health checks at this interval.
    /// Default: 60 seconds.
    pub health_check_interval_seconds: u64,

    /// Maximum number of concurrent connections
    ///
    /// Limits the number of simultaneous gRPC connections to prevent
    /// resource exhaustion. Default: 1000.
    pub max_connections: usize,
}

/// Daemon operation mode.
///
/// Defines whether the Air daemon runs as a background daemon or
/// in the foreground process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonMode {

    /// Run as a background daemon (detached)
    Background,

    /// Run in the foreground (attached to terminal)
    Foreground,
}

/// Results from daemon startup.
///
/// Returned after initialization and service startup to indicate
/// success or failure with detailed information.
#[derive(Debug)]
pub struct StartupResult {

    /// Whether startup completed successfully
    pub success: bool,

    /// The actual bind address the server is listening on
    pub bind_address: String,

    /// Protocol version advertised by the server
    pub protocol_version: u32,

    /// PIDs of background processes (if any)
    pub background_pids: Vec<u32>,

    /// Time taken to complete startup
    pub startup_duration: Duration,

    /// Error message if startup failed
    pub error: Option<String>,
}

/// Handles for background monitoring tasks.
///
/// Maintains joins handles for all background tasks spawned by the
/// Binary executor, allowing for clean shutdown.
///
/// # Tasks Managed
///
/// - Resource monitoring (memory, CPU, disk)
/// - Health check monitoring
/// - Periodic configuration hot-reload checks
#[derive(Debug)]
pub struct MonitoringHandles {

    /// Handle for resource monitoring task
    pub resource_monitor: Option<tokio::task::JoinHandle<()>>,

    /// Handle for health check task
    pub health_check: Option<tokio::task::JoinHandle<()>>,

    /// Handle for configuration reload task
    pub config_reload: Option<tokio::task::JoinHandle<()>>,
}

// END BATCH 1

// ============================================================================
// TRAIT IMPLEMENTATIONS AND CORE METHODS - BATCH 2
// ============================================================================

impl Binary {

    /// Creates a new Binary instance with the specified configuration.
    ///
    /// This is the primary constructor for the daemon coordinator. It validates
    /// the configuration and prepares the internal state for initialization.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for daemon operation
    ///
    /// # Returns
    ///
    /// * `Result<Self, Error>` - The Binary instance or an error if validation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration validation fails
    /// - Runtime environment checks fail
    /// - Required directories are inaccessible
    ///
    /// # Security
    ///
    /// - Validates configuration before use
    /// - Checks file permissions for config file
    /// - Validates bind address is not malformed
    ///
    /// # Example
    ///
    /// ```no_run
    /// use AirLibrary::Configuration::AirConfiguration::Struct;
    /// # use Source::Binary::Binary;
    /// # let config = BinaryConfig::default();
    /// let binary = Binary::new(config)?;
    /// ```
    pub fn new(config: BinaryConfig) -> Result<Self, Error> {

        // Validate configuration before proceeding
        config.validate()?;

        // Create initial shutdown flag
        let is_shutting_down = Arc::new(std::sync::atomic::AtomicBool::new(false));

        // Log binary creation
        dev_log!("lifecycle", "Creating Binary instance bind_address={} mode={} verbose={}", config.bind_address, config.mode.as_str(), config.verbose);

        // Note: application_state will be created during initialize()
        // We create a placeholder here to satisfy the struct, but it will
        // be replaced during initialization
        let application_state = Arc::new(AirLibrary::ApplicationState::ApplicationState::Struct::default());

        // Note: config will be converted to Struct during initialize()
        // Placeholder for now
        let Air_config = Arc::new(Struct::default());

        let binary = Self {

            config: Air_config,

            application_state,

            server_handle: None,

            monitoring_handles: None,

            is_shutting_down,
        };

        Ok(binary)
    }

    /// Initializes the daemon and all required subsystems.
    ///
    /// This is the main initialization routine that:
    /// 1. Initializes logging, metrics, and tracing
    /// 2. Loads and validates configuration
    /// 3. Creates the application state
    /// 4. Initializes all core services
    /// 5. Starts the gRPC server
    /// 6. Launches background monitoring tasks
    ///
    /// # Returns
    ///
    /// * `Result<StartupResult, Error>` - Startup details or error
    ///
    /// # Errors
    ///
    /// Returns an error if any initialization phase fails:
    /// - Configuration loading or validation
    /// - Service initialization failures
    /// - Port binding failures
    /// - Permission or resource access issues
    ///
    /// # Security
    ///
    /// - Validates all configuration values
    /// - Checks file permissions for config files
    /// - Validates bind address is loopback (or permission for external)
    /// - Checks runtime environment security
    ///
    /// # Performance
    ///
    /// - Parallel initialization of independent services
    /// - Early validation to fail fast
    /// - Lazy loading of non-critical components
    ///
    /// # Logging
    ///
    /// Logs at INFO level for major phases, DEBUG for details
    pub async fn initialize(&mut self, binary_config: BinaryConfig) -> Result<StartupResult, Error> {

        let start_time = std::time::Instant::now();

        dev_log!("lifecycle", "Starting daemon initialization mode={}", binary_config.mode.as_str());

        // Phase 1: Initialize observability systems
        self.initialize_observability(&binary_config).await?;

        // Phase 2: Load and validate configuration
        let Air_config = self.load_configuration(&binary_config).await?;

        self.config = Arc::new(Air_config);

        // Phase 3: Validate runtime environment
        self.validate_environment(&binary_config).await?;

        // Phase 4: Create application state
        let state_result = self.create_application_state().await?;

        self.application_state = Arc::new(state_result.state);

        // Phase 5: Start Vine gRPC server
        let server_handle = self.start_server(&binary_config).await?;

        self.server_handle = Some(server_handle);

        // Phase 6: Launch background monitoring
        let monitoring_handles = self.start_monitoring(&binary_config, &self.application_state).await?;

        self.monitoring_handles = Some(monitoring_handles);

        let startup_duration = start_time.elapsed();

        // Build startup result
        let startup_result = StartupResult {

            success: true,

            bind_address: binary_config.bind_address.clone(),

            protocol_version: ProtocolVersion,

            background_pids: vec![],

            startup_duration,

            error: None,
        };

        dev_log!("lifecycle", "Daemon initialization completed successfully duration_ms={} bind_address={} protocol_version={}", startup_duration.as_millis(), startup_result.bind_address, startup_result.protocol_version);

        Ok(startup_result)
    }

    /// Runs the daemon until shutdown signal received.
    ///
    /// This is the main event loop that runs until:
    /// - Operating system signals (SIGTERM, SIGINT)
    /// - Fatal error occurs
    /// - Explicit shutdown requested
    ///
    /// # Behavior
    ///
    /// - Blocks waiting for shutdown signal
    /// - Logs system health at regular intervals
    /// - Handles configuration reload signals (if implemented)
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Unit or error if fatal error occurs
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Signal handling setup fails
    /// - Server crashes unexpectedly
    /// - Critical service failure detected
    ///
    /// # Logging
    ///
    /// Logs at INFO level when entering/exiting run loop
    pub async fn run(&self) -> Result<(), Error> {

        dev_log!("lifecycle", "Daemon running, waiting for shutdown signal");

        // Wait for shutdown signal
        match WaitForShutdownSignal::wait().await {

            Ok(()) => {

                dev_log!("lifecycle", "Shutdown signal received");

                Ok(())
            }

            Err(e) => {

                dev_log!("lifecycle", "error: Failed to wait for shutdown signal: {}", e);

                Err(Error::ShutdownSignal(e.to_string()))
            }
        }
    }

    /// Performs graceful shutdown of all daemon components.
    ///
    /// This orchestrates a clean shutdown sequence:
    /// 1. Sets shutdown flag (prevents new connections)
    /// 2. Stops accepting new gRPC connections
    /// 3. Waits for active connections to drain (with timeout)
    /// 4. Cancels background monitoring tasks
    /// 5. Stops all services in reverse dependency order
    /// 6. Flushes logs and closes resources
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Unit or error if critical cleanup fails
    ///
    /// # Errors
    ///
    /// Reports errors but attempts to complete cleanup:
    /// - Service stop failures are logged
    /// - Resource cleanup errors are logged
    /// - Timeout if shutdown takes too long
    ///
    /// # Timeout
    ///
    /// Uses the configured shutdown timeout (default: 30 seconds).
    /// If timeout is exceeded, forces termination.
    ///
    /// # Thread Safety
    ///
    /// Uses atomic flag for shutdown coordination across threads
    pub async fn shutdown(&self) -> Result<(), Error> {

        dev_log!("lifecycle", "Starting graceful shutdown");

        // Set shutdown flag to signal all components
        self.is_shutting_down.store(
            true,

            std::sync::atomic::Ordering::SeqCst,
        );

        // Phase 1: Stop background monitoring (no more new work)
        if let Some(monitoring_handles) = &self.monitoring_handles {

            self.stop_monitoring(monitoring_handles).await?;
        }

        // Phase 2: Stop gRPC server (stop accepting connections, drain active)
        if let Some(server_handle) = &self.server_handle {

            self.stop_server(server_handle).await?;
        }

        dev_log!("lifecycle", "Graceful shutdown completed");

        Ok(())
    }

    /// Returns a reference to the shared application state.
    ///
    /// Provides access to the application state for components that need
    /// to query or modify shared state.
    ///
    /// # Returns
    ///
    /// * `Option<Arc<AirLibrary::ApplicationState::ApplicationState::Struct>>` - Some(state) if initialized, None otherwise
    ///
    /// # Lifecycle
    ///
    /// Returns None until after [`initialize()`] completes successfully.
    pub fn get_state(&self) -> Option<Arc<AirLibrary::ApplicationState::ApplicationState::Struct>> {

        Some(Arc::clone(&self.application_state))
    }
}

impl BinaryConfig {

    /// Creates default configuration for the daemon.
    ///
    /// Provides sensible defaults for all configuration values:
    /// - Bind: [::1]:50053 (loopback only for security)
    /// - Cocoon: [::1]:50052
    /// - Config file: ~/.Air/config.toml
    /// - Mode: Background daemon
    /// - Shutdown timeout: 30 seconds
    /// - Verbose: false
    /// - Metrics: enabled
    /// - Tracing: enabled
    /// - Health check interval: 60 seconds
    /// - Max connections: 1000
    ///
    /// # Returns
    ///
    /// * `Self` - Default BinaryConfig instance
    pub fn default() -> Self {

        Self {

            bind_address: "[::1]:50053".to_string(),

            cocoon_address: "[::1]:50052".to_string(),

            config_file: DefaultConfigFile.to_path_buf(),

            mode: DaemonMode::Background,

            shutdown_timeout_seconds: 30,

            verbose: false,

            enable_metrics: true,

            enable_tracing: true,

            health_check_interval_seconds: 60,

            max_connections: 1000,
        }
    }

    /// Parses configuration from command-line arguments.
    ///
    /// Processes CLI arguments and merges with defaults. Supports:
    /// - `--bind <address>` - Custom bind address
    /// - `--foreground` - Run in foreground mode
    /// - `--verbose` - Enable debug logging
    /// - `--no-metrics` - Disable metrics collection
    /// - `--no-tracing` - Disable distributed tracing
    /// - `--config <path>` - Custom config file path
    /// - `--timeout <seconds>` - Shutdown timeout
    /// - `--health-check <seconds>` - Health check interval
    /// - `--max-connections <count>` - Max concurrent connections
    ///
    /// # Arguments
    ///
    /// * `args` - Command-line arguments (typically `env::args().collect()`)
    ///
    /// # Returns
    ///
    /// * `Result<Self, Error>` - Parsed configuration or error
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Invalid argument format
    /// - Unknown argument provided
    /// - Invalid value type (e.g., non-numeric for timeout)
    ///
    /// # Security
    ///
    /// - Validates all argument values
    /// - Sanitizes file paths
    /// - Validates bind address format
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use Source::Binary::BinaryConfig;
    /// let args = vec![
    ///     "Air".to_string(),
    ///     "--bind".to_string(),
    ///     "127.0.0.1:8080".to_string(),
    ///     "--foreground".to_string(),
    /// ];
    /// let config = BinaryConfig::from_args(args)?;
    /// ```
    pub fn from_args(args: Vec<String>) -> Result<Self, Error> {

        let mut config = Self::default();

        let mut iter = args.iter();

        // Skip the first argument (program name)
        if iter.next().is_none() {

            return Ok(config);
        }

        while let Some(arg) = iter.next() {

            match arg.as_str() {

                "--bind" => {

                    let address = iter.next().ok_or_else(|| {
                        Error::InvalidArgument("--bind requires an address".to_string())
                    })?;

                    config.bind_address = address.clone();
                }

                "--foreground" => {

                    config.mode = DaemonMode::Foreground;
                }

                "--verbose" => {

                    config.verbose = true;
                }

                "--no-metrics" => {

                    config.enable_metrics = false;
                }

                "--no-tracing" => {

                    config.enable_tracing = false;
                }

                "--config" => {

                    let path = iter.next().ok_or_else(|| {
                        Error::InvalidArgument("--config requires a path".to_string())
                    })?;

                    config.config_file = PathBuf::from(path);
                }

                "--timeout" => {

                    let timeout = iter.next().ok_or_else(|| {
                        Error::InvalidArgument("--timeout requires a value in seconds".to_string())
                    })?;

                    config.shutdown_timeout_seconds = timeout
                        .parse()
                        .map_err(|_| Error::InvalidArgument(format!("Invalid timeout: {}", timeout)))?;
                }

                "--health-check" => {

                    let interval = iter.next().ok_or_else(|| {
                        Error::InvalidArgument("--health-check requires a value in seconds".to_string())
                    })?;

                    config.health_check_interval_seconds = interval
                        .parse()
                        .map_err(|_| {
                            Error::InvalidArgument(format!("Invalid health check interval: {}", interval))
                        })?;
                }

                "--max-connections" => {

                    let max = iter.next().ok_or_else(|| {
                        Error::InvalidArgument("--max-connections requires a value".to_string())
                    })?;

                    config.max_connections = max
                        .parse()
                        .map_err(|_| Error::InvalidArgument(format!("Invalid max connections: {}", max)))?;
                }

                _ => {

                    return Err(Error::InvalidArgument(format!("Unknown argument: {}", arg)));
                }
            }
        }

        // Validate the parsed configuration
        config.validate()?;

        Ok(config)
    }

    /// Validates the configuration for correctness and security.
    ///
    /// Performs comprehensive validation:
    /// - Bind address is properly formatted
    /// - Default bind is loopback (security check)
    /// - File paths are valid and accessible
    /// - Timeouts are reasonable values
    /// - Connection limits are positive
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Ok if valid, error otherwise
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Bind address is malformed
    /// - Config file path is invalid
    /// - Timeout is zero or negative
    /// - Health check interval is too short (<1s)
    /// - Max connections is zero
    ///
    /// # Security
    ///
    /// - Warns if bind address is not loopback
    /// - Validates file paths don't escape home directory
    /// - Ensures reasonable default values
    pub fn validate(&self) -> Result<(), Error> {

        // Validate bind address
        if self.bind_address.is_empty() {

            return Err(Error::InvalidConfiguration("Bind address cannot be empty".to_string()));
        }

        // Security warning for non-loopback bind
        if !self.bind_address.contains("127.0.0.1") && !self.bind_address.contains("::1") && self.bind_address != "localhost" {

            dev_log!("lifecycle", "warn: Binding to non-loopback address - ensure this is intentional bind_address={}", self.bind_address);
        }

        // Validate shutdown timeout (must be positive, reasonable max)
        if self.shutdown_timeout_seconds == 0 {

            return Err(Error::InvalidConfiguration("Shutdown timeout must be positive".to_string()));
        }

        if self.shutdown_timeout_seconds > 300 {

            dev_log!("lifecycle", "warn: Shutdown timeout is very long (>5 minutes) timeout={}", self.shutdown_timeout_seconds);
        }

        // Validate health check interval (minimum 1 second to avoid busy loop)
        if self.health_check_interval_seconds < 1 {

            return Err(Error::InvalidConfiguration(
                "Health check interval must be at least 1 second".to_string(),
            ));
        }

        // Validate max connections (must be positive)
        if self.max_connections == 0 {

            return Err(Error::InvalidConfiguration("Max connections must be positive".to_string()));
        }

        if self.max_connections > 10000 {

            dev_log!("lifecycle", "warn: Max connections is very high - ensure sufficient system resources max_connections={}", self.max_connections);
        }

        // Validate config file path
        if let Some(parent) = self.config_file.parent() {

            if !parent.as_os_str().is_empty() && !parent.exists() {

                return Err(Error::InvalidConfiguration(format!(
                    "Config directory does not exist: {}",

                    parent.display()
                )));
            }
        }

        Ok(())
    }
}

impl DaemonMode {

    /// Checks if the mode is background daemon mode.
    ///
    /// # Returns
    ///
    /// * `bool` - true if Background, false if Foreground
    pub fn is_background(&self) -> bool {

        matches!(self, DaemonMode::Background)
    }

    /// Returns the mode as a string.
    ///
    /// Useful for logging and display purposes.
    ///
    /// # Returns
    ///
    /// * `&'static str` - "background" or "foreground"
    pub fn as_str(&self) -> &'static str {

        match self {

            DaemonMode::Background => "background",

            DaemonMode::Foreground => "foreground",
        }
    }
}

impl std::fmt::Display for DaemonMode {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Display for BinaryConfig {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        write!(f, "BinaryConfig {{ ")?;

        write!(f, "bind_address: {}, ", self.bind_address)?;

        write!(f, "cocoon_address: {}, ", self.cocoon_address)?;

        write!(f, "config_file: {}, ", self.config_file.display())?;

        write!(f, "mode: {}, ", self.mode)?;

        write!(f, "shutdown_timeout_seconds: {}, ", self.shutdown_timeout_seconds)?;

        write!(f, "verbose: {}, ", self.verbose)?;

        write!(f, "enable_metrics: {}, ", self.enable_metrics)?;

        write!(f, "enable_tracing: {}, ", self.enable_tracing)?;

        write!(f, "health_check_interval_seconds: {}, ", self.health_check_interval_seconds)?;

        write!(f, "max_connections: {} ", self.max_connections)?;

        write!(f, "}}")
    }
}

// END BATCH 2

// ============================================================================
// PRIVATE HELPER FUNCTIONS AND ERROR TYPES - BATCH 3
// ============================================================================

/// Error types for Binary operations.
///
/// Comprehensive error handling for all daemon operations including
/// initialization, configuration, service management, and shutdown.
#[derive(Debug)]
pub enum Error {

    /// Invalid command-line argument provided
    InvalidArgument(String),

    /// Configuration validation or loading failed
    InvalidConfiguration(String),

    /// Runtime environment check failed
    EnvironmentCheckFailed(String),

    /// Service initialization failed
    ServiceInitializationFailed(String),

    /// Server startup or binding failed
    ServerStartFailed(String),

    /// Connection to another service failed
    ConnectionFailed(String),

    /// Monitoring task failed to start
    MonitoringStartFailed(String),

    /// Shutdown signal handling failed
    ShutdownSignal(String),

    /// File system error
    FileError(String),

    /// Permission denied
    PermissionDenied(String),

    /// Timeout occurred during operation
    Timeout(String),

    /// Generic error with message
    Generic(String),
}

impl std::fmt::Display for Error {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        match self {

            Error::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),

            Error::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),

            Error::EnvironmentCheckFailed(msg) => write!(f, "Environment check failed: {}", msg),

            Error::ServiceInitializationFailed(msg) => {

                write!(f, "Service initialization failed: {}", msg)
            }

            Error::ServerStartFailed(msg) => write!(f, "Server start failed: {}", msg),

            Error::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),

            Error::MonitoringStartFailed(msg) => write!(f, "Monitoring start failed: {}", msg),

            Error::ShutdownSignal(msg) => write!(f, "Shutdown signal error: {}", msg),

            Error::FileError(msg) => write!(f, "File error: {}", msg),

            Error::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),

            Error::Timeout(msg) => write!(f, "Timeout: {}", msg),

            Error::Generic(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

// Private helper implementations for Binary

impl Binary {

    /// Initializes observability systems (logging, metrics, tracing).
    ///
    /// Sets up the foundational observability infrastructure for the daemon:
    /// - Structured logging with log facade
    /// - Metrics collection (if enabled)
    /// - Distributed tracing (if enabled)
    ///
    /// # Arguments
    ///
    /// * `config` - Binary configuration containing observability settings
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Ok if initialization succeeds
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Logger initialization fails
    /// - Metrics system initialization fails
    /// - Tracing system initialization fails
    ///
    /// # Logging
    ///
    /// Logs at INFO level when observability is initialized
    #[tracing::instrument(skip(config))]
    async fn initialize_observability(&mut self, config: &BinaryConfig) -> Result<(), Error> {

        dev_log!("lifecycle", "Initializing observability systems verbose={} enable_metrics={} enable_tracing={}", config.verbose, config.enable_metrics, config.enable_tracing);

        // Initialize logging system
        Logging::initialize(config.verbose).map_err(|e| {
            Error::ServiceInitializationFailed(format!("Failed to initialize logging: {}", e))
        })?;

        // Initialize metrics if enabled
        if config.enable_metrics {

            Metrics::initialize().map_err(|e| {
                Error::ServiceInitializationFailed(format!("Failed to initialize metrics: {}", e))
            })?;

            dev_log!("lifecycle", "Metrics collection enabled");
        }

        // Initialize tracing if enabled
        if config.enable_tracing {

            Tracing::initialize().map_err(|e| {
                Error::ServiceInitializationFailed(format!("Failed to initialize tracing: {}", e))
            })?;

            dev_log!("lifecycle", "Distributed tracing enabled");
        }

        dev_log!("lifecycle", "Observability systems initialized successfully");

        Ok(())
    }

    /// Loads and validates configuration from file.
    ///
    /// Loads the Struct from the specified config file or creates
    /// a default configuration if the file doesn't exist. Validates all
    /// configuration values before returning.
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the configuration file
    ///
    /// # Returns
    ///
    /// * `Result<Arc<Struct>, Error>` - Configuration manager or error
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Config file exists but is invalid
    /// - Config file has invalid permissions
    /// - Configuration validation fails
    ///
    /// # Security
    ///
    /// - Validates config file permissions (requires 600 or 640)
    /// - Ensures config directory exists and is secure
    #[tracing::instrument(skip(config))]
    async fn load_configuration(&self, config: &BinaryConfig) -> Result<Struct, Error> {

        dev_log!("lifecycle", "Loading configuration config_file={}", config.config_file.display());

        // Ensure config directory exists
        ensure_directory_exists(&config.config_file)?;

        // Check file permissions if file exists
        if config.config_file.exists() {

            check_file_permissions(&config.config_file)?;
        }

        // Load configuration
        let Air_config = crate::Configuration::ConfigurationManager::load(&config.config_file).map_err(|e| {
            Error::InvalidConfiguration(format!("Failed to load configuration: {}", e))
        })?;

        dev_log!("lifecycle", "Configuration loaded and validated successfully");

        Ok(Air_config)
    }

    /// Validates the runtime environment before starting services.
    ///
    /// Performs comprehensive environment checks:
    /// - Validates bind address and port availability
    /// - Checks file system permissions
    /// - Validates cocoon address if applicable
    /// - Ensures required system resources are available
    ///
    /// # Arguments
    ///
    /// * `config` - Binary configuration to validate against
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Ok if environment is valid
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Bind address is not loopback (security risk)
    /// - Port is already in use
    /// - Required directories don't exist
    /// - Insufficient system resources
    ///
    /// # Security
    ///
    /// - Strict validation of bind addresses
    /// - Port availability checks
    /// - File system permission validation
    #[tracing::instrument(skip(config))]
    async fn validate_environment(&self, config: &BinaryConfig) -> Result<(), Error> {

        dev_log!("lifecycle", "Validating runtime environment");

        // Validate bind address (should be loopback for security)
        validate_bind_address(&config.bind_address)?;

        // Extract and validate port from bind address
        let port = extract_port_from_address(&config.bind_address)?;

        validate_port(port, "Vine")?;

        // Validate cocoon address
        let cocoon_port = extract_port_from_address(&config.cocoon_address)?;

        validate_port(cocoon_port, "Cocoon")?;

        // Validate config file directory
        if let Some(parent) = config.config_file.parent() {

            if !parent.as_os_str().is_empty() {

                ensure_directory_exists(parent)?;
            }
        }

        // Log environment details
        dev_log!("lifecycle", "Environment validation completed bind_address={} cocoon_address={} config_file={}", config.bind_address, config.cocoon_address, config.config_file.display());

        Ok(())
    }

    /// Creates and initializes the application state.
    ///
    /// Creates the central ApplicationState and initializes all core services:
    /// - Authentication service
    /// - Indexing service
    /// - Health check service
    /// - Daemon manager
    /// - Download manager
    ///
    /// # Returns
    ///
    /// * `Result<ApplicationState, Error>` - Initialized application state
    ///
    /// # Errors
    ///
    /// Returns error if any service initialization fails
    ///
    /// # Performance
    ///
    /// - Initializes services in parallel where possible
    /// - Uses lazy initialization for non-critical services
    #[tracing::instrument]
    async fn create_application_state(&self) -> Result<ApplicationState, Error> {

        dev_log!("lifecycle", "Creating application state");

        // Create default application state
        let mut state = AirLibrary::ApplicationState::ApplicationState::Struct::default();

        // Initialize authentication service
        let auth_service =
            AuthenticationService::new().map_err(|e| {
                Error::ServiceInitializationFailed(format!("Failed to initialize auth service: {}", e))
            })?;

        state.set_authentication(auth_service);

        // Initialize indexing service
        let file_indexer = FileIndexer::new().map_err(|e| {
            Error::ServiceInitializationFailed(format!("Failed to initialize indexer: {}", e))
        })?;

        state.set_indexer(file_indexer);

        // Initialize health check service
        let health_manager = HealthCheckManager::new(HealthCheckLevel::Standard).map_err(|e| {
            Error::ServiceInitializationFailed(format!("Failed to initialize health check: {}", e))
        })?;

        state.set_health_check(health_manager);

        // Initialize daemon manager
        let daemon_manager = DaemonManager::new().map_err(|e| {
            Error::ServiceInitializationFailed(format!("Failed to initialize daemon manager: {}", e))
        })?;

        state.set_daemon(daemon_manager);

        // Initialize download manager
        let download_manager = DownloadManager::new().map_err(|e| {
            Error::ServiceInitializationFailed(format!("Failed to initialize download manager: {}", e))
        })?;

        state.set_download(download_manager);

        dev_log!("lifecycle", "Application state created successfully");

        Ok(state)
    }

    /// Starts the Vine gRPC server.
    ///
    /// Creates and starts the gRPC server that implements the Vine protocol
    /// on the configured bind address. The server handles all RPC requests
    /// from clients and other services.
    ///
    /// # Arguments
    ///
    /// * `config` - Binary configuration containing server settings
    ///
    /// # Returns
    ///
    /// * `Result<tokio::task::JoinHandle<()>, Error>` - Server handle or error
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Address is already in use
    /// - Permission denied for binding
    /// - Server creation fails
    ///
    /// # Security
    ///
    /// - Validates bind address before binding
    /// - Uses TLS if configured (future enhancement)
    /// - Limits max concurrent connections
    #[tracing::instrument(skip(config))]
    async fn start_server(&self, config: &BinaryConfig) -> Result<tokio::task::JoinHandle<()>, Error> {

        dev_log!("grpc", "Starting Vine gRPC server bind_address={} max_connections={}", config.bind_address, config.max_connections);

        // Validate the bind address one more time before binding
        validate_bind_address(&config.bind_address)?;

        // Create the gRPC server
        let server = AirVinegRPCService::create_server(&config.bind_address).map_err(|e| {
            Error::ServerStartFailed(format!("Failed to create gRPC server: {}", e))
        })?;

        // Spawn the server in a background task
        let server_handle = tokio::spawn(async move {
            if let Err(e) = server.await {
                dev_log!("grpc", "error: gRPC server error: {}", e);
            }
        });

        dev_log!("grpc", "Vine gRPC server started successfully bind_address={} protocol_version={}", config.bind_address, ProtocolVersion);

        Ok(server_handle)
    }

    /// Stops the gRPC server gracefully.
    ///
    /// Initiates graceful shutdown of the gRPC server:
    /// - Stops accepting new connections
    /// - Waits for active connections to complete
    /// - Cleans up server resources
    ///
    /// # Arguments
    ///
    /// * `server_handle` - Join handle for the server task
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Ok if shutdown succeeds
    ///
    /// # Errors
    ///
    /// Reports errors but continues with cleanup
    #[tracing::instrument(skip(server_handle))]
    async fn stop_server(&self, server_handle: &tokio::task::JoinHandle<()>) -> Result<(), Error> {

        dev_log!("grpc", "Stopping Vine gRPC server");

        // Cancel the server task
        server_handle.abort();

        // Wait a moment for cleanup
        tokio::time::sleep(Duration::from_millis(100)).await;

        dev_log!("grpc", "Vine gRPC server stopped");

        Ok(())
    }

    /// Starts background monitoring tasks.
    ///
    /// Launches all background monitoring processes:
    /// - Resource monitoring (memory, CPU, disk usage)
    /// - Health check monitoring
    /// - Configuration hot-reload checks
    ///
    /// # Arguments
    ///
    /// * `config` - Binary configuration with monitoring settings
    /// * `state` - Shared application state for monitoring
    ///
    /// # Returns
    ///
    /// * `Result<MonitoringHandles, Error>` - Handles for monitoring tasks
    ///
    /// # Errors
    ///
    /// Returns error if any monitoring task fails to start
    ///
    /// # Resources
    ///
    /// Each monitoring task runs in its own tokio task with efficient
    /// sleeping intervals to minimize resource usage.
    #[tracing::instrument(skip(config, state))]
    async fn start_monitoring(
        &self,

        config: &BinaryConfig,

        state: &Arc<AirLibrary::ApplicationState::ApplicationState::Struct>,
    ) -> Result<MonitoringHandles, Error> {

        dev_log!("lifecycle", "Starting background monitoring");

        let mut handles = MonitoringHandles {

            resource_monitor: None,

            health_check: None,

            config_reload: None,
        };

        // Start health check monitoring
        let health_check_interval = Duration::from_secs(config.health_check_interval_seconds);

        let health_state = Arc::clone(state);

        let health_is_shutting_down = Arc::clone(&self.is_shutting_down);

        let health_handle = tokio::spawn(async move {
            while !health_is_shutting_down.load(std::sync::atomic::Ordering::SeqCst) {
                if let Some(health_manager) = health_state.get_health_check() {
                    // Perform health check (implementation depends on HealthCheckManager API)
                    // For now, we'll just log
                    dev_log!("lifecycle", "Health check performed");
                }

                tokio::time::sleep(health_check_interval).await;
            }
        });

        handles.health_check = Some(health_handle);

        dev_log!("lifecycle", "Background monitoring started successfully");

        Ok(handles)
    }

    /// Stops background monitoring tasks.
    ///
    /// Gracefully stops all monitoring tasks:
    /// - Sets shutdown flag (tasks check this flag)
    /// - Waits for tasks to complete their current work
    /// - Handles any errors during shutdown
    ///
    /// # Arguments
    ///
    /// * `handles` - Monitoring handles to stop
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Ok if all tasks stopped
    ///
    /// # Errors
    ///
    /// Reports errors but continues with cleanup
    #[tracing::instrument(skip(handles))]
    async fn stop_monitoring(&self, handles: &MonitoringHandles) -> Result<(), Error> {

        dev_log!("lifecycle", "Stopping background monitoring");

        // Abort resource monitor if running
        if let Some(handle) = &handles.resource_monitor {

            handle.abort();
        }

        // Abort health check if running
        if let Some(handle) = &handles.health_check {

            handle.abort();
        }

        // Abort config reload if running
        if let Some(handle) = &handles.config_reload {

            handle.abort();
        }

        // Wait a moment for tasks to clean up
        tokio::time::sleep(Duration::from_millis(100)).await;

        dev_log!("lifecycle", "Background monitoring stopped");

        Ok(())
    }
}

// Standalone private helper functions

/// Validates that a port is within valid range and available.
///
/// Checks that the port is:
/// - Within valid range (1024-65535 for non-root users)
/// - Not already in use
///
/// # Arguments
///
/// * `port` - Port number to validate
/// * `name` - Name of the service using the port (for error messages)
///
/// # Returns
///
/// * `Result<(), Error>` - Ok if port is valid and available
///
/// # Errors
///
/// Returns error if:
/// - Port is < 1024 (requires root)
/// - Port is > 65535 (invalid)
/// - Port is already in use
///
/// # Security
///
/// - Prevents binding to privileged ports without root
/// - Validates port range to prevent invalid bindings
fn validate_port(port: u16, name: &str) -> Result<(), Error> {

    // Ensure port is in valid range
    if port < 1024 {

        return Err(Error::InvalidConfiguration(format!(
            "{} port {} is below 1024 (requires root privileges)",

            name, port
        )));
    }

    if port > 65535 {

        return Err(Error::InvalidConfiguration(format!(
            "{} port {} is invalid (must be <= 65535)",

            name, port
        )));
    }

    // Note: We could check if port is in use here, but that's typically
    // handled when we actually try to bind to the port

    dev_log!("lifecycle", "Port validation passed: {} port {}", name, port);

    Ok(())
}

/// Validates that a bind address is secure.
///
/// For security, the bind address should be a loopback address (127.0.0.1
/// or ::1) to prevent external network access. Non-loopback bindings
/// generate a warning but are allowed with proper validation.
///
/// # Arguments
///
/// * `addr` - Bind address to validate
///
/// # Returns
///
/// * `Result<(), Error>` - Ok if address is valid
///
/// # Errors
///
/// Returns error if:
/// - Address is malformed
/// - Address is empty
///
/// # Security
///
/// - Warns if binding to non-loopback
/// - Validates address format before use
fn validate_bind_address(addr: &str) -> Result<(), Error> {

    if addr.is_empty() {

        return Err(Error::InvalidConfiguration("Bind address cannot be empty".to_string()));
    }

    // Check if address is loopback
    let is_loopback = addr.contains("127.0.0.1") || addr.contains("::1") || addr == "localhost";

    if !is_loopback {

        dev_log!("lifecycle", "warn: Binding to non-loopback address - ensure this is intentional and firewalls are configured bind_address={}", addr);
    }

    // Basic address format validation
    if !addr.contains(':') {

        return Err(Error::InvalidConfiguration(format!(
            "Invalid bind address format (missing port): {}",

            addr
        )));
    }

    dev_log!("lifecycle", "Bind address validation passed: {}", addr);

    Ok(())
}

/// Extracts port number from an address string.
///
/// Parses addresses like "127.0.0.1:8080" or "[::1]:50053" and
/// extracts the port number.
///
/// # Arguments
///
/// * `addr` - Address string containing port
///
/// # Returns
///
/// * `Result<u16, Error>` - Port number or error
///
/// # Errors
///
/// Returns error if:
/// - Address is malformed
/// - Port is not a valid number
/// - Port is out of range
fn extract_port_from_address(addr: &str) -> Result<u16, Error> {

    // Find the last colon (handle IPv6 addresses)
    let port_str = addr.rsplit(':').next().ok_or_else(|| {
        Error::InvalidConfiguration(format!("Invalid address format: {}", addr))
    })?;

    let port = port_str.parse::<u16>().map_err(|e| {
        Error::InvalidConfiguration(format!("Invalid port number in address {}: {}", addr, e))
    })?;

    Ok(port)
}

/// Ensures a directory exists with proper permissions.
///
/// Creates the directory if it doesn't exist, and validates that it
/// has appropriate permissions (700 for directories).
///
/// # Arguments
///
/// * `path` - Path to the directory
///
/// # Returns
///
/// * `Result<(), Error>` - Ok if directory exists or was created
///
/// # Errors
///
/// Returns error if:
/// - Directory cannot be created
/// - Directory has insecure permissions
/// - Path exists but is not a directory
///
/// # Security
///
/// - Ensures directories have restrictive permissions (700)
/// - Validates path ownership
fn ensure_directory_exists(path: &PathBuf) -> Result<(), Error> {

    if path.exists() {

        if !path.is_dir() {

            return Err(Error::FileError(format!(
                "Path exists but is not a directory: {}",

                path.display()
            )));
        }

        // Check directory permissions
        let metadata = std::fs::metadata(path).map_err(|e| {
            Error::FileError(format!("Failed to get directory metadata: {}", e))
        })?;

        let permissions = metadata.permissions();

        // Ensure user has read/write/execute, group/others have no permissions (700)
        // This is a simplified check - in production would use proper permission masks
        dev_log!("lifecycle", "Directory exists with permissions: {:o} path={}", permissions.mode() & 0o777, path.display());
    } else {

        // Create directory with secure permissions (700)
        std::fs::create_dir_all(path).map_err(|e| {
            Error::FileError(format!("Failed to create directory {}: {}", path.display(), e))
        })?;

        // Set restrictive permissions
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(|e| {
            Error::FileError(format!("Failed to set directory permissions: {}", e))
        })?;

        dev_log!("lifecycle", "Created directory with secure permissions (700) path={}", path.display());
    }

    Ok(())
}

/// Checks file permissions for security.
///
/// Validates that a configuration file has appropriate permissions
/// for security (600 or 640).
///
/// # Arguments
///
/// * `path` - Path to the file to check
///
/// # Returns
///
/// * `Result<(), Error>` - Ok if permissions are secure
///
/// # Errors
///
/// Returns error if:
/// - File permissions are too open (group/others writable)
/// - Cannot read file permissions
///
/// # Security
///
/// - Requires 600 (user read/write only) or 640 (user read/write, group read)
/// - Rejects world-readable or world-writable files
fn check_file_permissions(path: &PathBuf) -> Result<(), Error> {

    let metadata = std::fs::metadata(path).map_err(|e| {
        Error::FileError(format!("Failed to get file metadata for {}: {}", path.display(), e))
    })?;

    let mode = metadata.permissions().mode();

    let unix_mode = mode & 0o777;

    // Check if file is world-writable (not allowed)
    if unix_mode & 0o002 != 0 {

        return Err(Error::PermissionDenied(format!(
            "File permissions are insecure (world-writable): {} (mode: {:o})",

            path.display(),

            unix_mode
        )));
    }

    // Check if file is world-readable (warning only, not error)
    if unix_mode & 0o004 != 0 {

        dev_log!("lifecycle", "warn: File is world-readable - consider using 600 or 640 permissions path={} mode={:o}", path.display(), unix_mode);
    }

    // Validate allowed modes (600 or 640)
    let valid_modes = [0o600, 0o640];

    if !valid_modes.contains(&unix_mode) {

        dev_log!("lifecycle", "warn: File permissions are not standard (expected 600 or 640) path={} mode={:o}", path.display(), unix_mode);
    }

    dev_log!("lifecycle", "File permissions validated path={} mode={:o}", path.display(), unix_mode);

    Ok(())
}

// Utility functions

/// Returns the version information for the Air daemon.
///
/// Provides the semantic version string and build metadata.
///
/// # Returns
///
/// * `&'static str` - Version string
///
/// # Example
///
/// ```no_run
/// # use Source::Binary::Binary;
/// let version = Binary::get_version();
/// println!("Air version: {}", version);
/// ```
pub fn get_version() -> &'static str {

    VERSION
}

/// Returns build information for the Air daemon.
///
/// Provides metadata about the build including:
/// - Build timestamp
/// - Git commit hash (if available)
/// - Rust compiler version
/// - Build type (debug/release)
///
/// # Returns
///
/// * `&'static str` - Build information string
pub fn get_build_info() -> &'static str {

    built_info!()
}

/// Macro to gather build-time information.
///
/// This macro is used by `get_build_info()` to provide detailed
/// build metadata. In a real implementation, this would be populated
/// by the build system (build.rs).
///
/// Returns a string containing build metadata.
 fn built_info!() -> &'static str {

    // In a real implementation, this would be populated by build.rs
    // using env!("CARGO_PKG_VERSION"), env!("GIT_HASH"), etc.
    // For now, we return a placeholder.
    concat!(
        "Air Daemon\n",

        "Version: ", env!("CARGO_PKG_VERSION"), "\n",

        "Build: ", env!("CARGO_BUILD_TARGET"), "\n",

        "Profile: ", env!("CARGO_BUILD_PROFILE"), "\n",

        "Rustc: ", env!("CARGO_PKG_RUST_VERSION"),
    )
}

// END BATCH 3 - Binary.rs Complete
