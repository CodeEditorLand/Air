//! # Vine Protocol Implementation (Air)
//!
//! Air's Vine surface. Implements the gRPC protocol that runs **on Air's
//! side** of the bus - Air hosts `AirVinegRPCService` on
//! `[::1]:50053` so Mountain (and external clients) can talk to Air's
//! background services (indexing, downloads, updates, authentication).
//!
//! ## Two-crate layout (as of 2026-05-28)
//!
//! The canonical Vine protocol types + client primitives + (eventually)
//! server scaffolding live in the `Element/Vine` workspace crate. Air
//! depends on that crate via `Vine = { workspace = true }` and exposes its
//! richer surface alongside Air's own server impl:
//!
//! - [`Generated`] - **Air-local** prost output for `Air.proto`. Air's own
//!   service definitions (UpdateService, IndexingService, DownloaderService,
//!   AuthenticationService) live here, not in the cross-cutting Vine crate.
//! - [`Server`] - Air's gRPC server implementation
//!   (`Server::AirVinegRPCService`), wiring Air's `ApplicationState` into the
//!   generated service traits.
//! - [`Error`] - re-exports the canonical `VineError` from the Vine crate and
//!   exposes `Error::AirCompat` constructors for migration of legacy call sites
//!   that used Air's pre-2026-05-28 variant names.
//!
//! ## Imported from the Vine crate
//!
//! For convenience, the most-used Vine crate items are re-exported at this
//! module's root so Air code can write `use crate::Vine::{VineError,
//! VineHost, IPCProvider};` without spelling out the workspace path:
//!
//! - `VineError` / `Result` - canonical error / result types
//! - `VineHost` / `ApplicationStateAccess` / `IPCProvider` - embedder seam (Air
//!   will implement these on its `ApplicationState` in a later slice so the
//!   server-side handler tree can be hosted against Air's runtime)
//! - `ProtocolVersion` / `DefaultRequestTimeoutMs` / `DefaultAirAddress` -
//!   canonical constants
//!
//! Air's own protocol additions (e.g. `Air.proto`-specific message types)
//! continue to live under [`Generated`] and are accessed via that
//! sub-module unchanged.

pub mod Error;

pub mod Generated;

pub mod Server;

// --- Canonical re-exports from `Element/Vine` ---

pub use ::Vine::{
	ApplicationStateAccess,
	DefaultAirAddress,
	DefaultMaxMessageSize,
	DefaultRequestTimeoutMs,
	IPCProvider,
	ProtocolVersion,
	VineHost,
};
