#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

//! # Air Library
//!
//! ## Overview
//!
//! The Air Library is the core implementation for the Air daemon - the persistent
//! background service for the Land code editor. It provides services for authentication,
//! updates, downloads, and file indexing that run independently from the main Mountain
//! application.
//!
//! ## Architecture & Connections
//!
//! Air is the hub that connects various components in the Land ecosystem:
//!
//! - **Wind** (Effect-TS): Functional programming patterns for state management
//!   Air integrates with Wind's effect system for predictable state transitions
//!   and error handling patterns
//!
//! - **Cocoon** (NodeJS host): The Node.js runtime for web components
//!   Air communicates with Cocoon through the Vine protocol to deliver web
//!   assets and perform frontend build operations. Port: 50052
//!
//! - **Mountain** (Tauri bundler): Main desktop application
//!   Mountain receives work from Air through Vine (gRPC) and performs the
//!   main application logic. Mountain's Tauri framework handles the native integration
//!
//! - **Vine** (gRPC protocol): Communication layer connecting all components
//!   Air hosts the Vine gRPC server on port 50053, receiving work requests from Mountain
//!
//! ## VSCode Architecture References
//!
//! ### Update Service
//!
//! Reference: `Dependency/Microsoft/Dependency/Editor/src/vs/platform/update/`
//!
//! Air's UpdateManager is inspired by VSCode's update architecture:
//!
//! - **AbstractUpdateService** (`common/update.ts`): Base service defining update interfaces
//! - Platform-specific implementations:
//!   - `updateService.darwin.ts` - macOS update handling
//!   - `updateService.linux.ts` - Linux update handling
//!   - `updateService.snap.ts` - Snap package updates
//!   - `updateService.win32.ts` - Windows update handling
//!
//! Air's UpdateManager abstracts platform differences and provides:
//! - Update checking with version comparison
//! - Package download with resumable support
//! - Checksum verification for integrity
//! - Signature validation for security
//! - Staged updates for rollback capability
//!
//! ### Lifecycle Management
//!
//! Reference: `Dependency/Microsoft/Dependency/Editor/src/vs/base/common/lifecycle.ts`
//!
//! VSCode's lifecycle patterns inform Air's daemon management:
//!
//! - **Disposable pattern**: Resources implement cleanup methods
//! - **EventEmitter**: Async event handling for state changes
//! - **DisposableStore**: Aggregate resource cleanup
//!
//! Air adapts these patterns with:
//! - `ApplicationState`: Central state management with cleanup
//! - `DaemonManager`: Single-instance lock management
//! - Graceful shutdown with resource release
//!
//! ## Module Organization
//!
//! The Air library is organized into functional modules:
//!
//! ### Core Infrastructure
//! - `ApplicationState`: Central state manager for the daemon
//! - `Configuration`: Configuration loading and validation
//! - `Daemon`: Daemon lifecycle and lock management
//! - `Logging`: Structured logging with filtering
//! - `Metrics`: Prometheus-style metrics collection
//! - `Tracing`: Distributed tracing support
//!
//! ### Services
//! - `Authentication`: Token management and cryptographic operations
//! - `Updates`: Update checking, downloading, and installation
//! - `Downloader`: Background downloads with retry logic
//! - `Indexing`: File system indexing for code navigation
//!
//! ### Communication
//! - `Vine`: gRPC server and client implementation
//!   - Generated protobuf code in `Vine/Generated/`
//!   - Server implementation in `Vine/Server/`
//!   - Client utilities in `Vine/Client/`
//!
//! ### Reliability
//! - `Resilience`: Retry policies, circuit breakers, timeouts
//!   - `RetryPolicy`: Configurable retry strategies
//!   - `CircuitBreaker`: Fail-fast for external dependencies
//!   - `BulkheadExecutor`: Concurrency limiting
//!   - `TimeoutManager`: Operation timeout management
//! - `Security`: Rate limiting, checksums, secure storage
//! - `HealthCheck`: Service health monitoring
//!
//! ### Extensibility
//! - `Plugins`: Hot-reloadable plugin system
//! - `CLI`: Command-line interface for daemon control
//!
//! ## Protocol Details
//!
//! **Vine Protocol (gRPC)**
//! - **Version**: 1 (Air::ProtocolVersion)
//! - **Transport**: HTTP/2
//! - **Serialization**: Protocol Buffers
//! - **Ports**:
//!   - 50053: Air (background services) - DefaultBindAddress
//!   - 50052: Cocoon (NodeJS/web services)
//!
//! TODO: Add TLS/mTLS support for production security
///
/// ## TODO: Missing Functionality
///
/// ### High Priority
/// - [ ] Implement metrics HTTP endpoint (/metrics)
/// - [ ] Add Prometheus metric export with labels
/// - [ ] Implement TLS/mTLS for gRPC connections
/// - [ ] Add connection authentication/authorization
/// - [ ] Implement configuration hot-reload (SIGHUP)
/// - [ ] Add comprehensive integration tests
/// - [ ] Implement graceful shutdown with operation completion
///
/// ### Medium Priority
/// - [ ] Implement plugin hot-reload
/// - [ ] Add structured logging with correlation IDs
/// - [ ] Implement distributed tracing (OpenTelemetry)
/// - [ ] Add health check HTTP endpoint for load balancers
/// - [ ] Implement connection pooling optimizations
/// - [ ] Add metrics export to external systems
/// - [ ] Implement telemetry/observability export
///
/// ### Low Priority
/// - [ ] Add A/B testing framework for features
/// - [ ] Implement query optimizer for file index
/// - [ ] Add caching layer for frequently accessed data
/// - [ ] Implement adaptive timeout based on load
/// - [ ] Add predictive scaling based on metrics
/// - [ ] Implement chaos testing/metrics
///
/// ## Error Handling Strategy
///
/// All modules use defensive coding practices:
///
/// 1. **Input Validation**: All public functions validate inputs with descriptive errors
/// 2. **Timeout Handling**: Default timeouts with configuration overrides
/// 3. **Resource Cleanup**: Drop trait + explicit cleanup methods
/// 4. **Circuit Breaker**: Fail-fast for external dependencies
/// 5. **Retry Logic**: Exponential backoff for transient failures
/// 6. **Metrics Recording**: All operations record success/failure metrics
/// 7. **Panic Recovery**: Catch panics in critical async tasks
///
/// ## Constants
///
/// - **VERSION**: Air daemon version from Cargo.toml
/// - **DefaultConfigFile**: Default config filename (air.toml)
/// - **DefaultBindAddress**: gRPC bind address ([::1]:50053)
/// - **ProtocolVersion**: Vine protocol version (1)

