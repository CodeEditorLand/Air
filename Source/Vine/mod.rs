//! # Vine Protocol Implementation (Air)
//!
//! Air's Vine surface. Implements the gRPC protocol that runs on Air's
//! side of the bus - Air hosts `AirVinegRPCService` on `[::1]:50053` so
//! callers (Mountain, external clients, integration tests) can reach
//! Air's background services (indexing, downloads, updates,
//! authentication).
//!
//! ## Layout
//!
//! - [`Generated`] - Air-local prost output for `Air.proto`. Air's own
//!   service definitions (`UpdateService`, `IndexingService`,
//!   `DownloaderService`, `AuthenticationService`) live here.
//! - [`Server`] - Air's gRPC server implementation
//!   (`Server::AirVinegRPCService`), wiring Air's `ApplicationState` into
//!   the generated service traits.
//! - [`Error`] - re-exports the canonical [`VineError`] from the
//!   `Vine` crate and exposes [`Error::AirCompat`] alias constructors.
//!
//! ## Re-exports from the `Vine` crate
//!
//! For convenience, the most-used `Vine` crate items are re-exported at
//! this module's root so Air code can write `use crate::Vine::{VineHost,
//! IPCProvider};` without spelling out the workspace crate path:
//!
//! - [`VineHost`] / [`ApplicationStateAccess`] / [`IPCProvider`] -
//!   embedder seam (Air's `ApplicationState` implements these to host the
//!   `Vine` notification handler tree against Air's runtime).
//! - [`ProtocolVersion`] / [`DefaultRequestTimeoutMs`] /
//!   [`DefaultMaxMessageSize`] / [`DefaultAirAddress`] - canonical
//!   protocol constants.

pub mod Error;

pub mod Generated;

pub mod Server;

// --- Re-exports from `Vine` ---

pub use ::Vine::{
	ApplicationStateAccess,
	DefaultAirAddress,
	DefaultMaxMessageSize,
	DefaultRequestTimeoutMs,
	IPCProvider,
	ProtocolVersion,
	VineHost,
};
