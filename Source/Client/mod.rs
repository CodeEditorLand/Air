//! # Air::Client
//!
//! Destination crate-side module for the Air-Client synthesis described in
//! plan task #4 of `.hermes/plan/Mountain-Crate-Split.md`. Today this is
//! a scaffold; Phase 2 lands the actual ported content here.
//!
//! ## Why this module
//!
//! Air owns `Air.proto` and its server (`Vine::Server::AirVinegRPCService`).
//! Mountain currently carries the matching *client* side in
//! `Mountain/Source/Air/{AirClient,AirServiceProvider,AirServiceTypesStub}`
//! (35 files, ~2 191 LOC of live Rust). Both halves of the same gRPC
//! contract belong together; the client should live in the crate that owns
//! the proto.
//!
//! ## Synthesis status (2026-05-28)
//!
//! Phase 1 (audit) - **done**. See
//! `.hermes/plan/AirClient-Synthesis-Audit.md` for the full file
//! inventory, the eight audited improvements that must survive the port
//! (connection pooling + retry, mTLS handshake env-var pattern, atomic /
//! UUID request-ID generation, high-level provider surface, deduplication
//! against `crate::Utility::GenerateRequestId{,WithPrefix}`, deprecated
//! `AirServiceTypesStub` skip-list, canonical default address
//! `[::1]:50053` vs. the stub's buggy `127.0.0.1:50051`), and the Phase 2
//! / Phase 3 migration plan.
//!
//! Phase 2 (synthesise) - **scheduled for next Disperse session**. Will:
//!
//! - Add `client` and `server` cargo features to `Air/Cargo.toml`. `server`
//!   gates the existing daemon modules (`ApplicationState`, `Authentication`,
//!   `Daemon`, `Downloader`, `HealthCheck`, `Indexing`, `Updates`,
//!   `Vine::Server::AirVinegRPCService`). `client` (this module) keeps Air's
//!   transitive graph slim for Mountain's consumption.
//! - Port `Mountain/Source/Air/AirClient.rs` (1 266 LOC) into
//!   `Air/Source/Client/AirClient.rs` with path rewrites only.
//! - Port the 11 atomized `AirClient/*` DTOs verbatim.
//! - Port `Mountain/Source/Air/AirServiceProvider.rs` (794 LOC), removing the
//!   local `GenerateRequestID::Fn` duplicate in favour of Air's existing
//!   `crate::Utility::GenerateRequestId{,WithPrefix}` helpers.
//! - **Skip** `Mountain/Source/Air/AirServiceTypesStub/*` entirely (header note
//!   in source: "Zero callers as of 2026-05-02. Remove this entire module when
//!   the live Air client is wired in"). Deprecated, dead code, wrong port
//!   constant.
//!
//! Phase 3 (migrate) - Mountain switches to `pub use Air::Client::*;`
//! re-export under `#[deprecated]`, then drops the shim once user signs
//! off.
//!
//! ## What lives here today
//!
//! Nothing yet. This `mod.rs` exists so the `Air/Source/Library.rs`
//! `pub mod Client;` declaration compiles, and so the Phase 2 session has
//! a known landing site for the file moves.
//!
//! Mountain's `Source/Air/` continues to function as the canonical
//! implementation until Phase 2 lands.

// Phase 2 of the audit-driven port has begun: 9 pure-data DTO atoms from
// `Mountain/Source/Air/AirClient/` have landed under `Client::AirClient::*`.
// Remaining work (per `.hermes/plan/AirClient-Synthesis-Audit.md`):
//
// - `AirClient.rs` (1 266 LOC) - top-level gRPC client wrapper
// - `AirClient/DownloadStream.rs` (45 LOC) - `tonic::Streaming` wrapper
// - `AirServiceProvider.rs` (794 LOC) - high-level provider surface
// - `AirServiceProvider/GenerateRequestID.rs` - deduplicate against
//   `crate::Utility::GenerateRequestId{,WithPrefix}`
//
// `Mountain/Source/Air/AirServiceTypesStub/*` is the audited skip-list.

pub mod AirClient;

pub mod AirServiceProvider;
