//! `AirClient::DownloadFile` - downloads a single file via the Air
//! daemon's `DownloaderService` and writes it to a local filesystem path.
//!
//! Server-side handles HTTP-layer retries, redirects, and resume; the
//! gRPC call returns only after the local file is fully written (or the
//! download fails). For incremental delivery see
//! [`AirClient::DownloadStream`].

use std::collections::HashMap;

use tonic::Request;

use crate::{
	AirError,
	Client::AirClient::{AirClient, FileInfo},
	Vine::Generated::air::DownloadRequest,
	dev_log,
};

impl AirClient {
	/// Downloads a file.
	///
	/// # Arguments
	///
	/// - `request_id` - opaque correlation id.
	/// - `url` - HTTPS URL of the file.
	/// - `destination_path` - local filesystem path the file writes to.
	/// - `checksum` - SHA-256 hex string; empty disables verification.
	/// - `headers` - extra HTTP headers.
	pub async fn DownloadFile(
		&self,

		request_id:String,

		url:String,

		destination_path:String,

		checksum:String,

		headers:HashMap<String, String>,
	) -> Result<FileInfo::Struct, AirError> {
		dev_log!("grpc", "[AirClient] Downloading file from: {}", url);

		let RequestPayload = DownloadRequest { request_id, url, destination_path, checksum, headers };

		let Client = self
			.Client()
			.ok_or_else(|| AirError::Network("Air client not initialized".to_string()))?;

		let mut ClientGuard = Client.lock().await;

		match ClientGuard.download_file(Request::new(RequestPayload)).await {
			Ok(Response) => {
				let Response = Response.into_inner();

				if Response.success {
					dev_log!("grpc", "[AirClient] File downloaded successfully to: {}", Response.file_path);

					Ok(FileInfo::Struct {
						file_path:Response.file_path,
						file_size:Response.file_size,
						checksum:Response.checksum,
					})
				} else {
					dev_log!("grpc", "error: [AirClient] File download failed: {}", Response.error);

					Err(AirError::Network(Response.error))
				}
			},

			Err(Status) => {
				dev_log!("grpc", "error: [AirClient] Download file RPC error: {}", Status);

				Err(AirError::Network(format!("Download file RPC error: {}", Status)))
			},
		}
	}
}
