//! `AirServiceProvider::GetResourceUsage` - process resource counts.
//! Wraps [`crate::Client::AirClient::AirClient::GetResourceUsage`].

use crate::{
	AirError,
	Client::{
		AirClient::ResourceUsage,
		AirServiceProvider::{AirServiceProvider},
	},
	dev_log,
};

impl AirServiceProvider {
	/// Returns the daemon's current resource-usage snapshot.
	pub async fn GetResourceUsage(&self) -> Result<ResourceUsage::Struct, AirError> {
		let RequestID = crate::Utility::GenerateRequestId();

		dev_log!("grpc", "[AirServiceProvider] GetResourceUsage (request_id: {})", RequestID);

		self.client.GetResourceUsage(RequestID).await
	}
}
