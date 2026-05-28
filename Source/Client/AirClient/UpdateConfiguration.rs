//! `AirClient::UpdateConfiguration` - applies a partial configuration
//! patch to the named section. Sends only the keys the caller wants to
//! change; the daemon merges over the existing section.

use std::collections::HashMap;

use tonic::Request;

use crate::{AirError, Client::AirClient::AirClient, Vine::Generated::air::UpdateConfigurationRequest, dev_log};

impl AirClient {
	/// Updates configuration for the given section.
	pub async fn UpdateConfiguration(
		&self,

		request_id:String,

		section:String,

		updates:HashMap<String, String>,
	) -> Result<(), AirError> {
		let SectionDisplay = section.clone();

		dev_log!(
			"grpc",
			"[AirClient] Updating configuration for section: {} ({} keys)",
			SectionDisplay,
			updates.len()
		);

		let RequestPayload = UpdateConfigurationRequest { request_id, section, updates };

		let Client = self
			.Client()
			.ok_or_else(|| AirError::Network("Air client not initialized".to_string()))?;

		let mut ClientGuard = Client.lock().await;

		match ClientGuard.update_configuration(Request::new(RequestPayload)).await {
			Ok(Response) => {
				let Response = Response.into_inner();

				if Response.success {
					dev_log!(
						"grpc",
						"[AirClient] Configuration updated successfully for section: {}",
						SectionDisplay
					);

					Ok(())
				} else {
					dev_log!("grpc", "error: [AirClient] Failed to update configuration: {}", Response.error);

					Err(AirError::Configuration(Response.error))
				}
			},

			Err(Status) => {
				dev_log!("grpc", "error: [AirClient] Update configuration RPC error: {}", Status);

				Err(AirError::Network(format!("Update configuration RPC error: {}", Status)))
			},
		}
	}
}
