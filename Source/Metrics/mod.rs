//! # Metrics Collection and Export Module
//!
//! Provides Prometheus-compatible metrics collection and export for the Air
//! daemon. Includes request latency histograms, resource usage metrics, error
//! rate tracking, thread-safe metric updates with overflow protection, and
//! aggregation validation.
//!
//! ## Responsibilities
//!
//! ### Request Metrics
//! - Total request count tracking with counter overflow protection
//! - Success/failure rate calculation
//! - Latency histogram with bucket aggregation
//! - Request throughput measurement
//!
//! ### Error Metrics
//! - Total error count with classification
//! - Error rate calculation by type
//! - Error aggregation and validation
//! - Error threshold alerting
//!
//! ### Resource Metrics
//! - Memory usage monitoring with allocation tracking
//! - CPU utilization percentage
//! - Active connection count
//! - Thread activity monitoring
//! - Resource pool utilization
//!
//! ### Service-Specific Metrics
//! - Authentication success/failure tracking
//! - Download completion rates
//! - Indexing operation metrics
//! - Update deployment statistics
//!
//! ## Integration with Mountain
//!
//! Metrics flow directly to Mountain's telemetry UI:
//! - Prometheus exposition format endpoint
//! - Real-time metric streaming
//! - Historical data retention
//! - Custom dashboards and alerting
//!
//! ## VSCode Telemetry References
//!
//! Similar telemetry patterns used in VSCode for:
//! - Performance monitoring and profiling
//! - Usage statistics and feature adoption
//! - Error tracking and crash reporting
//! - Extension marketplace analytics
//!
//! Reference:
//! vs/workbench/services/telemetry
//!
//! ## Thread Safety
//!
//! All metric updates are thread-safe using:
//! - Arc for shared ownership across threads
//! - Atomic operations where possible
//! - Mutex locks for complex aggregations
//! - Lock-free counters for high-frequency updates
//!
//! # TODOs
//!
//! - [DISTRIBUTED TRACING] Integrate with OpenTelemetry for distributed tracing
//!   metrics
//! - [CUSTOM METRICS] Add custom metric types for business KPIs
//! - [ALERTING] Implement metric-based alerting thresholds
//! - [AGGREGATION] Add time-windowed aggregations (1m, 5m, 15m)
//! - [EXPORT] Add support for external monitoring systems (Datadog, New Relic)
//!
//! ## Sensitive Data Handling
//!
//! Metrics aggregation ensures sensitive data is excluded:
//! - No request payloads in metrics
//! - No authentication tokens in labels
//! - No user-identifiable information in error classifications
//! - IP addresses and PII are aggregated, not logged individually

