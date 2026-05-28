//! `AirClient::GetMetrics` - fetches a metrics snapshot from the Air
//! daemon. The wire response is a `HashMap<String, String>` so the daemon
//! can ship arbitrary keys; this method extracts the canonical numeric
//! fields ([`AirMetrics::Struct`]) by name and parses each as `f64`,
//! defaulting to `0.0` on missing / unparseable entries.

use tonic::Request;

use crate::{
	AirError,
	Client::AirClient::{AirClient, AirMetrics},
	Vine::Generated::air::MetricsRequest,
	dev_log,
};

impl AirClient {
	/// Gets metrics from the Air daemon.
	///
	/// # Arguments
	///
	/// - `request_id` - opaque correlation id.
	/// - `metric_type` - optional filter (`"performance"` / `"resources"` /
	///   `"requests"`). `None` requests the full metric set.
	pub async fn GetMetrics(
		&self,

		request_id:String,

		metric_type:Option<String>,
	) -> Result<AirMetrics::Struct, AirError> {
		dev_log!("grpc", "[AirClient] Getting metrics (type: {:?})", metric_type.as_deref());

		let RequestPayload = MetricsRequest { request_id, metric_type:metric_type.unwrap_or_default() };

		let Client = self
			.Client()
			.ok_or_else(|| AirError::Network("Air client not initialized".to_string()))?;

		let mut ClientGuard = Client.lock().await;

		match ClientGuard.get_metrics(Request::new(RequestPayload)).await {
			Ok(Response) => {
				let Response = Response.into_inner();

				dev_log!("grpc", "[AirClient] Metrics retrieved");

				let Metrics = AirMetrics::Struct {
					memory_usage_mb:Response
						.metrics
						.get("memory_usage_mb")
						.and_then(|S| S.parse::<f64>().ok())
						.unwrap_or(0.0),

					cpu_usage_percent:Response
						.metrics
						.get("cpu_usage_percent")
						.and_then(|S| S.parse::<f64>().ok())
						.unwrap_or(0.0),

					network_usage_mbps:Response
						.metrics
						.get("network_usage_mbps")
						.and_then(|S| S.parse::<f64>().ok())
						.unwrap_or(0.0),

					disk_usage_mb:Response
						.metrics
						.get("disk_usage_mb")
						.and_then(|S| S.parse::<f64>().ok())
						.unwrap_or(0.0),

					average_response_time:Response
						.metrics
						.get("average_response_time")
						.and_then(|S| S.parse::<f64>().ok())
						.unwrap_or(0.0),
				};

				Ok(Metrics)
			},

			Err(Status) => {
				dev_log!("grpc", "error: [AirClient] Get metrics RPC error: {}", Status);

				Err(AirError::Network(format!("Get metrics RPC error: {}", Status)))
			},
		}
	}
}
