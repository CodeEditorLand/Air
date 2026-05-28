//! `AirServiceProvider::CheckForUpdates` - probe the Air daemon for an
//! available update. Wraps
//! [`crate::Client::AirClient::AirClient::CheckForUpdates`] and collapses
//! `update_available == false` into `Ok(None)` so callers can pattern-
//! match `Some(info)` for the live-update case.

use crate::{
	AirError,
	Client::{
		AirClient::UpdateInfo,
		AirServiceProvider::{AirServiceProvider, GenerateRequestID},
	},
	dev_log,
};

impl AirServiceProvider {
	/// Checks for an available update on the given channel
	/// (`"stable"` / `"beta"` / `"nightly"`).
	///
	/// Returns `Ok(Some(info))` when an update is offered and
	/// `Ok(None)` when the daemon reports no update.
	pub async fn CheckForUpdates(
		&self,

		current_version:String,

		channel:String,
	) -> Result<Option<UpdateInfo::Struct>, AirError> {
		let RequestID = GenerateRequestID::Fn();

		dev_log!("grpc", "[AirServiceProvider] CheckForUpdates (request_id: {})", RequestID);

		let Info = self.client.CheckForUpdates(RequestID, current_version, channel).await?;

		if Info.update_available { Ok(Some(Info)) } else { Ok(None) }
	}
}
