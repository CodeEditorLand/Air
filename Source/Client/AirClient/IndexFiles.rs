//! `AirClient::IndexFiles` - drives the Air daemon's `IndexingService`
//! over a directory tree. Filters by include / exclude glob patterns and
//! bounds recursion depth so a stray request can't walk the entire
//! filesystem.

use tonic::Request;

use crate::{
	AirError,
	Client::AirClient::{AirClient, IndexInfo},
	Vine::Generated::air::IndexRequest,
	dev_log,
};

impl AirClient {
	/// Indexes files in a directory.
	///
	/// # Arguments
	///
	/// - `request_id` - opaque correlation id.
	/// - `path` - root directory.
	/// - `patterns` - glob include list (empty = include all).
	/// - `exclude_patterns` - glob exclude list.
	/// - `max_depth` - recursion bound; `0` indexes only `path` itself.
	pub async fn IndexFiles(
		&self,

		request_id:String,

		path:String,

		patterns:Vec<String>,

		exclude_patterns:Vec<String>,

		max_depth:u32,
	) -> Result<IndexInfo::Struct, AirError> {
		dev_log!("grpc", "[AirClient] Indexing files in: {}", path);

		let RequestPayload = IndexRequest { request_id, path, patterns, exclude_patterns, max_depth };

		let Client = self.Client().ok_or_else(|| AirError::Network("Air client not initialized".to_string()))?;

		let mut ClientGuard = Client.lock().await;

		match ClientGuard.index_files(Request::new(RequestPayload)).await {
			Ok(Response) => {
				let Response = Response.into_inner();

				dev_log!(
					"grpc",
					"[AirClient] Files indexed: {} (total size: {} bytes)",
					Response.files_indexed,
					Response.total_size
				);

				Ok(IndexInfo::Struct { files_indexed:Response.files_indexed, total_size:Response.total_size })
			},

			Err(Status) => {
				dev_log!("grpc", "error: [AirClient] Index files RPC error: {}", Status);

				Err(AirError::Network(format!("Index files RPC error: {}", Status)))
			},
		}
	}
}
