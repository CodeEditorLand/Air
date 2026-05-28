//! `AirServiceProvider::Authenticate` - authenticate a user with the Air
//! daemon. Mints a request id, forwards to
//! [`crate::Client::AirClient::AirClient::Authenticate`], and surfaces
//! the returned session token.

use crate::{
	AirError,
	Client::AirServiceProvider::{AirServiceProvider, GenerateRequestID},
	dev_log,
};

impl AirServiceProvider {
	/// Authenticates a user against the named provider (`"github"`,
	/// `"gitlab"`, `"microsoft"`, …) and returns the session token on
	/// success.
	pub async fn Authenticate(&self, username:String, password:String, provider:String) -> Result<String, AirError> {
		let RequestID = GenerateRequestID::Fn();

		dev_log!("grpc", "[AirServiceProvider] Authenticate (request_id: {})", RequestID);

		self.client.Authenticate(RequestID, username, password, provider).await
	}
}