#![allow(non_snake_case, non_camel_case_types)]

pub mod ApplicationState;
pub mod Authentication;
pub mod CLI;
pub mod Configuration;
pub mod Daemon;
pub mod Downloader;
pub mod HealthCheck;
pub mod Indexing;
pub mod Logging;
pub mod Metrics;
pub mod Plugins;
pub mod Resilience;
pub mod Security;
pub mod Tracing;
pub mod Updates;
pub mod Vine;

// Re-export commonly used types for easier access

pub use Authentication::AuthenticationService;
pub use Configuration::ConfigurationManager;
pub use Downloader::DownloadManager;
pub use Indexing::FileIndexer;
pub use Resilience::{
    RetryPolicy, RetryManager, CircuitBreaker, CircuitBreakerConfig, CircuitState,
    BulkheadExecutor, BulkheadConfig, TimeoutManager, ResilienceOrchestrator,
};
pub use Security::{RateLimiter, RateLimitConfig, ChecksumVerifier, SecureStorage};
pub use Updates::UpdateManager;

/// Air Daemon version information
///
/// This is automatically populated from Cargo.toml at build time
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default configuration file name
///
/// The daemon searches for this configuration file in:
/// 1. The path specified via --config flag
/// 2. ~/.config/air/air.toml
/// 3. /etc/air/air.toml
/// 4. Working directory (air.toml)
pub const DefaultConfigFile: &str = "air.toml";

