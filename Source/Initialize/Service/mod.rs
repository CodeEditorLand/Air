//! # Service Module
//!
//! ## File: Initialize/Service/mod.rs
//!
//! ## Role in Air Architecture
//!
//! Provides initialization functions for all Air daemon services.

pub mod Echo;
pub mod State;
pub mod Health;
pub mod Auth;
pub mod Update;
pub mod Download;
pub mod Index;
pub mod Vine;

// Convenience re-exports
pub use Echo::StartEcho;
pub use State::CreateState;
pub use Health::StartHealthCheck;
pub use Auth::StartAuth;
pub use Update::StartUpdate;
pub use Download::StartDownload;
pub use Index::StartIndex;
pub use Vine::{StartService, WaitForShutdown as WaitForServiceShutdown};
