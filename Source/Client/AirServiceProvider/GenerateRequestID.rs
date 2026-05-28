//! Generate a fresh UUID-v4 (simple form) for use as an Air request id.
//! Each Air RPC carries one of these so the embedder can correlate replies
//! with the originating call across log lines + traces.
//!
//! Synthesised from
//! `Mountain/Source/Air/AirServiceProvider/GenerateRequestID.rs` per
//! `.hermes/plan/AirClient-Synthesis-Audit.md` § "Pre-existing Air-side
//! coverage". Mountain's helper is a thin `Uuid::new_v4().simple()` call;
//! Air already exposes the equivalent at [`crate::Utility::GenerateRequestId`]
//! with the same UUID-v4 generator. This file delegates to that canonical
//! helper rather than duplicating the implementation - the dedup discharges
//! the audit checklist's "deduplicate against Air's existing helper" item.
//!
//! Mountain's port (Phase 3) will switch its `use` statement from
//! `crate::Air::AirServiceProvider::GenerateRequestID` to
//! `Air::Client::AirServiceProvider::GenerateRequestID` and pick up Air's
//! canonical implementation transparently.

pub fn Fn() -> String { crate::Utility::GenerateRequestId() }
