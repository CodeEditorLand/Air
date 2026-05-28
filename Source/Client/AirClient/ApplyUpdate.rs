//! `AirClient::ApplyUpdate` - applies a previously-downloaded update
//! package via the Air daemon's `UpdateService`.
//!
//! `response.success == true` → `Ok(())`. `response.success == false` →
//! [`AirError::Internal`] carrying the server-side error string. tonic
//! transport / status failures surface as [`AirError::Network`].

use tonic::Request;

use crate::{
	AirError,
	Client::AirClient::AirClient,
	Vine::Generated::air::ApplyUpdateRequest,
	dev_log,
};

impl AirClient {
	/// Applies an update package.
	///
	/// # Arguments
	///
	/// - `request_id` - opaque correlation id.
	/// - `version` - the version string of the update being applied.
	/// - `update_path` - filesystem path of the downloaded update bundle.
	pub async fn ApplyUpdate(&self, request_id:String, version:String, update_path:String) -> Result<(), AirError> {
		dev_log!("grpc", "[AirClient] Applying update version: {}", version);

		let RequestPayload = ApplyUpdateRequest { request_id, version, update_path };

		let Client = self.Client().ok_or_else(|| AirError::Network("Air client not initialized".to_string()))?;

		let mut ClientGuard = Client.lock().await;

		match ClientGuard.apply_update(Request::new(RequestPayload)).await {
			Ok(Response) => {
				let Response = Response.into_inner();

				if Response.success {
					dev_log!("grpc", "[AirClient] Update applied successfully");

					Ok(())
				} else {
					dev_log!("grpc", "error: [AirClient] Update application failed: {}", Response.error);

					Err(AirError::Internal(Response.error))
				}
			},

			Err(Status) => {
				dev_log!("grpc", "error: [AirClient] Apply update RPC error: {}", Status);

				Err(AirError::Network(format!("Apply update RPC error: {}", Status)))
			},
		}
	}
}
