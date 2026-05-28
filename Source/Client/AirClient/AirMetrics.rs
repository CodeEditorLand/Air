//! Metrics from the Air daemon.

#[derive(Debug, Clone)]
pub struct Struct {
	pub memory_usage_mb:f64,

	pub cpu_usage_percent:f64,

	pub network_usage_mbps:f64,

	pub disk_usage_mb:f64,

	pub average_response_time:f64,
}
