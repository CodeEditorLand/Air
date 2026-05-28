//! # Air::Client
//!
//! Client-side gRPC wrappers for callers that connect to Air's
//! `AirService` server (Mountain in the editor process, external scripts,
//! integration tests). Air owns `Air.proto` and the matching prost / tonic
//! types under [`crate::Vine::Generated::air`]; this module wraps those
//! generated types in an ergonomic façade.
//!
//! ## Layout
//!
//! - [`AirClient`] - low-level gRPC client wrapper. Holds an
//!   `Arc<Mutex<AirServiceClient<Channel>>>` so clones share one channel.
//!   Per-domain methods (authentication, updates, downloads, indexing,
//!   monitoring) live as `impl AirClient` blocks in the same module.
//! - [`AirServiceProvider`] - high-level surface that wraps
//!   [`AirClient`] with automatic request-id generation, structured
//!   error translation, and ergonomic per-operation method signatures.
//!
//! ## Threading model
//!
//! Cheap to clone (Arc ref-count bump). The interior `tokio::sync::Mutex`
//! serialises concurrent RPCs on a single channel - tonic recommends this
//! pattern for clients that are shared across many call sites.

pub mod AirClient;

pub mod AirServiceProvider;
