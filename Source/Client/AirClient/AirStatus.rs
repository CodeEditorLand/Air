//! Status of the Air daemon.

#[derive(Debug, Clone)]
pub struct Struct {
	pub version:String,

	pub uptime_seconds:u64,

	pub total_requests:u64,

	pub successful_requests:u64,

	pub failed_requests:u64,

	pub average_response_time:f64,

	pub memory_usage_mb:f64,

	pub cpu_usage_percent:f64,

	pub active_requests:u32,
}
