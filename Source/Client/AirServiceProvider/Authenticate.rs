//! `AirServiceProvider::Authenticate` - authenticate a user with the Air
//! daemon. Mints a request id, forwards to
//! [`crate::Client::AirClient::AirClient::Authenticate`], and surfaces
//! the returned session token.

use crate::{AirError, Client::AirServiceProvider::AirServiceProvider, dev_log};

impl AirServiceProvider {
	/// Authenticates a user against the named provider (`"github"`,
	/// `"gitlab"`, `"microsoft"`, …) and returns the session token on
	/// success.
	pub async fn Authenticate(&self, username:String, password:String, provider:String) -> Result<String, AirError> {
		let RequestID = crate::Utility::GenerateRequestId();

		dev_log!("grpc", "[AirServiceProvider] Authenticate (request_id: {})", RequestID);

		self.client.Authenticate(RequestID, username, password, provider).await
	}
}