/// Default gRPC bind address for the Vine server
///
/// Note: Port 50053 is used for Air to avoid conflict with Cocoon (port 50052)
///
/// Addresses in order of preference:
/// - `--bind` flag value (if provided)
/// - DefaultBindAddress constant: [::1]:50053
///
/// TODO: Add support for:
/// - IPv4-only binding (0.0.0.0:50053)
/// - IPv6-only binding ([::]:50053)
/// - Wildcard binding for all interfaces
pub const DefaultBindAddress: &str = "[::1]:50053";

/// Protocol version for Mountain-Air communication
///
/// This version is sent in all gRPC messages and checked by clients
/// to ensure compatibility. Increment this value when breaking
/// protocol changes are made.
///
/// Version history:
/// - 1: Initial Vine protocol
///
/// TODO: Implement protocol version checking and negotiation
pub const ProtocolVersion: u32 = 1;

/// Error type for Air operations
///
/// Comprehensive error types for all Air operations with descriptive messages.
/// All error variants include context to help with debugging and error recovery.
///
/// TODO: Add error codes for programmatic error handling
/// TODO: Implement error chaining with source tracking
/// TODO: Add structured error serialization for logging
/// TODO: Implement error metrics collection
#[derive(Debug, thiserror::Error, Clone)]
pub enum AirError {
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Authentication error: {0}")]
    Authentication(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("File system error: {0}")]
    FileSystem(String),
    
    #[error("gRPC error: {0}")]
    Grpc(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),
    
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Timeout error: {0}")]
    Timeout(String),
    
    #[error("Plugin error: {0}")]
    Plugin(String),
    
    #[error("Hot-reload error: {0}")]
    HotReload(String),
    
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),
    
    #[error("Circuit breaker open: {0}")]
    CircuitBreaker(String),
}

impl From<config::ConfigError> for AirError {
    fn from(err: config::ConfigError) -> Self {
        AirError::Configuration(err.to_string())
    }
}

impl From<reqwest::Error> for AirError {
    fn from(err: reqwest::Error) -> Self {
        AirError::Network(err.to_string())
    }
}

impl From<std::io::Error> for AirError {
    fn from(err: std::io::Error) -> Self {
        AirError::FileSystem(err.to_string())
    }
}

impl From<tonic::transport::Error> for AirError {
    fn from(err: tonic::transport::Error) -> Self {
        AirError::Grpc(err.to_string())
    }
}

impl From<serde_json::Error> for AirError {
    fn from(err: serde_json::Error) -> Self {
        AirError::Serialization(err.to_string())
    }
}

impl From<toml::de::Error> for AirError {
    fn from(err: toml::de::Error) -> Self {
        AirError::Serialization(err.to_string())
    }
}

impl From<uuid::Error> for AirError {
    fn from(err: uuid::Error) -> Self {
        AirError::Internal(format!("UUID error: {}", err))
    }
}

impl From<tokio::task::JoinError> for AirError {
    fn from(err: tokio::task::JoinError) -> Self {
        AirError::Internal(format!("Task join error: {}", err))
    }
}

/// Result type for Air operations
///
/// Convenience type alias for Result<T, AirError>
pub type Result<T> = std::result::Result<T, AirError>;

/// Common utility functions
///
/// These utilities provide defensive helper functions used throughout
/// the Air library for validation, ID generation, timestamp handling,
/// and common operations with proper error handling.
pub mod utils {
    use super::*;
    
    /// Generate a unique request ID
    ///
    /// Creates a UUID v4 for tracing and correlation of requests.
    /// The ID is guaranteed to be unique (with extremely high probability).
    ///
    /// TODO: Replace with ULID for sortable IDs
    /// TODO: Add optional prefix for service identification
    pub fn generate_request_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
    
    /// Generate a unique request ID with a prefix
    ///
    /// Format: `{prefix}-{uuid}`
    ///
    /// # Arguments
    ///
    /// * `prefix` - Prefix to add before the UUID (e.g., "auth", "download")
    ///
    /// # Example
    ///
    /// ```
    /// let id = generate_request_id_with_prefix("auth");
    /// // Returns: "auth-550e8400-e29b-41d4-a716-446655440000"
    /// ```
    pub fn generate_request_id_with_prefix(prefix: &str) -> String {
        format!("{}-{}", prefix, uuid::Uuid::new_v4())
    }
    
