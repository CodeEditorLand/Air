use std::sync::{
	Arc,
	atomic::{AtomicI64, AtomicU64, Ordering},
};

use crate::{
	AirError,
	Metrics::{AggregationValidator::AggregationValidator, MinMaxUpdate::MinMaxUpdate},
	Result,
	dev_log,
};

/// MetricsCollector for collecting and exporting Prometheus metrics with thread
/// safety
#[derive(Debug, Clone)]
pub struct MetricsCollector {
	// Request metrics with atomic updates
	pub(crate) requests_total:Arc<AtomicU64>,

	pub(crate) requests_successful:Arc<AtomicU64>,

	pub(crate) requests_failed:Arc<AtomicU64>,

	pub(crate) request_latency_sum_ms:Arc<AtomicU64>,

	pub(crate) request_latency_count:Arc<AtomicU64>,

	pub(crate) request_latency_min_ms:Arc<AtomicU64>,

	pub(crate) request_latency_max_ms:Arc<AtomicU64>,

	// Error metrics
	pub(crate) errors_total:Arc<AtomicU64>,

	pub(crate) errors_by_type:Arc<std::sync::Mutex<std::collections::HashMap<String, u64>>>,

	// Resource metrics
	pub(crate) memory_usage_bytes:Arc<AtomicI64>,

	pub(crate) cpu_usage_percent:Arc<AtomicU64>,

	pub(crate) active_connections:Arc<AtomicU64>,

	pub(crate) threads_active:Arc<AtomicU64>,

	// Service-specific metrics
	pub(crate) authentication_operations:Arc<AtomicU64>,

	pub(crate) authentication_failures:Arc<AtomicU64>,

	pub(crate) downloads_total:Arc<AtomicU64>,

	pub(crate) downloads_completed:Arc<AtomicU64>,

	pub(crate) downloads_failed:Arc<AtomicU64>,

	pub(crate) downloads_bytes_total:Arc<AtomicU64>,

	pub(crate) indexing_operations:Arc<AtomicU64>,

	pub(crate) indexing_entries:Arc<AtomicI64>,

	pub(crate) updates_checked:Arc<AtomicU64>,

	pub(crate) updates_applied:Arc<AtomicU64>,

	// Aggregation validator
	pub(crate) aggregator:Arc<std::sync::Mutex<AggregationValidator>>,
}

impl MetricsCollector {
	/// Create a new MetricsCollector with thread-safe initialization
	pub fn new() -> Result<Self> {
		dev_log!("metrics", "[Metrics] MetricsCollector initialized successfully");

		Ok(Self {
			requests_total:Arc::new(AtomicU64::new(0)),
			requests_successful:Arc::new(AtomicU64::new(0)),
			requests_failed:Arc::new(AtomicU64::new(0)),
			request_latency_sum_ms:Arc::new(AtomicU64::new(0)),
			request_latency_count:Arc::new(AtomicU64::new(0)),
			request_latency_min_ms:Arc::new(AtomicU64::new(u64::MAX)),
			request_latency_max_ms:Arc::new(AtomicU64::new(0)),
			errors_total:Arc::new(AtomicU64::new(0)),
			errors_by_type:Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
			memory_usage_bytes:Arc::new(AtomicI64::new(0)),
			cpu_usage_percent:Arc::new(AtomicU64::new(0)),
			active_connections:Arc::new(AtomicU64::new(0)),
			threads_active:Arc::new(AtomicU64::new(0)),
			authentication_operations:Arc::new(AtomicU64::new(0)),
			authentication_failures:Arc::new(AtomicU64::new(0)),
			downloads_total:Arc::new(AtomicU64::new(0)),
			downloads_completed:Arc::new(AtomicU64::new(0)),
			downloads_failed:Arc::new(AtomicU64::new(0)),
			downloads_bytes_total:Arc::new(AtomicU64::new(0)),
			indexing_operations:Arc::new(AtomicU64::new(0)),
			indexing_entries:Arc::new(AtomicI64::new(0)),
			updates_checked:Arc::new(AtomicU64::new(0)),
			updates_applied:Arc::new(AtomicU64::new(0)),
			aggregator:Arc::new(std::sync::Mutex::new(AggregationValidator::new(3600))),
		})
	}

	/// Validate aggregation window before metric updates
	pub fn ValidateAggregation(&self) -> Result<()> {
		match self.aggregator.lock() {
			Ok(mut validator) => validator.validate().map_err(|e| AirError::Internal(e)),

			Err(_) => {
				dev_log!("metrics", "warn: [Metrics] Failed to acquire aggregation validator lock");

				Ok(())
			},
		}
	}

