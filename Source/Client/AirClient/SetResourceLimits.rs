//! `AirClient::SetResourceLimits` - asks the Air daemon to enforce
//! ceilings on its own resource usage. Daemon-side enforcement is
//! advisory; OS-level cgroups still bound the process.

use tonic::Request;

use crate::{
	AirError,
	Client::AirClient::AirClient,
	Vine::Generated::air::ResourceLimitsRequest,
	dev_log,
};

impl AirClient {
	/// Sets daemon resource ceilings.
	///
	/// # Arguments
	///
	/// - `request_id` - opaque correlation id.
	/// - `memory_limit_mb` - max resident memory.
	/// - `cpu_limit_percent` - max CPU utilisation (whole percent).
	/// - `disk_limit_mb` - max disk-usage budget.
	pub async fn SetResourceLimits(
		&self,

		request_id:String,

		memory_limit_mb:u32,

		cpu_limit_percent:u32,

		disk_limit_mb:u32,
	) -> Result<(), AirError> {
		dev_log!(
			"grpc",
			"[AirClient] Setting resource limits: memory={}MB, cpu={}%, disk={}MB",
			memory_limit_mb,
			cpu_limit_percent,
			disk_limit_mb
		);

		let RequestPayload = ResourceLimitsRequest { request_id, memory_limit_mb, cpu_limit_percent, disk_limit_mb };

		let Client = self.Client().ok_or_else(|| AirError::Network("Air client not initialized".to_string()))?;

		let mut ClientGuard = Client.lock().await;

		match ClientGuard.set_resource_limits(Request::new(RequestPayload)).await {
			Ok(Response) => {
				let Response = Response.into_inner();

				if Response.success {
					dev_log!("grpc", "[AirClient] Resource limits set successfully");

					Ok(())
				} else {
					dev_log!("grpc", "error: [AirClient] Failed to set resource limits: {}", Response.error);

					Err(AirError::ResourceLimit(Response.error))
				}
			},

			Err(Status) => {
				dev_log!("grpc", "error: [AirClient] Set resource limits RPC error: {}", Status);

				Err(AirError::Network(format!("Set resource limits RPC error: {}", Status)))
			},
		}
	}
}
