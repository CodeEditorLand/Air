//! Wrapper for an asynchronous Air download stream. Adapts the tonic
//! streaming API into a `next().await` iterator that yields
//! [`DownloadStreamChunk::Struct`] items.
//!
//! Synthesised from `Mountain/Source/Air/AirClient/DownloadStream.rs`.
//! Path-only differences from Mountain:
//!
//! - `AirLibrary::Vine::Generated::air::DownloadStreamResponse` →
//!   `crate::Vine::Generated::air::DownloadStreamResponse` (Air owns the
//!   generated proto natively, no cross-crate hop needed).
//! - `CommonLibrary::Error::CommonError` → [`crate::AirError`] - Air's own
//!   canonical error type. The `IPCError { Description }` wrap maps onto
//!   `AirError::Network(String)` since stream failures are transport-level
//!   (HTTP/2 frame errors, server disconnect).
//! - `crate::Air::AirClient::DownloadStreamChunk` →
//!   [`crate::Client::AirClient::DownloadStreamChunk`].
//! - The Mountain version is `#[cfg(feature = "AirIntegration")]`-gated because
//!   it lives in Mountain and needs the optional Air dep. Air itself always has
//!   these types, so no cfg gate needed.

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