	/// Record a successful request with thread-safe atomic updates
	pub fn RecordRequestSuccess(&self, LatencySeconds:f64) {
		let _ = self.ValidateAggregation();

		let LatencyMs = (LatencySeconds * 1000.0) as u64;

		// Update request counts
		let _ = self.requests_total.fetch_add(1, Ordering::Relaxed);

		let _ = self.requests_successful.fetch_add(1, Ordering::Relaxed);

		// Update latency metrics
		let _ = self.request_latency_sum_ms.fetch_add(LatencyMs, Ordering::Relaxed);

		let _ = self.request_latency_count.fetch_add(1, Ordering::Relaxed);

		// Update min/max latency
		MinMaxUpdate(&self.request_latency_min_ms, &self.request_latency_max_ms, LatencyMs);

		dev_log!(
			"metrics",
			"[Metrics] Recorded successful request with latency: {:.3}s",
			LatencySeconds
		);
	}

	/// Record a failed request with thread-safe atomic updates
	pub fn RecordRequestFailure(&self, ErrorType:&str, LatencySeconds:f64) {
		let _ = self.ValidateAggregation();

		let LatencyMs = (LatencySeconds * 1000.0) as u64;

		// Update request counts
		let _ = self.requests_total.fetch_add(1, Ordering::Relaxed);

		let _ = self.requests_failed.fetch_add(1, Ordering::Relaxed);

		let _ = self.errors_total.fetch_add(1, Ordering::Relaxed);

		// Update latency metrics
		let _ = self.request_latency_sum_ms.fetch_add(LatencyMs, Ordering::Relaxed);

		let _ = self.request_latency_count.fetch_add(1, Ordering::Relaxed);

		// Update min/max latency
		MinMaxUpdate(&self.request_latency_min_ms, &self.request_latency_max_ms, LatencyMs);

		// Track error by type with redaction
		let RedactedError = self.RedactErrorType(ErrorType);

		let RedactedErrorClone = RedactedError.clone();

		if let Ok(mut error_map) = self.errors_by_type.lock() {
			*error_map.entry(RedactedError).or_insert(0) += 1;
		}

		dev_log!(
			"metrics",
			"[Metrics] Recorded failed request: {}, latency: {:.3}s",
			RedactedErrorClone,
			LatencySeconds
		);
	}

	/// Update resource usage metrics with thread-safe atomic updates
	pub fn UpdateResourceMetrics(&self, MemoryBytes:u64, CPUPercent:f64, ActiveConns:u64, ActiveThreads:u64) {
		self.memory_usage_bytes.store(MemoryBytes as i64, Ordering::Relaxed);

		self.cpu_usage_percent.store((CPUPercent * 100.0) as u64, Ordering::Relaxed);

		self.active_connections.store(ActiveConns, Ordering::Relaxed);

		self.threads_active.store(ActiveThreads, Ordering::Relaxed);

		dev_log!(
			"metrics",
			"[Metrics] Updated resource metrics - Memory: {}B, CPU: {:.1}%, Connections: {}, Threads: {}",
			MemoryBytes,
			CPUPercent,
			ActiveConns,
			ActiveThreads
		);
	}

	/// Record authentication operation
	pub fn RecordAuthenticationOperation(&self, Success:bool) {
		let _ = self.authentication_operations.fetch_add(1, Ordering::Relaxed);

		if !Success {
			let _ = self.authentication_failures.fetch_add(1, Ordering::Relaxed);
		}
	}

	/// Record download operation
	pub fn RecordDownload(&self, Success:bool, Bytes:u64) {
		let _ = self.downloads_total.fetch_add(1, Ordering::Relaxed);

		let _ = self.downloads_bytes_total.fetch_add(Bytes, Ordering::Relaxed);

		if Success {
			let _ = self.downloads_completed.fetch_add(1, Ordering::Relaxed);
		} else {
			let _ = self.downloads_failed.fetch_add(1, Ordering::Relaxed);
		}
	}

	/// Record indexing operation
	pub fn RecordIndexingOperation(&self, EntriesIndexed:u64) {
		let _ = self.indexing_operations.fetch_add(1, Ordering::Relaxed);

		self.indexing_entries.store(EntriesIndexed as i64, Ordering::Relaxed);
	}

	/// Record update check
	pub fn RecordUpdateCheck(&self, UpdatesAvailable:bool) {
		let _ = self.updates_checked.fetch_add(1, Ordering::Relaxed);

		if UpdatesAvailable {
			let _ = self.updates_applied.fetch_add(1, Ordering::Relaxed);
		}
	}

