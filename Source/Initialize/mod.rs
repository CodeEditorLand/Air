//! # Initialize Module
//!
//! ## File: Initialize/mod.rs
//!
//! ## Role in Air Architecture
//!
//! Provides initialization functions for all Air daemon components,
//! including configuration, service setup, and CLI command handling.
//!
//! ## Public Exports
//!
//! - `Configure::Log::ConfigureLog` - Logging configuration
//! - `Configure::Port::SelectPort` - Port binding configuration
//! - `Build::BuildServer` - gRPC server building
//! - `Service::*` - Service initialization
//! - `Command::*` - CLI command handling
//!
//! ## Mod Organization
//!
//! - `Configure/` - Configuration-related initialization
//! - `Build/` - Server building
//! - `Service/` - Service startup
//! - `Command/` - CLI command handling

pub mod Configure;
pub mod Build;
pub mod Service;
pub mod Command;
