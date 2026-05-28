//! `AirServiceProvider::IndexFiles` - kick off a directory index pass on
//! the Air daemon. Wraps
//! [`crate::Client::AirClient::AirClient::IndexFiles`].

use crate::{
	AirError,
	Client::{AirClient::IndexInfo, AirServiceProvider::AirServiceProvider},
	dev_log,
};

impl AirServiceProvider {
	/// Indexes files under `path`. `patterns` includes globs to match;
	/// `exclude_patterns` filters those out. `max_depth` of `0` is
	/// unlimited.
	pub async fn IndexFiles(
		&self,

		path:String,

		patterns:Vec<String>,

		exclude_patterns:Vec<String>,

		max_depth:u32,
	) -> Result<IndexInfo::Struct, AirError> {
		let RequestID = crate::Utility::GenerateRequestId();

		dev_log!(
			"grpc",
			"[AirServiceProvider] IndexFiles (request_id: {}, path: {})",
			RequestID,
			path
		);

		self.client
			.IndexFiles(RequestID, path, patterns, exclude_patterns, max_depth)
			.await
	}
}
