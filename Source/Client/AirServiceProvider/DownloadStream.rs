//! `AirServiceProvider::DownloadStream` - initiate a streaming download.
//! Wraps [`crate::Client::AirClient::AirClient::DownloadStream`] and
//! returns the stream wrapper so callers can pump chunks via
//! `.next().await`.

use std::collections::HashMap;

use crate::{
	AirError,
	Client::{
		AirClient::DownloadStream as DownloadStreamDTO,
		AirServiceProvider::{AirServiceProvider, GenerateRequestID},
	},
	dev_log,
};

impl AirServiceProvider {
	/// Starts a streaming download from `url`. The returned wrapper
	/// yields [`DownloadStreamDTO::Struct`] items until
	/// `chunk.completed == true`.
	pub async fn DownloadStream(
		&self,

		url:String,

		headers:HashMap<String, String>,
	) -> Result<DownloadStreamDTO::Struct, AirError> {
		let RequestID = GenerateRequestID::Fn();

		dev_log!(
			"grpc",
			"[AirServiceProvider] DownloadStream (request_id: {}, url: {})",
			RequestID,
			url
		);

		self.client.DownloadStream(RequestID, url, headers).await
	}
}
