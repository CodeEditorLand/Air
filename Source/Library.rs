//! # Air Library
//!
//! Core library for the Air daemon - the persistent background service for Land.
//! Provides services for authentication, updates, downloads, and file indexing
//! that run independently from the main Mountain application.

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

// Re-export commonly used types

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
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default configuration file name
pub const DefaultConfigFile: &str = "air.toml";

/// Default gRPC bind address (Note: Moved to port 50053 to avoid conflict with Cocoon which uses 50052)
pub const DefaultBindAddress: &str = "[::1]:50053";

/// Protocol version for Mountain-Air communication
pub const ProtocolVersion: u32 = 1;

/// Error type for Air operations
#[derive(Debug, thiserror::Error)]
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

/// Result type for Air operations
pub type Result<T> = std::result::Result<T, AirError>;

/// Common utility functions
pub mod utils {
    use super::*;
    
    /// Generate a unique request ID
    pub fn GenerateRequestId() -> String {
        uuid::Uuid::new_v4().to_string()
    }
    
    /// Get current timestamp in milliseconds
    pub fn CurrentTimestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
    
    /// Validate file path security
    pub fn ValidateFilePath(path: &str) -> Result<()> {
        if path.contains("..") || path.contains("\\") {
            return Err(AirError::Configuration("Invalid file path".to_string()));
        }
        Ok(())
    }
}
