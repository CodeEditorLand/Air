//! `AirClient::GetResourceUsage` - fetches a structured resource snapshot
//! from the Air daemon (memory, CPU, disk, network).
//!
//! `thread_count` and `open_file_handles` default to `0` because the
//! daemon's `ResourceUsageResponse` proto does not yet carry those
//! fields; populate them on the daemon side when needed.

use tonic::Request;

use crate::{
	AirError,
	Client::AirClient::{AirClient, ResourceUsage},
	Vine::Generated::air::ResourceUsageRequest,
	dev_log,
};

impl AirClient {
	/// Gets daemon resource-usage stats.
	pub async fn GetResourceUsage(&self, request_id:String) -> Result<ResourceUsage::Struct, AirError> {
		dev_log!("grpc", "[AirClient] Getting resource usage");

		let RequestPayload = ResourceUsageRequest { request_id };

		let Client = self
			.Client()
			.ok_or_else(|| AirError::Network("Air client not initialized".to_string()))?;

		let mut ClientGuard = Client.lock().await;

		match ClientGuard.get_resource_usage(Request::new(RequestPayload)).await {
			Ok(Response) => {
				let Response = Response.into_inner();

				dev_log!("grpc", "[AirClient] Resource usage retrieved");

				Ok(ResourceUsage::Struct {
					memory_usage_mb:Response.memory_usage_mb,
					cpu_usage_percent:Response.cpu_usage_percent,
					disk_usage_mb:Response.disk_usage_mb,
					network_usage_mbps:Response.network_usage_mbps,
					thread_count:0,
					open_file_handles:0,
				})
			},

			Err(Status) => {
				dev_log!("grpc", "error: [AirClient] Get resource usage RPC error: {}", Status);

				Err(AirError::Network(format!("Get resource usage RPC error: {}", Status)))
			},
		}
	}
}
