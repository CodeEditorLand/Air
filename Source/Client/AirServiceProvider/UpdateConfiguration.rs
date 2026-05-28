//! `AirServiceProvider::UpdateConfiguration` - patch a configuration
//! section on the daemon. Wraps
//! [`crate::Client::AirClient::AirClient::UpdateConfiguration`].

use std::collections::HashMap;

use crate::{
	AirError,
	Client::AirServiceProvider::{AirServiceProvider, GenerateRequestID},
	dev_log,
};

impl AirServiceProvider {
	/// Writes the key/value pairs in `updates` to the named
	/// configuration section. Keys not in `updates` are left
	/// untouched.
	pub async fn UpdateConfiguration(
		&self,

		section:String,

		updates:HashMap<String, String>,
	) -> Result<(), AirError> {
		let RequestID = GenerateRequestID::Fn();

		dev_log!(
			"grpc",
			"[AirServiceProvider] UpdateConfiguration (request_id: {}, section: {})",
			RequestID,
			section
		);

		self.client.UpdateConfiguration(RequestID, section, updates).await
	}
}