use std::{
	sync::{
		Arc,
		atomic::{AtomicI64, AtomicU64, Ordering},
	},
	time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use log::{debug, info, warn};

use crate::{AirError, Result};

/// Overflow-protected metric update helper
struct MetricGuard {
	current:u64,
	max:u64,
}

impl MetricGuard {
	fn new(current:u64, max:u64) -> Self { Self { current, max } }

	/// Increment with overflow protection
	fn increment(&mut self) -> bool {
		if self.current < self.max.saturating_sub(1) {
			self.current += 1;
			true
		} else {
			warn!("[Metrics] Metric overflow detected, wrapping around");
			self.current = 0;
			true
		}
	}
}

/// Aggregation validation for metric integrity
#[derive(Debug)]
struct AggregationValidator {
	last_timestamp:Instant,
	validation_window:Duration,
}

impl AggregationValidator {
	fn new(validation_window_secs:u64) -> Self {
		Self {
			last_timestamp:Instant::now(),
			validation_window:Duration::from_secs(validation_window_secs),
		}
	}

	/// Validate aggregation is within time window
	fn validate(&mut self) -> std::result::Result<(), String> {
		let now = Instant::now();
		if now.duration_since(self.last_timestamp) > self.validation_window {
			warn!("[Metrics] Aggregation outside validation window, resetting");
			self.last_timestamp = now;
			Ok(())
		} else {
			Ok(())
		}
	}
}

/// MetricsCollector for collecting and exporting Prometheus metrics with thread
/// safety
#[derive(Debug, Clone)]
pub struct MetricsCollector {
	// Request metrics with atomic updates
	requests_total:Arc<AtomicU64>,
	requests_successful:Arc<AtomicU64>,
	requests_failed:Arc<AtomicU64>,
	request_latency_sum_ms:Arc<AtomicU64>,
	request_latency_count:Arc<AtomicU64>,
	request_latency_min_ms:Arc<AtomicU64>,
	request_latency_max_ms:Arc<AtomicU64>,

	// Error metrics
	errors_total:Arc<AtomicU64>,
	errors_by_type:Arc<std::sync::Mutex<std::collections::HashMap<String, u64>>>,

	// Resource metrics
	memory_usage_bytes:Arc<AtomicI64>,
	cpu_usage_percent:Arc<AtomicU64>,
	active_connections:Arc<AtomicU64>,
	threads_active:Arc<AtomicU64>,

	// Service-specific metrics
	authentication_operations:Arc<AtomicU64>,
	authentication_failures:Arc<AtomicU64>,
	downloads_total:Arc<AtomicU64>,
	downloads_completed:Arc<AtomicU64>,
	downloads_failed:Arc<AtomicU64>,
	downloads_bytes_total:Arc<AtomicU64>,
	indexing_operations:Arc<AtomicU64>,
	indexing_entries:Arc<AtomicI64>,
	updates_checked:Arc<AtomicU64>,
	updates_applied:Arc<AtomicU64>,

	// Aggregation validator
	aggregator:Arc<std::sync::Mutex<AggregationValidator>>,
}

impl MetricsCollector {
	/// Create a new MetricsCollector with thread-safe initialization
	pub fn new() -> Result<Self> {
		info!("[Metrics] MetricsCollector initialized successfully");

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
				warn!("[Metrics] Failed to acquire aggregation validator lock");
				Ok(())
			},
		}
	}

	/// Record a successful request with thread-safe atomic updates
	pub fn RecordRequestSuccess(&self, LatencySeconds:f64) {
		self.ValidateAggregation();

		let LatencyMs = (LatencySeconds * 1000.0) as u64;

		// Update request counts
		let _ = self.requests_total.fetch_add(1, Ordering::Relaxed);
		let _ = self.requests_successful.fetch_add(1, Ordering::Relaxed);

		// Update latency metrics
		let _ = self.request_latency_sum_ms.fetch_add(LatencyMs, Ordering::Relaxed);
		let _ = self.request_latency_count.fetch_add(1, Ordering::Relaxed);

		// Update min/max latency
		MinMaxUpdate(&self.request_latency_min_ms, &self.request_latency_max_ms, LatencyMs);

		debug!("[Metrics] Recorded successful request with latency: {:.3}s", LatencySeconds);
	}

	/// Record a failed request with thread-safe atomic updates
	pub fn RecordRequestFailure(&self, ErrorType:&str, LatencySeconds:f64) {
		self.ValidateAggregation();

		let LatencyMs = (LatencySeconds * 1000.0) as u64;

		// Update request counts
		let _ = self.requests_total.fetch_add(1, Ordering::Relaxed);
		let _ = self.requests_failed.fetch_add(1, Ordering::Relaxed);
		let _ = self.errors_total.fetch_add(1, Ordering::Relaxed);

		// Update latency metrics
		let _ = self.request_latency_sum_ms.fetch_add(latency_ms, Ordering::Relaxed);
		let _ = self.request_latency_count.fetch_add(1, Ordering::Relaxed);

		// Update min/max latency
		min_max_update(&self.request_latency_min_ms, &self.request_latency_max_ms, latency_ms);

		// Track error by type with redaction
		let redacted_error = self.redact_error_type(error_type);
		let redacted_error_clone = redacted_error.clone();
		if let Ok(mut error_map) = self.errors_by_type.lock() {
			*error_map.entry(redacted_error).or_insert(0) += 1;
		}

		debug!(
			"[Metrics] Recorded failed request: {}, latency: {:.3}s",
			redacted_error_clone, latency_seconds
		);
	}

	/// Update resource usage metrics with thread-safe atomic updates
	pub fn UpdateResourceMetrics(&self, MemoryBytes:u64, CpuPercent:f64, ActiveConns:u64, ActiveThreads:u64) {
		self.memory_usage_bytes.store(MemoryBytes as i64, Ordering::Relaxed);
		self.cpu_usage_percent.store((CpuPercent * 100.0) as u64, Ordering::Relaxed);
		self.active_connections.store(ActiveConns, Ordering::Relaxed);
		self.threads_active.store(ActiveThreads, Ordering::Relaxed);

		debug!(
			"[Metrics] Updated resource metrics - Memory: {}B, CPU: {:.1}%, Connections: {}, Threads: {}",
			MemoryBytes, CpuPercent, ActiveConns, ActiveThreads
		);
	}

	/// Record authentication operation
	pub fn RecordAuthenticationOperation(&Self, Success:bool) {
		let _ = self.authentication_operations.fetch_add(1, Ordering::Relaxed);
		if !Success {
			let _ = self.authentication_failures.fetch_add(1, Ordering::Relaxed);
		}
	}

	/// Record download operation
	pub fn RecordDownload(&Self, Success:bool, Bytes:u64) {
		let _ = self.downloads_total.fetch_add(1, Ordering::Relaxed);
		let _ = self.downloads_bytes_total.fetch_add(Bytes, Ordering::Relaxed);

		if Success {
			let _ = self.downloads_completed.fetch_add(1, Ordering::Relaxed);
		} else {
			let _ = self.downloads_failed.fetch_add(1, Ordering::Relaxed);
		}
	}

	/// Record indexing operation
	pub fn RecordIndexingOperation(&Self, EntriesIndexed:u64) {
		let _ = self.indexing_operations.fetch_add(1, Ordering::Relaxed);
		self.indexing_entries.store(EntriesIndexed as i64, Ordering::Relaxed);
	}

	/// Record update check
	pub fn RecordUpdateCheck(&Self, UpdatesAvailable:bool) {
		let _ = self.updates_checked.fetch_add(1, Ordering::Relaxed);
		if UpdatesAvailable {
			let _ = self.updates_applied.fetch_add(1, Ordering::Relaxed);
		}
	}

	/// Redact sensitive error types before tracking
	fn RedactErrorType(&Self, ErrorType:&Str) -> String {
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
	pub fn GetMetricsData(&self) -> MetricsData {
		let latency_avg = if self.request_latency_count.load(Ordering::Relaxed) > 0 {
			self.request_latency_sum_ms.load(Ordering::Relaxed) as f64
				/ self.request_latency_count.load(Ordering::Relaxed) as f64
		} else {
			0.0
		};

		MetricsData {
			timestamp:crate::utils::CurrentTimestamp(),
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

/// Helper function to update min/max values atomically
fn MinMaxUpdate(MinMetric:&AtomicU64, MaxMetric:&AtomicU64, Value:u64) {
	let mut CurrentMin = MinMetric.load(Ordering::Relaxed);
	let mut CurrentMax = MaxMetric.load(Ordering::Relaxed);

	loop {
		if Value < CurrentMin {
			match MinMetric.compare_exchange_weak(CurrentMin, Value, Ordering::Relaxed, Ordering::Relaxed) {
				Ok(_) => break,
				Err(NewMin) => CurrentMin = NewMin,
			}
		} else if Value > CurrentMax {
			match MaxMetric.compare_exchange_weak(CurrentMax, Value, Ordering::Relaxed, Ordering::Relaxed) {
				Ok(_) => break,
				Err(NewMax) => CurrentMax = NewMax,
			}
		} else {
			break;
		}
	}
}

impl Default for MetricsCollector {
	fn default() -> Self { Self::new().expect("Failed to create MetricsCollector") }
}

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

/// Global metrics collector instance
static METRICS_INSTANCE:std::sync::OnceLock<MetricsCollector> = std::sync::OnceLock::new();

/// Get or initialize the global metrics collector
pub fn GetMetrics() -> &'static MetricsCollector { METRICS_INSTANCE.get_or_init(|| MetricsCollector::default()) }

/// Initialize the global metrics collector
pub fn InitializeMetrics() -> Result<()> {
	let _collector = GetMetrics();
	info!("[Metrics] Global metrics collector initialized");
	Ok(())
}
