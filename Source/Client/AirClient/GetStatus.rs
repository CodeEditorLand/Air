//! `AirClient::GetStatus` - fetches a wide snapshot of the Air daemon's
//! runtime status: version string, uptime, request counters,
//! response-time average, memory + CPU usage, in-flight request count.
//! Intended for status dashboards / health checks that need richer detail
//! than [`AirClient::HealthCheck`] returns.

use tonic::Request;

use crate::{
	AirError,
	Client::AirClient::{AirClient, AirStatus},
	Vine::Generated::air::StatusRequest,
	dev_log,
};

impl AirClient {
	/// Gets the Air daemon status snapshot.
	pub async fn GetStatus(&self, request_id:String) -> Result<AirStatus::Struct, AirError> {
		dev_log!("grpc", "[AirClient] Getting Air daemon status");

		let RequestPayload = StatusRequest { request_id };

		let Client = self
			.Client()
			.ok_or_else(|| AirError::Network("Air client not initialized".to_string()))?;

		let mut ClientGuard = Client.lock().await;

		match ClientGuard.get_status(Request::new(RequestPayload)).await {
			Ok(Response) => {
				let Response = Response.into_inner();

				dev_log!(
					"grpc",
					"[AirClient] Status retrieved. Active requests: {}",
					Response.active_requests
				);

				Ok(AirStatus::Struct {
					version:Response.version,
					uptime_seconds:Response.uptime_seconds,
					total_requests:Response.total_requests,
					successful_requests:Response.successful_requests,
					failed_requests:Response.failed_requests,
					average_response_time:Response.average_response_time,
					memory_usage_mb:Response.memory_usage_mb,
					cpu_usage_percent:Response.cpu_usage_percent,
					active_requests:Response.active_requests,
				})
			},

			Err(Status) => {
				dev_log!("grpc", "error: [AirClient] Get status RPC error: {}", Status);

				Err(AirError::Network(format!("Get status RPC error: {}", Status)))
			},
		}
	}
}
