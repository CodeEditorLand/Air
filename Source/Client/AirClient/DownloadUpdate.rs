//! `AirClient::DownloadUpdate` - downloads an update package to disk via
//! the Air daemon's `DownloaderService`. Uses the same `DownloadRequest`
//! wire shape as [`AirClient::DownloadFile`] but routes via the
//! `download_update` RPC so server-side metrics can attribute the bandwidth
//! to update operations.

use std::collections::HashMap;

use tonic::Request;

use crate::{
	AirError,
	Client::AirClient::{AirClient, FileInfo},
	Vine::Generated::air::DownloadRequest,
	dev_log,
};

impl AirClient {
	/// Downloads an update package.
	///
	/// # Arguments
	///
	/// - `request_id` - opaque correlation id.
	/// - `url` - HTTPS URL of the update package.
	/// - `destination_path` - local filesystem path the package writes to.
	/// - `checksum` - SHA-256 hex string; empty disables verification.
	/// - `headers` - extra HTTP headers (e.g. `"Authorization"`).
	pub async fn DownloadUpdate(
		&self,

		request_id:String,

		url:String,

		destination_path:String,

		checksum:String,

		headers:HashMap<String, String>,
	) -> Result<FileInfo::Struct, AirError> {
		dev_log!("grpc", "[AirClient] Downloading update from: {}", url);

		let RequestPayload = DownloadRequest { request_id, url, destination_path, checksum, headers };

		let Client = self
			.Client()
			.ok_or_else(|| AirError::Network("Air client not initialized".to_string()))?;

		let mut ClientGuard = Client.lock().await;

		match ClientGuard.download_update(Request::new(RequestPayload)).await {
			Ok(Response) => {
				let Response = Response.into_inner();

				if Response.success {
					dev_log!("grpc", "[AirClient] Update downloaded successfully to: {}", Response.file_path);

					Ok(FileInfo::Struct {
						file_path:Response.file_path,
						file_size:Response.file_size,
						checksum:Response.checksum,
					})
				} else {
					dev_log!("grpc", "error: [AirClient] Update download failed: {}", Response.error);

					Err(AirError::Network(Response.error))
				}
			},

			Err(Status) => {
				dev_log!("grpc", "error: [AirClient] Download update RPC error: {}", Status);

				Err(AirError::Network(format!("Download update RPC error: {}", Status)))
			},
		}
	}
}
