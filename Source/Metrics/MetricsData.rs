use serde::{Deserialize, Serialize};

/// Structured metrics data for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsData {
	pub timestamp:u64,

	pub requests_total:u64,

	pub requests_successful:u64,

	pub requests_failed:u64,

	pub errors_total:u64,

	pub memory_bytes:u64,

	pub cpu_percent:f64,

	pub active_connections:u64,

	pub active_threads:u64,

	pub authentication_operations:u64,

	pub authentication_failures:u64,

	pub downloads_total:u64,

	pub downloads_completed:u64,

	pub downloads_failed:u64,

	pub downloads_bytes:u64,

	pub indexing_operations:u64,

	pub indexing_entries:u64,

	pub updates_checked:u64,

	pub updates_applied:u64,

	pub latency_avg_ms:f64,

	pub latency_min_ms:u64,

	pub latency_max_ms:u64,
}

impl MetricsData {
	/// Calculate success rate as percentage
	pub fn SuccessRate(&self) -> f64 {
		if self.requests_total == 0 {
			return 100.0;
		}

		(self.requests_successful as f64 / self.requests_total as f64) * 100.0
	}

	/// Calculate download success rate
	pub fn DownloadSuccessRate(&self) -> f64 {
		if self.downloads_total == 0 {
			return 100.0;
		}

		(self.downloads_completed as f64 / self.downloads_total as f64) * 100.0
	}

	/// Calculate error rate
	pub fn ErrorRate(&self) -> f64 {
		if self.requests_total == 0 {
			return 0.0;
		}

		(self.errors_total as f64 / self.requests_total as f64) * 100.0
	}
}
