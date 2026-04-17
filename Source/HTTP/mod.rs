//! HTTP Client Module
//!
//! This module provides secured HTTP clients with DNS override functionality.
//! All HTTP clients in this module use the local DNS server for DNS resolution,
//! ensuring that `*.editor.land` domains resolve only to `127.x.x.x` addresses.

#[path = "Client.rs"]
mod Client;

// Re-export public items from Client module for external use
pub use Client::{secured_client, secured_client_builder, secured_client_with_timeout};
// Note: LandDnsResolver, TokioResolver, and land_resolver are re-exported from
// within Client.rs from the Mist crate, so they should be accessible via Client
