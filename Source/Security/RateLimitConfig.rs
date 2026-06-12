use serde::{Deserialize, Serialize};

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
	/// Requests per second per IP
	pub requests_per_second_ip:u32,

	/// Requests per second per client
	pub requests_per_second_client:u32,

	/// Burst capacity (tokens)
	pub burst_capacity:u32,

	/// Token refill interval in milliseconds
	pub refill_interval_ms:u64,
}

impl Default for RateLimitConfig {
	fn default() -> Self {
		Self {
			requests_per_second_ip:100,

			requests_per_second_client:50,

			burst_capacity:200,

			refill_interval_ms:100,
		}
	}
}