	/// Redact sensitive error types before tracking
	fn RedactErrorType(&self, ErrorType:&str) -> String {
		let Redacted = ErrorType.to_lowercase();

		// Redact common patterns
		if Redacted.contains("password") || Redacted.contains("token") || Redacted.contains("secret") {
			return "sensitive_error".to_string();
		}

		Redacted
	}

	/// Export metrics in Prometheus text format
	pub fn ExportMetrics(&self) -> Result<String> {
		let metrics_data = self.GetMetricsData();

		let mut output = String::new();

		output.push_str("# HELP air_requests_total Total number of requests processed by Air daemon\n");

		output.push_str("# TYPE air_requests_total counter\n");

		output.push_str(&format!("air_requests_total {}\n", metrics_data.requests_total));

		output.push_str("# HELP air_requests_successful Total number of successful requests\n");

		output.push_str("# TYPE air_requests_successful counter\n");

		output.push_str(&format!("air_requests_successful {}\n", metrics_data.requests_successful));

		output.push_str("# HELP air_requests_failed Total number of failed requests\n");

		output.push_str("# TYPE air_requests_failed counter\n");

		output.push_str(&format!("air_requests_failed {}\n", metrics_data.requests_failed));

		output.push_str("# HELP air_errors_total Total number of errors encountered\n");

		output.push_str("# TYPE air_errors_total counter\n");

		output.push_str(&format!("air_errors_total {}\n", metrics_data.errors_total));

		output.push_str("# HELP air_memory_usage_bytes Memory usage in bytes\n");

		output.push_str("# TYPE air_memory_usage_bytes gauge\n");

		output.push_str(&format!("air_memory_usage_bytes {}\n", metrics_data.memory_bytes));

		output.push_str("# HELP air_cpu_usage_percent CPU usage in hundredths of a percent\n");

		output.push_str("# TYPE air_cpu_usage_percent gauge\n");

		output.push_str(&format!("air_cpu_usage_percent {}\n", metrics_data.cpu_percent));

		output.push_str("# HELP air_active_connections Number of active connections\n");

		output.push_str("# TYPE air_active_connections gauge\n");

		output.push_str(&format!("air_active_connections {}\n", metrics_data.active_connections));

		output.push_str("# HELP air_threads_active Number of active threads\n");

		output.push_str("# TYPE air_threads_active gauge\n");

		output.push_str(&format!("air_threads_active {}\n", metrics_data.active_threads));

		output.push_str("# HELP air_authentication_operations_total Total authentication operations\n");

		output.push_str("# TYPE air_authentication_operations_total counter\n");

		output.push_str(&format!(
			"air_authentication_operations_total {}\n",
			metrics_data.authentication_operations
		));

		output.push_str("# HELP air_authentication_failures_total Total authentication failures\n");

		output.push_str("# TYPE air_authentication_failures_total counter\n");

		output.push_str(&format!(
			"air_authentication_failures_total {}\n",
			metrics_data.authentication_failures
		));

		output.push_str("# HELP air_downloads_total Total downloads initiated\n");

		output.push_str("# TYPE air_downloads_total counter\n");

		output.push_str(&format!("air_downloads_total {}\n", metrics_data.downloads_total));

		output.push_str("# HELP air_downloads_completed_total Total downloads completed successfully\n");

		output.push_str("# TYPE air_downloads_completed_total counter\n");

		output.push_str(&format!("air_downloads_completed_total {}\n", metrics_data.downloads_completed));

		output.push_str("# HELP air_downloads_failed_total Total downloads failed\n");

		output.push_str("# TYPE air_downloads_failed_total counter\n");

		output.push_str(&format!("air_downloads_failed_total {}\n", metrics_data.downloads_failed));

		output.push_str("# HELP air_downloads_bytes_total Total bytes downloaded\n");

		output.push_str("# TYPE air_downloads_bytes_total counter\n");

		output.push_str(&format!("air_downloads_bytes_total {}\n", metrics_data.downloads_bytes));

		output.push_str("# HELP air_indexing_operations_total Total indexing operations\n");

		output.push_str("# TYPE air_indexing_operations_total counter\n");

		output.push_str(&format!("air_indexing_operations_total {}\n", metrics_data.indexing_operations));

		output.push_str("# HELP air_indexing_entries Number of indexed entries\n");

		output.push_str("# TYPE air_indexing_entries gauge\n");

		output.push_str(&format!("air_indexing_entries {}\n", metrics_data.indexing_entries));

		output.push_str("# HELP air_updates_checked_total Total update checks performed\n");

		output.push_str("# TYPE air_updates_checked_total counter\n");

		output.push_str(&format!("air_updates_checked_total {}\n", metrics_data.updates_checked));

		output.push_str("# HELP air_updates_applied_total Total updates applied\n");

		output.push_str("# TYPE air_updates_applied_total counter\n");

		output.push_str(&format!("air_updates_applied_total {}\n", metrics_data.updates_applied));

		Ok(output)
	}

