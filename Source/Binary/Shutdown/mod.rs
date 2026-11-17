//! # Shutdown
//!
//! Binary shutdown leaf facade. `Binary.rs` reaches the signal handler
//! through `crate::Binary::Shutdown::WaitForShutdownSignal`.

pub mod WaitForShutdownSignal;
