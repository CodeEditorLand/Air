//! `AirServiceProvider::GetFileInfo` - retrieve extended file metadata.
//! Wraps [`crate::Client::AirClient::AirClient::GetFileInfo`].

use crate::{
	AirError,
	Client::{AirClient::ExtendedFileInfo, AirServiceProvider::AirServiceProvider},
	dev_log,
};

impl AirServiceProvider {
	/// Returns metadata for the file at `path` (size, mime type,
	/// checksum, modified time).
	pub async fn GetFileInfo(&self, path:String) -> Result<ExtendedFileInfo::Struct, AirError> {
		let RequestID = crate::Utility::GenerateRequestId();

		dev_log!(
			"grpc",
			"[AirServiceProvider] GetFileInfo (request_id: {}, path: {})",
			RequestID,
			path
		);

		self.client.GetFileInfo(RequestID, path).await
	}
}