	/// Get metrics as structured data
	pub fn GetMetricsData(&self) -> crate::Metrics::MetricsData::MetricsData {
		let latency_avg = if self.request_latency_count.load(Ordering::Relaxed) > 0 {
			self.request_latency_sum_ms.load(Ordering::Relaxed) as f64
				/ self.request_latency_count.load(Ordering::Relaxed) as f64
		} else {
			0.0
		};

		crate::Metrics::MetricsData::MetricsData {
			timestamp:crate::Utility::CurrentTimestamp(),

			requests_total:self.requests_total.load(Ordering::Relaxed),

			requests_successful:self.requests_successful.load(Ordering::Relaxed),

			requests_failed:self.requests_failed.load(Ordering::Relaxed),

			errors_total:self.errors_total.load(Ordering::Relaxed),

			memory_bytes:self.memory_usage_bytes.load(Ordering::Relaxed).max(0) as u64,

			cpu_percent:self.cpu_usage_percent.load(Ordering::Relaxed) as f64 / 100.0,

			active_connections:self.active_connections.load(Ordering::Relaxed),

			active_threads:self.threads_active.load(Ordering::Relaxed),

			authentication_operations:self.authentication_operations.load(Ordering::Relaxed),

			authentication_failures:self.authentication_failures.load(Ordering::Relaxed),

			downloads_total:self.downloads_total.load(Ordering::Relaxed),

			downloads_completed:self.downloads_completed.load(Ordering::Relaxed),

			downloads_failed:self.downloads_failed.load(Ordering::Relaxed),

			downloads_bytes:self.downloads_bytes_total.load(Ordering::Relaxed),

			indexing_operations:self.indexing_operations.load(Ordering::Relaxed),

			indexing_entries:self.indexing_entries.load(Ordering::Relaxed).max(0) as u64,

			updates_checked:self.updates_checked.load(Ordering::Relaxed),

			updates_applied:self.updates_applied.load(Ordering::Relaxed),

			latency_avg_ms:latency_avg,

			latency_min_ms:self.request_latency_min_ms.load(Ordering::Relaxed),

			latency_max_ms:self.request_latency_max_ms.load(Ordering::Relaxed),
		}
	}

	/// Reset all metrics for testing purposes
	#[cfg(test)]
	pub fn Reset(&self) {
		self.requests_total.store(0, Ordering::Relaxed);

		self.requests_successful.store(0, Ordering::Relaxed);

		self.requests_failed.store(0, Ordering::Relaxed);

		self.request_latency_sum_ms.store(0, Ordering::Relaxed);

		self.request_latency_count.store(0, Ordering::Relaxed);

		self.request_latency_min_ms.store(u64::MAX, Ordering::Relaxed);

		self.request_latency_max_ms.store(0, Ordering::Relaxed);

		self.errors_total.store(0, Ordering::Relaxed);

		self.memory_usage_bytes.store(0, Ordering::Relaxed);

		self.cpu_usage_percent.store(0, Ordering::Relaxed);

		self.active_connections.store(0, Ordering::Relaxed);

		self.threads_active.store(0, Ordering::Relaxed);

		self.authentication_operations.store(0, Ordering::Relaxed);

		self.authentication_failures.store(0, Ordering::Relaxed);

		self.downloads_total.store(0, Ordering::Relaxed);

		self.downloads_completed.store(0, Ordering::Relaxed);

		self.downloads_failed.store(0, Ordering::Relaxed);

		self.downloads_bytes_total.store(0, Ordering::Relaxed);

		self.indexing_operations.store(0, Ordering::Relaxed);

		self.indexing_entries.store(0, Ordering::Relaxed);

		self.updates_checked.store(0, Ordering::Relaxed);

		self.updates_applied.store(0, Ordering::Relaxed);
	}
}

impl Default for MetricsCollector {
	fn default() -> Self { Self::new().expect("Failed to create MetricsCollector") }
}
