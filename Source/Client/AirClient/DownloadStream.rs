//! Wrapper for an asynchronous Air download stream.
//!
//! Adapts the tonic streaming API into a `next().await` iterator that
//! yields [`DownloadStreamChunk::Struct`] items. Stream failures (HTTP/2
//! frame errors, mid-stream disconnect) surface as [`AirError::Network`].

use crate::{AirError, Client::AirClient::DownloadStreamChunk, Vine::Generated::air::DownloadStreamResponse, dev_log};

pub struct Struct {
	inner:tonic::codec::Streaming<DownloadStreamResponse>,
}

impl Struct {
	pub fn new(Stream:tonic::codec::Streaming<DownloadStreamResponse>) -> Self { Self { inner:Stream } }

	/// Returns the next chunk from the stream. `None` when the stream ends.
	pub async fn next(&mut self) -> Option<Result<DownloadStreamChunk::Struct, AirError>> {
		match futures_util::stream::StreamExt::next(&mut self.inner).await {
			Some(Ok(Response)) => {
				Some(Ok(DownloadStreamChunk::Struct {
					data:Response.chunk,
					total_size:Response.total_size,
					downloaded:Response.downloaded,
					completed:Response.completed,
					error:Response.error,
				}))
			},

			Some(Err(Error)) => {
				dev_log!("grpc", "error: [DownloadStream] Stream error: {}", Error);

				Some(Err(AirError::Network(format!("Stream error: {}", Error))))
			},

			None => None,
		}
	}
}
