//! `AirClient::GetFileInfo` - fetches extended metadata for a single
//! filesystem path from the Air daemon. Returns size, MIME type, SHA-256
//! checksum, modification time, and an `exists` flag (false when the path
//! is missing on the daemon side).

use tonic::Request;

use crate::{
	AirError,
	Client::AirClient::{AirClient, ExtendedFileInfo},
	Vine::Generated::air::FileInfoRequest,
	dev_log,
};

impl AirClient {
	/// Gets extended file information.
	pub async fn GetFileInfo(&self, request_id:String, path:String) -> Result<ExtendedFileInfo::Struct, AirError> {
		let PathDisplay = path.clone();

		dev_log!("grpc", "[AirClient] Getting file info for: {}", path);

		let RequestPayload = FileInfoRequest { request_id, path };

		let Client = self
			.Client()
			.ok_or_else(|| AirError::Network("Air client not initialized".to_string()))?;

		let mut ClientGuard = Client.lock().await;

		match ClientGuard.get_file_info(Request::new(RequestPayload)).await {
			Ok(Response) => {
				let Response = Response.into_inner();

				dev_log!(
					"grpc",
					"[AirClient] File info retrieved for: {} (exists: {})",
					PathDisplay,
					Response.exists
				);

				Ok(ExtendedFileInfo::Struct {
					exists:Response.exists,
					size:Response.size,
					mime_type:Response.mime_type,
					checksum:Response.checksum,
					modified_time:Response.modified_time,
				})
			},

			Err(Status) => {
				dev_log!("grpc", "error: [AirClient] Get file info RPC error: {}", Status);

				Err(AirError::Network(format!("Get file info RPC error: {}", Status)))
			},
		}
	}
}
