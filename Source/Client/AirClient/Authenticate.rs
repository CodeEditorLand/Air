//! `AirClient::authenticate` - authenticates a user against the Air
//! daemon's `AuthenticationService` and returns a session token on
//! success.
//!
//! Translates the wire-level response:
//!
//! - `response.success == true` → `Ok(response.token)`
//! - `response.success == false` → [`AirError::Authentication`] carrying
//!   `response.error`
//! - tonic transport / status failure → [`AirError::Network`]
//!
//! The caller passes a `request_id`; this lets log lines + traces from
//! the daemon side be cross-referenced. Use
//! [`crate::Client::AirServiceProvider::GenerateRequestID::Fn`] to mint
//! one.

use tonic::Request;

use crate::{
	AirError,
	Client::AirClient::AirClient,
	Vine::Generated::air::AuthenticationRequest,
	dev_log,
};

impl AirClient {
	/// Authenticates a user with the Air daemon.
	///
	/// # Arguments
	///
	/// - `request_id` - opaque correlation id; see module docs.
	/// - `username` / `password` - credentials.
	/// - `provider` - authentication provider name (e.g. `"github"`,
	///   `"gitlab"`, `"microsoft"`).
	///
	/// # Returns
	///
	/// On `Ok`, the session token string from the daemon.
	pub async fn Authenticate(
		&self,

		request_id:String,

		username:String,

		password:String,

		provider:String,
	) -> Result<String, AirError> {
		dev_log!(
			"grpc",
			"[AirClient] Authenticating user '{}' with provider '{}'",
			username,
			provider
		);

		let UsernameDisplay = username.clone();

		let RequestPayload = AuthenticationRequest { request_id, username, password, provider };

		let Client = self.Client().ok_or_else(|| AirError::Network("Air client not initialized".to_string()))?;

		let mut ClientGuard = Client.lock().await;

		match ClientGuard.authenticate(Request::new(RequestPayload)).await {
			Ok(Response) => {
				let Response = Response.into_inner();

				if Response.success {
					dev_log!("grpc", "[AirClient] Authentication successful for user '{}'", UsernameDisplay);

					Ok(Response.token)
				} else {
					dev_log!(
						"grpc",
						"error: [AirClient] Authentication failed for user '{}': {}",
						UsernameDisplay,
						Response.error
					);

					Err(AirError::Authentication(Response.error))
				}
			},

			Err(Status) => {
				dev_log!("grpc", "error: [AirClient] Authentication RPC error: {}", Status);

				Err(AirError::Network(format!("Authentication RPC error: {}", Status)))
			},
		}
	}
}
