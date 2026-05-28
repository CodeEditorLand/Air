//! # Vine Error Handling (Air)
//!
//! Air's Vine errors. Re-exports the canonical [`Vine::VineError`] from the
//! `Element/Vine` crate (the source of truth, synthesised from Mountain's
//! mature implementation in 2026-05-28) and provides a thin compatibility
//! layer for Air's pre-existing variant names.
//!
//! ## Source of truth
//!
//! `Element/Vine/Source/Error.rs` defines:
//!
//! - `ClientNotConnected(String)` - sidecar absent from pool
//! - `ConnectionFailed { SideCarIdentifier, Address, Reason }`
//! - `ConnectionLost(String)`
//! - `RPCError(String)` - generic gRPC failure
//! - `RequestTimeout { SideCarIdentifier, MethodName, TimeoutMilliseconds }`
//! - `RequestCanceled { SideCarIdentifier, MethodName }`
//! - `SerializationError(serde_json::Error)`
//! - `MessageTooLarge { ActualSize, MaxSize }`
//! - `InvalidMessageFormat(String)`
//! - `TonicTransportError(tonic::transport::Error)`
//! - `InternalLockError(String)`
//! - `InvalidState(String)`
//! - `InvalidUri(http::uri::InvalidUri)`
//! - `AddressParseError(std::net::AddrParseError)`
//!
//! Plus `IsRecoverable()` / `ToTonicStatus()` helpers and `From` impls for
//! every transport/serialization error in the stack. Air re-exports the
//! type unchanged so any new Air code can reach the full surface, while the
//! existing call sites continue to work via the [`AirCompat`] shim below.
//!
//! ## Migration notes
//!
//! Old Air variants → canonical Vine variants:
//!
//! | Old (Air-local) | New (Vine crate) |
//! | --- | --- |
//! | `Transport(String)` | `TonicTransportError` or `RPCError` |
//! | `Serialization(String)` | `SerializationError(serde_json::Error)` |
//! | `ClientNotConnected(String)` | `ClientNotConnected(String)` (same) |
//! | `Timeout(String)` | `RequestTimeout { … }` |
//! | `Authentication(String)` | `RPCError("auth: …")` |
//! | `Authorization(String)` | `RPCError("authz: …")` |
//! | `Internal(String)` | `InvalidState(String)` |
//!
//! Constructors are exposed via [`AirCompat`] so consumers that wrote
//! `VineError::Transport("...".into())` keep compiling - the shim builds the
//! equivalent canonical variant.

/// Canonical Vine error type, re-exported from the `Element/Vine` crate.
///
/// New code should call this directly. Pre-existing Air code can continue
/// using the [`AirCompat`] constructor surface for source-compatible
/// migration.
pub use Vine::Error::{Result, VineError};

/// Air-flavour compatibility constructors for the canonical [`VineError`].
///
/// These mirror Air's pre-2026-05-28 variant names but produce canonical
/// `VineError` values. Use during the migration; call sites should switch
/// to the canonical constructors when convenient.
pub struct AirCompat;

impl AirCompat {
	/// Maps the old `Transport(String)` variant onto the canonical
	/// `RPCError(String)`. Use [`VineError::TonicTransportError`] directly
	/// for `tonic::transport::Error` sources.
	pub fn Transport(Message:impl Into<String>) -> VineError {
		VineError::RPCError(format!("transport: {}", Message.into()))
	}

	/// Maps the old `Serialization(String)` variant onto the canonical
	/// `RPCError(String)`. Use [`VineError::SerializationError`] directly for
	/// `serde_json::Error` sources.
	pub fn Serialization(Message:impl Into<String>) -> VineError {
		VineError::RPCError(format!("serialization: {}", Message.into()))
	}

	/// Same shape as the canonical variant.
	pub fn ClientNotConnected(Identifier:impl Into<String>) -> VineError {
		VineError::ClientNotConnected(Identifier.into())
	}

	/// Maps the old `Timeout(String)` variant onto the canonical
	/// `RequestTimeout` structured variant. Caller passes a descriptive
	/// message; the structured fields are filled with defaults.
	pub fn Timeout(Message:impl Into<String>) -> VineError {
		VineError::RequestTimeout {
			SideCarIdentifier:"unknown".to_string(),
			MethodName:Message.into(),
			TimeoutMilliseconds:0,
		}
	}

	/// Maps the old `Authentication(String)` variant onto a canonical
	/// `RPCError` with an `auth:` prefix so log greps still hit.
	pub fn Authentication(Message:impl Into<String>) -> VineError {
		VineError::RPCError(format!("auth: {}", Message.into()))
	}

	/// Maps the old `Authorization(String)` variant onto a canonical
	/// `RPCError` with an `authz:` prefix.
	pub fn Authorization(Message:impl Into<String>) -> VineError {
		VineError::RPCError(format!("authz: {}", Message.into()))
	}

	/// Maps the old `Internal(String)` variant onto the canonical
	/// `InvalidState(String)`.
	pub fn Internal(Message:impl Into<String>) -> VineError { VineError::InvalidState(Message.into()) }
}