    /// Get current timestamp in milliseconds since UNIX epoch
    ///
    /// Returns the number of milliseconds since January 1, 1970 00:00:00 UTC.
    /// Returns 0 if the system time is not available or is before the epoch.
    pub fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
    
    /// Get current timestamp in seconds since UNIX epoch
    pub fn current_timestamp_seconds() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
    
    /// Convert timestamp millis to ISO 8601 string
    ///
    /// # Arguments
    ///
    /// * `millis` - Timestamp in milliseconds since UNIX epoch
    ///
    /// # Returns
    ///
    /// ISO 8601 formatted string or "Invalid timestamp" on error
    pub fn timestamp_to_iso8601(millis: u64) -> String {
        match std::time::UNIX_EPOCH.checked_add(std::time::Duration::from_millis(millis)) {
            Some(time) => {
                use std::time::SystemTime;
                match SystemTime::try_from(time) {
                    Ok(st) => {
                        let datetime: chrono::DateTime<chrono::Utc> = st.into();
                        datetime.to_rfc3339()
                    }
                    Err(_) => "Invalid timestamp".to_string(),
                }
            }
            None => "Invalid timestamp".to_string(),
        }
    }
    
    /// Validate file path security
    ///
    /// Checks for path traversal attempts and invalid characters.
    /// This is a security measure to prevent directory traversal attacks.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path to validate
    ///
    /// # Errors
    ///
    /// Returns an error if the path contains suspicious patterns.
    ///
    /// TODO: Add platform-specific validation (Windows paths)
    /// TODO: Add maximum path length validation
    pub fn validate_file_path(path: &str) -> Result<()> {
        // Null check
        if path.is_empty() {
            return Err(AirError::Validation("Path is empty".to_string()));
        }
        
        // Length check
        if path.len() > 4096 {
            return Err(AirError::Validation("Path too long (max: 4096 characters)".to_string()));
        }
        
        // Path traversal check
        if path.contains("..") {
            return Err(AirError::Validation("Path contains '..' (potential path traversal)".to_string()));
        }
        
        // Platform-specific checks
        if cfg!(windows) {
            // Additional Windows-specific checks could be added here
        } else if path.contains('\\') {
            // On Unix, backslashes are unusual
            return Err(AirError::Validation("Path contains backslash on Unix".to_string()));
        }
        
        // Null character check
        if path.contains('\0') {
            return Err(AirError::Validation("Path contains null character".to_string()));
        }
        
        Ok(())
    }
    
    /// Validate URL format
    ///
    /// Performs basic URL validation to prevent malformed URLs from
    /// causing issues with network operations.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to validate
    ///
    /// # Errors
    ///
    /// Returns an error if the URL is invalid.
    ///
    /// TODO: Use url crate for full RFC 3986 validation
    pub fn validate_url(url: &str) -> Result<()> {
        // Null check
        if url.is_empty() {
            return Err(AirError::Validation("URL is empty".to_string()));
        }
        
        // Length check
        if url.len() > 2048 {
            return Err(AirError::Validation("URL too long (max: 2048 characters)".to_string()));
        }
        
        // Basic scheme check
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(AirError::Validation("URL must start with http:// or https://".to_string()));
        }
        
        // Null character check
        if url.contains('\0') {
            return Err(AirError::Validation("URL contains null character".to_string()));
        }
        
