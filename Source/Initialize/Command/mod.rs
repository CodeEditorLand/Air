//! # Command Module
//!
//! ## File: Initialize/Command/mod.rs
//!
//! ## Role in Air Architecture
//!
//! Provides CLI command parsing and handling.

pub mod ParseArguments;
pub mod HandleCommand;
pub mod ValidateCommand;
pub mod Connect;

// Convenience re-exports
pub use ParseArguments::{ParseArguments, ParsedArguments};
pub use HandleCommand::HandleCommand;
pub use ValidateCommand::ValidateCommand;
pub use Connect::ConnectDaemon::Connect;
