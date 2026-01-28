//! # Vine Error Handling
//!
//! Error types and utilities for Vine gRPC communication.

use thiserror::Error;

/// Error type for Vine operations
#[derive(Debug, Error)]
pub enum VineError {
    #[error("Transport error: {0}")]
    Transport(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Client not connected: {0}")]
    ClientNotConnected(String),
    
    #[error("Request timeout: {0}")]
    Timeout(String),
    
    #[error("Authentication error: {0}")]
    Authentication(String),
    
    #[error("Authorization error: {0}")]
    Authorization(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<tonic::transport::Error> for VineError {
    fn from(err: tonic::transport::Error) -> Self {
        VineError::Transport(err.to_string())
    }
}

impl From<serde_json::Error> for VineError {
    fn from(err: serde_json::Error) -> Self {
        VineError::Serialization(err.to_string())
    }
}

impl From<std::net::AddrParseError> for VineError {
    fn from(err: std::net::AddrParseError) -> Self {
        VineError::Transport(format!("Invalid address: {}", err))
    }
}

impl From<std::io::Error> for VineError {
    fn from(err: std::io::Error) -> Self {
        VineError::Internal(err.to_string())
    }
}
