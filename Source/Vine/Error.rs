//! # Vine Error Handling (Air)
//!
//! Air's Vine errors. Re-exports the canonical [`Vine::Error::VineError`]
//! and provides a thin compatibility layer ([`AirCompat`]) that maps Air's
//! variant-name aliases onto the canonical variants.
//!
//! ## Canonical variants
//!
//! [`VineError`] exposes:
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
//! Plus [`VineError::IsRecoverable`] / [`VineError::ToTonicStatus`] and
//! `From` impls for every transport / serialization error in the stack.
//!
//! ## Air-flavour aliases
//!
//! [`AirCompat`] exposes constructor helpers under Air's prior variant
//! names (`Transport` / `Serialization` / `ClientNotConnected` / `Timeout`
//! / `Authentication` / `Authorization` / `Internal`) that build the
//! equivalent canonical [`VineError`] value. Use the canonical
//! constructors directly when adding new code; the aliases exist so
//! older call sites continue to compile.

/// Canonical Vine error type, re-exported from the `Vine` crate.
///
/// New code should construct this directly. The [`AirCompat`] helpers
/// below remain available for callers that prefer Air's prior variant
/// names.
/// Canonical Vine error enum, surfaced through Air's `Vine::Error` path.
pub type VineError = ::Vine::Error::VineError;

/// `Result<T, VineError>` convenience alias.
pub type Result<T> = ::Vine::Error::Result<T>;

/// Air-flavour compatibility constructors for the canonical [`VineError`].
///
/// Each method builds a canonical [`VineError`] value under Air's prior
/// variant name.
pub struct AirCompat;

impl AirCompat {
	/// Builds [`VineError::RPCError`] tagged with a `transport:` prefix.
	/// Use [`VineError::TonicTransportError`] directly when constructing
	/// from a `tonic::transport::Error` source.
	pub fn Transport(Message:impl Into<String>) -> VineError {
		VineError::RPCError(format!("transport: {}", Message.into()))
	}

	/// Builds [`VineError::RPCError`] tagged with a `serialization:`
	/// prefix. Use [`VineError::SerializationError`] directly when
	/// constructing from a `serde_json::Error` source.
	pub fn Serialization(Message:impl Into<String>) -> VineError {
		VineError::RPCError(format!("serialization: {}", Message.into()))
	}

	/// Same shape as the canonical variant.
	pub fn ClientNotConnected(Identifier:impl Into<String>) -> VineError {
		VineError::ClientNotConnected(Identifier.into())
	}

	/// Builds [`VineError::RequestTimeout`] with `SideCarIdentifier` and
	/// `TimeoutMilliseconds` set to placeholder defaults; the message is
	/// stored in `MethodName`.
	pub fn Timeout(Message:impl Into<String>) -> VineError {
		VineError::RequestTimeout {
			SideCarIdentifier:"unknown".to_string(),

			MethodName:Message.into(),

			TimeoutMilliseconds:0,
		}
	}

	/// Builds [`VineError::RPCError`] tagged with an `auth:` prefix so log
	/// greps still hit.
	pub fn Authentication(Message:impl Into<String>) -> VineError {
		VineError::RPCError(format!("auth: {}", Message.into()))
	}

	/// Builds [`VineError::RPCError`] tagged with an `authz:` prefix.
	pub fn Authorization(Message:impl Into<String>) -> VineError {
		VineError::RPCError(format!("authz: {}", Message.into()))
	}

	/// Maps onto [`VineError::InvalidState`].
	pub fn Internal(Message:impl Into<String>) -> VineError { VineError::InvalidState(Message.into()) }
}
