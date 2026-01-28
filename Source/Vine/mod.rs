//! # Vine Protocol Implementation
//!
//! Implements the gRPC protocol for communication between Mountain and Air.

pub mod Generated;
pub mod Server;
pub mod Error;

// Re-export commonly used types
pub use Server::AirVinegRPCService;
