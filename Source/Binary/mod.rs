//! # Binary Module
//!
//! ## File: Binary/mod.rs
//!
//! ## Role in Air Architecture
//!
//! Main entry point and orchestration for the Air daemon.

pub mod Binary;
pub mod Shutdown;
pub mod Monitor;

// Convenience re-exports
pub use Binary::Main;
pub use Shutdown::WaitForShutdownSignal;
pub use Monitor::{StartMonitoring, MonitoringHandles};
