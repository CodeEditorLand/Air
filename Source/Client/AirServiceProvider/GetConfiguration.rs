//! `AirServiceProvider::GetConfiguration` - read a configuration section
//! from the daemon. Wraps
//! [`crate::Client::AirClient::AirClient::GetConfiguration`].

use std::collections::HashMap;

use crate::{AirError, Client::AirServiceProvider::AirServiceProvider, dev_log};

impl AirServiceProvider {
	/// Reads a configuration section as a key/value map. Common
	/// sections: `"grpc"`, `"authentication"`, `"updates"`.
	pub async fn GetConfiguration(&self, section:String) -> Result<HashMap<String, String>, AirError> {
		let RequestID = crate::Utility::GenerateRequestId();

		dev_log!(
			"grpc",
			"[AirServiceProvider] GetConfiguration (request_id: {}, section: {})",
			RequestID,
			section
		);

		self.client.GetConfiguration(RequestID, section).await
	}
}
