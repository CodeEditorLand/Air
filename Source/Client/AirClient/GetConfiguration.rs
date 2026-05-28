//! `AirClient::GetConfiguration` - reads back the daemon's current
//! configuration for a named section.
//!
//! Sections are daemon-defined; canonical names include `"grpc"`,
//! `"authentication"`, `"updates"`. Returns the raw key → value map; the
//! caller parses individual entries.

use std::collections::HashMap;

use tonic::Request;

use crate::{
	AirError,
	Client::AirClient::AirClient,
	Vine::Generated::air::ConfigurationRequest,
	dev_log,
};

impl AirClient {
	/// Reads a configuration section from the daemon.
	pub async fn GetConfiguration(
		&self,

		request_id:String,

		section:String,
	) -> Result<HashMap<String, String>, AirError> {
		let SectionDisplay = section.clone();

		dev_log!("grpc", "[AirClient] Getting configuration for section: {}", section);

		let RequestPayload = ConfigurationRequest { request_id, section };

		let Client = self.Client().ok_or_else(|| AirError::Network("Air client not initialized".to_string()))?;

		let mut ClientGuard = Client.lock().await;

		match ClientGuard.get_configuration(Request::new(RequestPayload)).await {
			Ok(Response) => {
				let Response = Response.into_inner();

				dev_log!(
					"grpc",
					"[AirClient] Configuration retrieved for section: {} ({} keys)",
					SectionDisplay,
					Response.configuration.len()
				);

				Ok(Response.configuration)
			},

			Err(Status) => {
				dev_log!("grpc", "error: [AirClient] Get configuration RPC error: {}", Status);

				Err(AirError::Network(format!("Get configuration RPC error: {}", Status)))
			},
		}
	}
}
