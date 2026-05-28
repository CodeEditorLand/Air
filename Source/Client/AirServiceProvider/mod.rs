//! # Air::Client::AirServiceProvider
//!
//! High-level wrappers around `crate::Client::AirClient` that hide the
//! gRPC plumbing behind ergonomic methods (`SearchFiles`, `GetMetrics`,
//! `CheckForUpdates`, `AuthenticateUser`, `DownloadFile`, …). Each method
//! generates a request id, issues the gRPC call, and translates errors
//! into [`crate::AirError`].
//!
//! ## Synthesis status (2026-05-28)
//!
//! - [`GenerateRequestID::Fn`] - **ported as thin wrapper**. Mountain's helper
//!   duplicates `Uuid::new_v4().simple()`; Air already exposes that at
//!   [`crate::Utility::GenerateRequestId`]. The Air-side file delegates to the
//!   canonical helper, discharging the audit's dedup item.
//!
//! Pending:
//!
//! - Top-level `AirServiceProvider.rs` (794 LOC in Mountain) - high-level
//!   provider surface. Big lift; deferred to a follow-up slice.

pub mod GenerateRequestID;
