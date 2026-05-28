//! `AirClient::DownloadStream` - initiates a streaming download from the
//! Air daemon's `DownloaderService` and returns a [`DownloadStream::Struct`]
//! that yields chunks via `.next().await`.
//!
//! Unlike [`AirClient::DownloadFile`], the gRPC call returns immediately
//! after the server accepts the request; bytes flow as
//! [`DownloadStreamChunk::Struct`] items the caller pumps until
//! `chunk.completed == true`. Suitable for large files where the caller
//! wants to surface progress or stream into a sink without an intermediate
//! `Vec<u8>` buffer.

use std::collections::HashMap;

use tonic::Request;

use crate::{
	AirError,
	Client::AirClient::{AirClient, DownloadStream},
	Vine::Generated::air::DownloadStreamRequest,
	dev_log,
};

impl AirClient {
	/// Starts a streaming download.
	///
	/// # Arguments
	///
	/// - `request_id` - opaque correlation id.
	/// - `url` - HTTPS URL of the file.
	/// - `headers` - extra HTTP headers (e.g. `"Authorization"`).
	pub async fn DownloadStream(
		&self,

		request_id:String,

		url:String,

		headers:HashMap<String, String>,
	) -> Result<DownloadStream::Struct, AirError> {
		dev_log!("grpc", "[AirClient] Starting stream download from: {}", url);

		let RequestPayload = DownloadStreamRequest { request_id, url, headers };

		let Client = self.Client().ok_or_else(|| AirError::Network("Air client not initialized".to_string()))?;

		let mut ClientGuard = Client.lock().await;

		match ClientGuard.download_stream(Request::new(RequestPayload)).await {
			Ok(Response) => {
				dev_log!("grpc", "[AirClient] Stream download initiated successfully");

				Ok(DownloadStream::Struct::new(Response.into_inner()))
			},

			Err(Status) => {
				dev_log!("grpc", "error: [AirClient] Download stream RPC error: {}", Status);

				Err(AirError::Network(format!("Download stream RPC error: {}", Status)))
			},
		}
	}
}
