//! `AirServiceProvider::DownloadFile` - generic URL download routed
//! through Air. Wraps
//! [`crate::Client::AirClient::AirClient::DownloadFile`] with an empty
//! header map.

use std::collections::HashMap;

use crate::{
	AirError,
	Client::{AirClient::FileInfo, AirServiceProvider::AirServiceProvider},
	dev_log,
};

impl AirServiceProvider {
	/// Downloads `url` to `destination_path`. The optional `checksum`
	/// is forwarded for server-side verification; an empty string
	/// skips verification.
	pub async fn DownloadFile(
		&self,

		url:String,

		destination_path:String,

		checksum:String,
	) -> Result<FileInfo::Struct, AirError> {
		let RequestID = crate::Utility::GenerateRequestId();

		dev_log!("grpc", "[AirServiceProvider] DownloadFile (request_id: {})", RequestID);

		self.client
			.DownloadFile(RequestID, url, destination_path, checksum, HashMap::new())
			.await
	}
}
