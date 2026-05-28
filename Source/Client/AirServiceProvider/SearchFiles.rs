//! `AirServiceProvider::SearchFiles` - query Air's full-text index.
//! Wraps [`crate::Client::AirClient::AirClient::SearchFiles`].

use crate::{
	AirError,
	Client::{
		AirClient::FileResult,
		AirServiceProvider::{AirServiceProvider, GenerateRequestID},
	},
	dev_log,
};

impl AirServiceProvider {
	/// Searches for `query` under `path`. `max_results` of `0` is
	/// unlimited.
	pub async fn SearchFiles(
		&self,

		query:String,

		path:String,

		max_results:u32,
	) -> Result<Vec<FileResult::Struct>, AirError> {
		let RequestID = GenerateRequestID::Fn();

		dev_log!(
			"grpc",
			"[AirServiceProvider] SearchFiles (request_id: {}, query: {})",
			RequestID,
			query
		);

		self.client.SearchFiles(RequestID, query, path, max_results).await
	}
}
