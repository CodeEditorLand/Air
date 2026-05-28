//! `AirServiceProvider::UpdateConfiguration` - patch a configuration
//! section on the daemon. Wraps
//! [`crate::Client::AirClient::AirClient::UpdateConfiguration`].

use std::collections::HashMap;

use crate::{
	AirError,
	Client::AirServiceProvider::{AirServiceProvider},
	dev_log,
};

impl AirServiceProvider {
	/// Writes the key/value pairs in `updates` to the named
	/// configuration section. Keys not in `updates` are left
	/// untouched.
	pub async fn UpdateConfiguration(&self, section:String, updates:HashMap<String, String>) -> Result<(), AirError> {
		let RequestID = crate::Utility::GenerateRequestId();

		dev_log!(
			"grpc",
			"[AirServiceProvider] UpdateConfiguration (request_id: {}, section: {})",
			RequestID,
			section
		);

		self.client.UpdateConfiguration(RequestID, section, updates).await
	}
}
