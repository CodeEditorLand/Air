//! `AirServiceProvider::DownloadUpdate` - fetch an update package via the
//! Air daemon's `DownloaderService`. Wraps
//! [`crate::Client::AirClient::AirClient::DownloadUpdate`] and supplies
//! an empty header map by default.

use std::collections::HashMap;

use crate::{
	AirError,
	Client::{
		AirClient::FileInfo,
		AirServiceProvider::{AirServiceProvider},
	},
	dev_log,
};

impl AirServiceProvider {
	/// Downloads an update package to `destination_path`. The optional
	/// `checksum` is forwarded as-is; an empty string skips
	/// server-side verification.
	pub async fn DownloadUpdate(
		&self,

		url:String,

		destination_path:String,

		checksum:String,
	) -> Result<FileInfo::Struct, AirError> {
		let RequestID = crate::Utility::GenerateRequestId();

		dev_log!("grpc", "[AirServiceProvider] DownloadUpdate (request_id: {})", RequestID);

		self.client
			.DownloadUpdate(RequestID, url, destination_path, checksum, HashMap::new())
			.await
	}
}
