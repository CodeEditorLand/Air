//! HTTP Client Module
//!
//! This module provides secured HTTP clients with DNS override functionality.
//! All HTTP clients in this module use the local DNS server for DNS resolution,
//! ensuring that `*.editor.land` domains resolve only to `127.x.x.x`
//! addresses.

#[path = "Client.rs"]
pub mod Client;
