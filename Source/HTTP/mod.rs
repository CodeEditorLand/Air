//! HTTP Client Module
//!
//! This module provides secured HTTP clients with DNS override functionality.
//! All HTTP clients in this module use the local DNS server for DNS resolution,
//! ensuring that `*.editor.land` domains resolve only to `127.x.x.x` addresses.

mod client;

// Re-export public items from client module for external use
pub use client::secured_client;
pub use client::secured_client_with_timeout;
pub use client::secured_client_builder;
// Note: LandDnsResolver, TokioResolver, and land_resolver are re-exported from
// within client.rs from the Mist crate, so they should be accessible via client
