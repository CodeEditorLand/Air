//! # Connect Module
//!
//! ## File: Initialize/Command/Connect/mod.rs
//!
//! ## Role in Air Architecture
//!
//! Provides daemon connection functionality for CLI commands.

pub mod ConnectDaemon;

// Convenience re-exports
pub use ConnectDaemon::Connect;
