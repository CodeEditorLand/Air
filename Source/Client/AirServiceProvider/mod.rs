//! # Air::Client::AirServiceProvider
//!
//! High-level façade over [`crate::Client::AirClient::AirClient`]. Each
//! method generates a request id via [`GenerateRequestID::Fn`], issues the
//! gRPC call, and returns [`crate::AirError`] on failure. Compared with
//! the raw `AirClient` surface, this layer:
//!
//! - hides request-id plumbing from callers,
//! - returns ergonomic shapes (`CheckForUpdates` collapses
//!   `update_available == false` into `Ok(None)`),
//! - keeps a shared `Arc<AirClient>` so all callers reuse the same gRPC
//!   channel.
//!
//! ## Layout
//!
//! Per-domain methods live one-per-file under this module; each declares
//! a single `impl AirServiceProvider { … }` block:
//!
//! - Authentication: [`Authenticate`]
//! - Updates: [`CheckForUpdates`], [`DownloadUpdate`], [`ApplyUpdate`]
//! - Downloads: [`DownloadFile`], [`DownloadStream`]
//! - Indexing: [`IndexFiles`], [`SearchFiles`], [`GetFileInfo`]
//! - Status / monitoring: [`GetStatus`], [`HealthCheck`], [`GetMetrics`]
//! - Resources: [`GetResourceUsage`], [`SetResourceLimits`]
//! - Configuration: [`GetConfiguration`], [`UpdateConfiguration`]
//!
//! ## Threading model
//!
//! Cheap to clone (Arc ref-count bump). The interior `tokio::sync::Mutex`
//! on the underlying `AirClient` serialises concurrent RPCs on a single
//! channel.

// --- Request-id helper ---

pub mod GenerateRequestID;

// --- Authentication ---

pub mod Authenticate;

// --- Updates ---

pub mod ApplyUpdate;

pub mod CheckForUpdates;

pub mod DownloadUpdate;

// --- Downloads ---

pub mod DownloadFile;

pub mod DownloadStream;

// --- Indexing ---

pub mod GetFileInfo;

pub mod IndexFiles;

pub mod SearchFiles;

// --- Status + monitoring ---

pub mod GetMetrics;

pub mod GetStatus;

pub mod HealthCheck;

// --- Resource management ---

pub mod GetResourceUsage;

pub mod SetResourceLimits;

// --- Configuration ---

pub mod GetConfiguration;

pub mod UpdateConfiguration;

// --- Provider core ---

use std::sync::Arc;

use crate::{
	AirError,
	Client::AirClient::{AirClient, DEFAULT_AIR_SERVER_ADDRESS},
	dev_log,
};

/// High-level provider over [`AirClient`]. Holds the client in an
/// `Arc` so consumers can share one channel across the application.
#[derive(Debug, Clone)]
pub struct AirServiceProvider {
	/// Shared underlying gRPC client.
	client:Arc<AirClient>,
}

impl AirServiceProvider {
	/// Connects to the Air daemon at `address` and returns a ready-to-use
	/// provider.
	///
	/// # Errors
	///
	/// Forwards any error from [`AirClient::new`].
	pub async fn new(address:String) -> Result<Self, AirError> {
		dev_log!("grpc", "[AirServiceProvider] Creating AirServiceProvider at: {}", address);

		let Client = AirClient::new(&address).await?;

		dev_log!("grpc", "[AirServiceProvider] AirServiceProvider created successfully");

		Ok(Self { client:Arc::new(Client) })
	}

	/// Connects using [`DEFAULT_AIR_SERVER_ADDRESS`].
	pub async fn NewDefault() -> Result<Self, AirError> { Self::new(DEFAULT_AIR_SERVER_ADDRESS.to_string()).await }

	/// Wraps an existing [`AirClient`] handle. Useful when the gRPC
	/// channel is created elsewhere or shared with a non-provider call
	/// site.
	pub fn FromClient(Client:Arc<AirClient>) -> Self {
		dev_log!("grpc", "[AirServiceProvider] Creating AirServiceProvider from existing client");

		Self { client:Client }
	}

	/// Shared reference to the underlying client.
	pub fn Client(&self) -> &Arc<AirClient> { &self.client }

	/// Whether the underlying client is connected.
	pub fn IsConnected(&self) -> bool { self.client.is_connected() }

	/// Address of the Air daemon.
	pub fn Address(&self) -> &str { self.client.address() }
}
