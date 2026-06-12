//! # Mountain Client Module
//!
//! This module provides the gRPC client implementation for Air to communicate
//! with Mountain. Air acts as a client connecting to Mountain's gRPC server
//! for requesting status, health checks, and configuration operations.
//!
//! ## Architecture
//!
//! ```text
//! Air (Background Daemon) ──► MountainClient ──► gRPC ──► Mountain (Main App)
//! ```
//!
//! ## Features
//!
//! - **Connection Management**: Establish and maintain gRPC connections to
//!   Mountain
//! - **Health Monitoring**: Check Mountain's health status
//! - **Status Queries**: Query Mountain's operational status
//! - **Configuration**: Get and update Mountain configuration
//!
//! ## Configuration
//!
//! - **Default Address**: `[::1]:50051` (Mountain's default Vine server port)
//! - **Transport**: gRPC over TCP/IP with optional TLS
//! - **Timeouts**: Configurable connection and request timeouts
//!
//! ## TLS/mTLS Support
//!
//! The `mtls` feature enables TLS client support with:
//! - Client certificate authentication
//! - Secure encrypted communications
//! - Certificate validation against CA
//!
//! Note: TLS/mTLS implementation is a stub for future enhancement. The current
//! implementation focuses on establishing unencrypted connections for
//! development and testing purposes.

pub mod Constants;

pub mod MountainClient;

pub mod MountainClientConfig;

pub mod TlsConfig;
