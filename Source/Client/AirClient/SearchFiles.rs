//! `AirClient::SearchFiles` - issues a content / path search against the
//! Air daemon's `IndexingService` index.
//!
//! Returns an empty vector on success: the underlying
//! `SearchFilesResponse` does not yet carry per-hit detail in the live
//! proto, so the response is structural-only and downstream callers that
//! need hits should consume `IndexInfo::Struct` or wait for the schema to
//! grow per-hit fields.

use tonic::Request;

use crate::{
	AirError,
	Client::AirClient::{AirClient, FileResult},
	Vine::Generated::air::SearchRequest,
	dev_log,
};

impl AirClient {
	/// Searches the index for files matching `query`.
	///
	/// # Arguments
	///
	/// - `request_id` - opaque correlation id.
	/// - `query` - search expression (provider-defined syntax).
	/// - `path` - root path to scope the search; empty = whole index.
	/// - `max_results` - hard cap on hits returned.
	pub async fn SearchFiles(
		&self,

		request_id:String,

		query:String,

		path:String,

		max_results:u32,
	) -> Result<Vec<FileResult::Struct>, AirError> {
		dev_log!("grpc", "[AirClient] Searching for files with query: '{}' in: {}", query, path);

		let RequestPayload = SearchRequest { request_id, query, path, max_results };

		let Client = self
			.Client()
			.ok_or_else(|| AirError::Network("Air client not initialized".to_string()))?;

		let mut ClientGuard = Client.lock().await;

		match ClientGuard.search_files(Request::new(RequestPayload)).await {
			Ok(_Response) => {
				dev_log!("grpc", "[AirClient] Search completed");

				// SearchFilesResponse does not carry per-hit detail yet.
				Ok(Vec::new())
			},

			Err(Status) => {
				dev_log!("grpc", "error: [AirClient] Search files RPC error: {}", Status);

				Err(AirError::Network(format!("Search files RPC error: {}", Status)))
			},
		}
	}
}
