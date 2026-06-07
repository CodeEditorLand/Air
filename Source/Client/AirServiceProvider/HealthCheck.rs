//! `AirServiceProvider::HealthCheck` - liveness probe for the Air daemon.
//! Wraps [`crate::Client::AirClient::AirClient::HealthCheck`].

use crate::{AirError, Client::AirServiceProvider::AirServiceProvider, dev_log};

impl AirServiceProvider {

	/// Returns `true` when the daemon reports healthy. The gRPC call
	/// does not take a request id - the health check is intentionally
	/// uncorrelated so it stays cheap.
	pub async fn HealthCheck(&self) -> Result<bool, AirError> {
		dev_log!("grpc", "[AirServiceProvider] HealthCheck");

		self.client.HealthCheck().await
	}
}
