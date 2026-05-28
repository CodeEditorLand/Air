//! `AirServiceProvider::SetResourceLimits` - constrain the daemon's
//! resource budget. Wraps
//! [`crate::Client::AirClient::AirClient::SetResourceLimits`].

use crate::{AirError, Client::AirServiceProvider::AirServiceProvider, dev_log};

impl AirServiceProvider {
	/// Sets memory / CPU / disk caps on the daemon.
	///
	/// - `memory_limit_mb` - memory budget in MB
	/// - `cpu_limit_percent` - 0-100
	/// - `disk_limit_mb` - disk budget in MB
	pub async fn SetResourceLimits(
		&self,

		memory_limit_mb:u32,

		cpu_limit_percent:u32,

		disk_limit_mb:u32,
	) -> Result<(), AirError> {
		let RequestID = crate::Utility::GenerateRequestId();

		dev_log!("grpc", "[AirServiceProvider] SetResourceLimits (request_id: {})", RequestID);

		self.client
			.SetResourceLimits(RequestID, memory_limit_mb, cpu_limit_percent, disk_limit_mb)
			.await
	}
}
