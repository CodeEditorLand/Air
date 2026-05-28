//! `AirClient::CheckForUpdates` - queries the Air daemon's
//! `UpdateService` for available updates on the given channel.
//!
//! Maps the wire response into [`UpdateInfo::Struct`]. tonic transport /
//! status failures surface as [`AirError::Network`].

use tonic::Request;

use crate::{
	AirError,
	Client::AirClient::{AirClient, UpdateInfo},
	Vine::Generated::air::UpdateCheckRequest,
	dev_log,
};

impl AirClient {
	/// Checks for available updates.
	///
	/// # Arguments
	///
	/// - `request_id` - opaque correlation id.
	/// - `current_version` - the currently-running application version.
	/// - `channel` - update channel (`"stable"`, `"beta"`, `"nightly"`).
	pub async fn CheckForUpdates(
		&self,

		request_id:String,

		current_version:String,

		channel:String,
	) -> Result<UpdateInfo::Struct, AirError> {
		dev_log!("grpc", "[AirClient] Checking for updates for version '{}'", current_version);

		let RequestPayload = UpdateCheckRequest { request_id, current_version, channel };

		let Client = self.Client().ok_or_else(|| AirError::Network("Air client not initialized".to_string()))?;

		let mut ClientGuard = Client.lock().await;

		match ClientGuard.check_for_updates(Request::new(RequestPayload)).await {
			Ok(Response) => {
				let Response = Response.into_inner();

				dev_log!(
					"grpc",
					"[AirClient] Update check completed. Update available: {}",
					Response.update_available
				);

				Ok(UpdateInfo::Struct {
					update_available:Response.update_available,
					version:Response.version,
					download_url:Response.download_url,
					release_notes:Response.release_notes,
				})
			},

			Err(Status) => {
				dev_log!("grpc", "error: [AirClient] Check for updates RPC error: {}", Status);

				Err(AirError::Network(format!("Check for updates RPC error: {}", Status)))
			},
		}
	}
}
