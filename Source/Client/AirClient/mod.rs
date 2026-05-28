//! # Air::Client::AirClient
//!
//! gRPC client wrapper for the Air daemon service. Mountain reaches Air
//! through this façade for update management, authentication, file
//! indexing, and system monitoring. Synthesised from
//! `Mountain/Source/Air/AirClient.rs` per
//! `.hermes/plan/AirClient-Synthesis-Audit.md`. Mountain's copy stays as
//! source of truth until Phase 3 migration; this module receives the
//! canonical impl one slice at a time.
//!
//! ## Atomized DTOs (one wire-shape per file)
//!
//! Pure data DTOs (zero coupling to runtime types) - **ported**:
//!
//! - [`AirMetrics`] - daemon resource snapshot
//! - [`AirStatus`] - daemon uptime / request totals
//! - [`DownloadStreamChunk`] - one streaming download frame
//! - [`ExtendedFileInfo`] - file metadata for index queries
//! - [`FileInfo`] - downloaded-file metadata
//! - [`FileResult`] - file-search hit
//! - [`IndexInfo`] - indexing-progress snapshot
//! - [`ResourceUsage`] - process resource counts
//! - [`UpdateInfo`] - available-update metadata
//!
//! Streaming wrapper (uses `tonic::codec::Streaming` +
//! `crate::Vine::Generated::air::DownloadStreamResponse`) - **ported**:
//!
//! - [`DownloadStream`] - wraps `tonic::Streaming` to yield
//!   [`DownloadStreamChunk::Struct`] items via `.next().await`.
//!
//! ## Client foundation - **ported in this slice**
//!
//! - [`AirClient`] struct - thread-safe gRPC client over `Arc<Mutex<…>>`
//! - [`AirClient::new`] - async connect with parsed `Endpoint`
//! - [`AirClient::is_connected`] / [`AirClient::address`] accessors
//! - [`Debug`] impl
//! - [`IntoRequestExt`] - blanket helper to wrap any value as
//!   `tonic::Request<T>` with one method call
//! - [`DEFAULT_AIR_SERVER_ADDRESS`] constant (`"[::1]:50053"`, matches
//!   `crate::Vine::DefaultAirAddress`)
//!
//! ## Pending in follow-up slices
//!
//! Each domain method on `AirClient` is ~30-60 LOC of pattern:
//! `client.lock().await.<rpc>(request).await.map_err(...)`. They port
//! one-or-two at a time:
//!
//! - `authenticate` / `validate_token` (Authentication ops)
//! - `check_for_update` / `apply_update` (Update ops)
//! - `download_file` / `download_file_streaming` (Downloader ops)
//! - `index_files` / `search_files` / `get_file_info` (Indexing ops)
//! - `get_metrics` / `get_status` (Monitoring ops)
//!
//! Until those land, Mountain's `Source/Air/AirClient.rs` keeps providing
//! the methods.

// --- DTO submodules ---

pub mod AirMetrics;

pub mod AirStatus;

pub mod DownloadStream;

pub mod DownloadStreamChunk;

pub mod ExtendedFileInfo;

pub mod FileInfo;

pub mod FileResult;

pub mod IndexInfo;

pub mod ResourceUsage;

pub mod UpdateInfo;

// --- Client foundation ---

use std::sync::Arc;

use tokio::sync::Mutex;
use tonic::transport::Channel;

use crate::{AirError, Vine::Generated::air::air_service_client::AirServiceClient, dev_log};

/// Default gRPC server address for the Air daemon.
///
/// Port allocation (canonical, mirrors `crate::Vine::DefaultAirAddress`):
///
/// - `50051` - Mountain Vine server
/// - `50052` - Cocoon Vine server
/// - `50053` - Air Vine server (this constant)
pub const DEFAULT_AIR_SERVER_ADDRESS:&str = "[::1]:50053";

/// Air gRPC client wrapper.
///
/// Thread-safe via `Arc<Mutex<…>>`. Clones share the same underlying
/// channel - clone is cheap (Arc ref-count bump). The `address` field is
/// kept as the string the caller passed in so logs / `Debug` see the
/// original form (`http://[::1]:50053`, etc.).
#[derive(Clone)]
pub struct AirClient {
	/// Underlying tonic gRPC client wrapped in `Arc<Mutex<>>` for shared,
	/// thread-safe access from multiple call sites.
	client:Option<Arc<Mutex<AirServiceClient<Channel>>>>,

	/// Address of the Air daemon.
	address:String,
}

impl AirClient {
	/// Connects to the Air daemon at `address` and returns a ready-to-use
	/// client.
	///
	/// # Arguments
	///
	/// * `address` - gRPC server address (e.g. `"http://[::1]:50053"`).
	///
	/// # Errors
	///
	/// - `AirError::Network` if the address parses as a tonic `Endpoint` but
	///   the underlying connection attempt fails.
	/// - `AirError::Validation` if the address string is malformed.
	pub async fn new(address:&str) -> Result<Self, AirError> {
		dev_log!("grpc", "[AirClient] Connecting to Air daemon at: {}", address);

		let Endpoint = address.parse::<tonic::transport::Endpoint>().map_err(|E| {
			dev_log!("grpc", "error: [AirClient] Failed to parse address '{}': {}", address, E);

			AirError::Validation(format!("Invalid address '{}': {}", address, E))
		})?;

		let Channel = Endpoint.connect().await.map_err(|E| {
			dev_log!("grpc", "error: [AirClient] Failed to connect to Air daemon: {}", E);

			AirError::Network(format!("Connection failed: {}", E))
		})?;

		dev_log!("grpc", "[AirClient] Successfully connected to Air daemon at: {}", address);

		let Client = Arc::new(Mutex::new(AirServiceClient::new(Channel)));

		Ok(Self { client:Some(Client), address:address.to_string() })
	}

	/// Whether the client is connected to the Air daemon.
	pub fn is_connected(&self) -> bool { self.client.is_some() }

	/// Address of the Air daemon.
	pub fn address(&self) -> &str { &self.address }

	/// Borrow the underlying tonic client for issuing RPCs. Returns
	/// `None` when the client is disconnected.
	///
	/// Domain methods on `AirClient` (added one slice at a time per
	/// `.hermes/plan/AirClient-Synthesis-Audit.md`) call this and then
	/// `.lock().await` to obtain the mutex guard before issuing the RPC.
	pub(crate) fn Client(&self) -> Option<&Arc<Mutex<AirServiceClient<Channel>>>> { self.client.as_ref() }
}

impl std::fmt::Debug for AirClient {
	fn fmt(&self, f:&mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "AirClient({})", self.address) }
}

// ============================================================================
// tonic::Request Helper
// ============================================================================

/// Helper trait for converting any value into a `tonic::Request<T>`.
///
/// Implemented for every `T` via the blanket impl below. Used by the
/// domain-method ports to write `payload.into_request()` instead of
/// `tonic::Request::new(payload)`.
pub trait IntoRequestExt {
	fn into_request(self) -> tonic::Request<Self>
	where
		Self: Sized, {
		tonic::Request::new(self)
	}
}

impl<T> IntoRequestExt for T {}
