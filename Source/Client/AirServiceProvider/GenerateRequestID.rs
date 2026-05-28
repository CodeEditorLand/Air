//! Generate a fresh UUID-v4 (simple form) for use as an Air request id.
//!
//! Each Air RPC carries one of these so callers can correlate replies with
//! the originating call across log lines + traces. Delegates to
//! [`crate::Utility::GenerateRequestId`] - the crate-canonical helper -
//! so the same generator is used everywhere request ids are issued.

pub fn Fn() -> String { crate::Utility::GenerateRequestId() }