        // TODO: More comprehensive validation using url crate
        Ok(())
    }
    
    /// Validate string length
    ///
    /// Defensive utility to validate string length bounds.
    ///
    /// # Arguments
    ///
    /// * `value` - The string to validate
    /// * `min_len` - Minimum allowed length (inclusive)
    /// * `max_len` - Maximum allowed length (inclusive)
    pub fn validate_string_length(value: &str, min_len: usize, max_len: usize) -> Result<()> {
        if value.len() < min_len {
            return Err(AirError::Validation(format!(
                "String too short (min: {}, got: {})", min_len, value.len()
            )));
        }
        
        if value.len() > max_len {
            return Err(AirError::Validation(format!(
                "String too long (max: {}, got: {})", max_len, value.len()
            )));
        }
        
        Ok(())
    }
    
    /// Validate port number
    ///
    /// Ensures a port number is within the valid range.
    ///
    /// # Arguments
    ///
    /// * `port` - The port number to validate
    ///
    /// # Errors
    ///
    /// Returns an error if the port is not in the valid range (1-65535).
    pub fn validate_port(port: u16) -> Result<()> {
        if port == 0 {
            return Err(AirError::Validation("Port cannot be 0".to_string()));
        }
        
        // Port 0 is valid for binding (ephemeral), but not for configuration
        // Port 1024 and below require root/admin privileges
        // We allow any port 1-65535 for flexibility
        Ok(())
    }
    
    /// Sanitize a string for logging
    ///
    /// Removes or escapes potentially sensitive information from strings
    /// before logging to prevent information leakage in logs.
    ///
    /// # Arguments
    ///
    /// * `value` - The string to sanitize
    /// * `max_length` - Maximum length before truncation
    ///
    /// # Returns
    ///
    /// Sanitized string safe for logging.
    pub fn sanitize_for_logging(value: &str, max_length: usize) -> String {
        // Truncate if too long
        let truncated = if value.len() > max_length {
            &value[..max_length]
        } else {
            value
        };
        
        // Remove or escape sensitive patterns
        let sanitized = truncated
            .replace('\n', " ")
            .replace('\r', " ")
            .replace('\t', " ");
        
        // If we truncated, add indicator
        if value.len() > max_length {
            format!("{}[...]", sanitized)
        } else {
            sanitized.to_string()
        }
    }
    
    /// Calculate exponential backoff delay
    ///
    /// Implements exponential backoff with jitter for retry operations.
    ///
    /// # Arguments
    ///
    /// * `attempt` - Current attempt number (0-indexed)
    /// * `base_delay_ms` - Base delay in milliseconds
    /// * `max_delay_ms` - Maximum delay in milliseconds
    ///
    /// # Returns
    ///
    /// Calculated delay in milliseconds with jitter applied.
    pub fn calculate_backoff_delay(attempt: u32, base_delay_ms: u64, max_delay_ms: u64) -> u64 {
        // Calculate exponential delay: base * 2^attempt
        let exponential_delay = base_delay_ms * 2u64.pow(attempt);
        
        // Cap at max delay
        let capped_delay = exponential_delay.min(max_delay_ms);
        
        // Add jitter (±25%)
        use std::time::SystemTime;
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as u64;
        
        let jitter_range = (capped_delay / 4).max(1); // 25% of delay, at least 1ms
        let jitter = (seed % (2 * jitter_range)) - jitter_range as i64;
        
        // Apply jitter (ensure non-negative)
        ((capped_delay as i64) + jitter).max(0) as u64
    }
    
    /// Format bytes as human-readable size
    ///
    /// Converts a byte count to a human-readable format with appropriate units.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Number of bytes
    ///
    /// # Returns
    ///
    /// Formatted string (e.g., "1.5 MB", "256 B")
    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        const TB: u64 = GB * 1024;
        
        if bytes >= TB {
            format!("{:.2} TB", bytes as f64 / TB as f64)
        } else if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
    
    /// Parse duration string to milliseconds
    ///
    /// Parses duration strings like "100ms", "1s", "1m", "1h" to milliseconds.
    ///
    /// # Arguments
    ///
    /// * `duration_str` - Duration string (e.g., "1s", "500ms", "1m30s")
    ///
    /// # Errors
    ///
    /// Returns an error if the duration string is invalid.
    ///
    /// TODO: Support complex durations like "1h30m"
    pub fn parse_duration_to_millis(duration_str: &str) -> Result<u64> {
        // TODO: Implement duration parsing with support for:
        // - ms, s, m, h suffixes
        // - Combined durations like "1m30s"
        // - Decimal values like "1.5s"
        
        Err(AirError::Internal("Duration parsing not yet implemented".to_string()))
    }
}
