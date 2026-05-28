//! `AirServiceProvider::GetMetrics` - retrieve daemon-side metrics by
//! optional type filter. Wraps
//! [`crate::Client::AirClient::AirClient::GetMetrics`].

use crate::{
	AirError,
	Client::{
		AirClient::AirMetrics,
		AirServiceProvider::{AirServiceProvider, GenerateRequestID},
	},
	dev_log,
};

impl AirServiceProvider {
	/// Fetches daemon metrics. `metric_type` of `None` returns all
	/// counters; common values are `"performance"`, `"resources"`,
	/// `"requests"`.
	pub async fn GetMetrics(&self, metric_type:Option<String>) -> Result<AirMetrics::Struct, AirError> {
		let RequestID = GenerateRequestID::Fn();

		dev_log!("grpc", "[AirServiceProvider] GetMetrics (request_id: {})", RequestID);

		self.client.GetMetrics(RequestID, metric_type).await
	}
}
