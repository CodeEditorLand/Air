//! # Air::Client::AirClient
//!
//! gRPC client wrapper for the Air daemon service. Callers reach Air
//! through this façade for update management, authentication, file
//! indexing, and system monitoring.
//!
//! ## Layout
//!
//! Per-message DTOs live one-per-file under this module; each declares a
//! single `pub struct Struct` to match the file name:
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
//! - [`DownloadStream`] - wraps `tonic::Streaming` to yield
//!   [`DownloadStreamChunk::Struct`] items via `.next().await`.
//!
//! ## Client core
//!
//! - [`AirClient`] struct - thread-safe gRPC client over `Arc<Mutex<…>>`
//! - [`AirClient::new`] - async connect with parsed tonic `Endpoint`
//! - [`AirClient::is_connected`] / [`AirClient::address`] accessors
//! - [`Debug`] impl
//! - [`IntoRequestExt`] - blanket helper to wrap any value as
//!   `tonic::Request<T>` with one method call
//! - [`DEFAULT_AIR_SERVER_ADDRESS`] constant (`"[::1]:50053"`)

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

// --- Per-domain method impls (each declares its own `impl AirClient` block) ---

// Authentication
pub mod Authenticate;

// Updates
pub mod ApplyUpdate;

pub mod CheckForUpdates;

pub mod DownloadUpdate;

// Downloads
pub mod DownloadFile;

pub mod DownloadStreamRpc;

// Indexing
pub mod GetFileInfo;

pub mod IndexFiles;

pub mod SearchFiles;

// Status + monitoring
pub mod GetMetrics;

pub mod GetResourceUsage;

pub mod GetStatus;

pub mod HealthCheck;

pub mod SetResourceLimits;

// Configuration
pub mod GetConfiguration;

pub mod UpdateConfiguration;

// --- Client core ---

use std::sync::Arc;

use tokio::sync::Mutex;
use tonic::transport::Channel;

use crate::{AirError, Vine::Generated::air::air_service_client::AirServiceClient, dev_log};

/// Default gRPC server address for the Air daemon.
///
/// Port allocation:
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
	/// - [`AirError::Network`] if the address parses as a tonic `Endpoint`
	///   but the underlying connection attempt fails.
	/// - [`AirError::Validation`] if the address string is malformed.
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
	/// `None` when the client is disconnected. Per-domain method impls
	/// call this and then `.lock().await` to obtain the mutex guard
	/// before issuing the RPC.
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
/// Implemented for every `T` via the blanket impl below. Per-domain method
/// impls use `payload.into_request()` instead of
/// `tonic::Request::new(payload)`.
pub trait IntoRequestExt {
	fn into_request(self) -> tonic::Request<Self>
	where Self: Sized {
		tonic::Request::new(self)
	}
}

impl<T> IntoRequestExt for T {}
