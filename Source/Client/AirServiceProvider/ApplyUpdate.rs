//! `AirServiceProvider::ApplyUpdate` - tell the Air daemon to install a
//! previously downloaded update. Wraps
//! [`crate::Client::AirClient::AirClient::ApplyUpdate`].

use crate::{
	AirError,
	Client::AirServiceProvider::{AirServiceProvider, GenerateRequestID},
	dev_log,
};

impl AirServiceProvider {
	/// Applies the update package at `update_path` and tags it with
	/// `version` for the daemon's bookkeeping.
	pub async fn ApplyUpdate(&self, version:String, update_path:String) -> Result<(), AirError> {
		let RequestID = GenerateRequestID::Fn();

		dev_log!("grpc", "[AirServiceProvider] ApplyUpdate (request_id: {})", RequestID);

		self.client.ApplyUpdate(RequestID, version, update_path).await
	}
}
