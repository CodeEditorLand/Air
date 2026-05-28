//! `AirClient::HealthCheck` - pings the Air daemon and returns whether it
//! reports itself healthy. Lightweight liveness probe; for runtime detail
//! use [`AirClient::GetStatus`].

use tonic::Request;

use crate::{
	AirError,
	Client::AirClient::AirClient,
	Vine::Generated::air::HealthCheckRequest,
	dev_log,
};

impl AirClient {
	/// Performs a health check on the Air daemon.
	pub async fn HealthCheck(&self) -> Result<bool, AirError> {
		dev_log!("grpc", "[AirClient] Performing health check");

		let RequestPayload = HealthCheckRequest {};

		let Client = self.Client().ok_or_else(|| AirError::Network("Air client not initialized".to_string()))?;

		let mut ClientGuard = Client.lock().await;

		match ClientGuard.health_check(Request::new(RequestPayload)).await {
			Ok(Response) => {
				let Response = Response.into_inner();

				dev_log!("grpc", "[AirClient] Health check result: {}", Response.healthy);

				Ok(Response.healthy)
			},

			Err(Status) => {
				dev_log!("grpc", "error: [AirClient] Health check RPC error: {}", Status);

				Err(AirError::Network(format!("Health check RPC error: {}", Status)))
			},
		}
	}
}
