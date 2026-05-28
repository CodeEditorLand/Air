//! `AirServiceProvider::GetStatus` - snapshot of the Air daemon's
//! uptime / request counters. Wraps
//! [`crate::Client::AirClient::AirClient::GetStatus`].

use crate::{
	AirError,
	Client::{
		AirClient::AirStatus,
		AirServiceProvider::{AirServiceProvider, GenerateRequestID},
	},
	dev_log,
};

impl AirServiceProvider {
	/// Fetches the daemon's runtime status snapshot.
	pub async fn GetStatus(&self) -> Result<AirStatus::Struct, AirError> {
		let RequestID = GenerateRequestID::Fn();

		dev_log!("grpc", "[AirServiceProvider] GetStatus (request_id: {})", RequestID);

		self.client.GetStatus(RequestID).await
	}
}
