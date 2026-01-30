//! # Prometheus Metrics Module
//!
//! Provides Prometheus-compatible metrics collection and export for the Air daemon.
//! Includes request latency histograms, resource usage metrics, and error rate tracking.

use std::sync::Arc;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::gauge::Gauge;
use serde::{Deserialize, Serialize};
use log::{debug, info};

use crate::Result;

/// MetricsCollector for collecting and exporting Prometheus metrics
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    // Request metrics
    requests_total: Arc<Counter>,
    requests_successful: Arc<Counter>,
    requests_failed: Arc<Counter>,
    request_latency_sum_seconds: Arc<Gauge>,
    request_latency_count: Arc<Counter>,
    
    // Error metrics
    errors_total: Arc<Counter>,
    errors_by_type: Arc<std::sync::Mutex<std::collections::HashMap<String, Counter>>>,
    
    // Resource metrics
    memory_usage_bytes: Arc<Gauge>,
    cpu_usage_percent: Arc<Gauge>,
    active_connections: Arc<Gauge>,
    threads_active: Arc<Gauge>,
    
    // Service-specific metrics
    authentication_operations: Arc<Counter>,
    authentication_failures: Arc<Counter>,
    downloads_total: Arc<Counter>,
    downloads_completed: Arc<Counter>,
    downloads_failed: Arc<Counter>,
    indexing_operations: Arc<Counter>,
    indexing_entries: Arc<Gauge>,
    updates_checked: Arc<Counter>,
    updates_applied: Arc<Counter>,
}

impl MetricsCollector {
    /// Create a new MetricsCollector
    pub fn new() -> Result<Self> {
        info!("[Metrics] MetricsCollector initialized successfully");
        
        Ok(Self {
            requests_total: Arc::new(Counter::default()),
            requests_successful: Arc::new(Counter::default()),
            requests_failed: Arc::new(Counter::default()),
            request_latency_sum_seconds: Arc::new(Gauge::default()),
            request_latency_count: Arc::new(Counter::default()),
            errors_total: Arc::new(Counter::default()),
            errors_by_type: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            memory_usage_bytes: Arc::new(Gauge::default()),
            cpu_usage_percent: Arc::new(Gauge::default()),
            active_connections: Arc::new(Gauge::default()),
            threads_active: Arc::new(Gauge::default()),
            authentication_operations: Arc::new(Counter::default()),
            authentication_failures: Arc::new(Counter::default()),
            downloads_total: Arc::new(Counter::default()),
            downloads_completed: Arc::new(Counter::default()),
            downloads_failed: Arc::new(Counter::default()),
            indexing_operations: Arc::new(Counter::default()),
            indexing_entries: Arc::new(Gauge::default()),
            updates_checked: Arc::new(Counter::default()),
            updates_applied: Arc::new(Counter::default()),
        })
    }
    
    /// Record a successful request
    pub fn record_request_success(&self, latency_seconds: f64) {
        self.requests_total.inc();
        self.requests_successful.inc();
        
        // Record latency for histogram simulation
        let current_sum = self.request_latency_sum_seconds.get();
        self.request_latency_sum_seconds.set(current_sum as i64 + (latency_seconds * 1000.0) as i64);
        self.request_latency_count.inc();
        
        debug!("[Metrics] Recorded successful request with latency: {:.3}s", latency_seconds);
    }
    
    /// Record a failed request
    pub fn record_request_failure(&self, error_type: &str, latency_seconds: f64) {
        self.requests_total.inc();
        self.requests_failed.inc();
        self.errors_total.inc();
        
        // Record latency for histogram simulation
        let current_sum = self.request_latency_sum_seconds.get();
        self.request_latency_sum_seconds.set(current_sum as i64 + (latency_seconds * 1000.0) as i64);
        self.request_latency_count.inc();
        
        // Track error by type
        let mut error_map = self.errors_by_type.lock().unwrap();
        error_map.entry(error_type.to_string())
            .or_insert_with(|| Counter::default())
            .inc();
        
        debug!("[Metrics] Recorded failed request: {}, latency: {:.3}s", error_type, latency_seconds);
    }
    
    /// Update resource usage metrics
    pub fn update_resource_metrics(&self, memory_bytes: u64, cpu_percent: f64, active_conns: u64, active_threads: u64) {
        self.memory_usage_bytes.set(memory_bytes as i64);
        self.cpu_usage_percent.set(cpu_percent as i64);
        self.active_connections.set(active_conns as i64);
        self.threads_active.set(active_threads as i64);
        debug!("[Metrics] Updated resource metrics - Memory: {}B, CPU: {:.1}%, Connections: {}, Threads: {}", 
               memory_bytes, cpu_percent, active_conns, active_threads);
    }
    
    /// Record authentication operation
    pub fn record_authentication_operation(&self, success: bool) {
        self.authentication_operations.inc();
        if !success {
            self.authentication_failures.inc();
        }
    }
    
    /// Record download operation
    pub fn record_download(&self, success: bool) {
        self.downloads_total.inc();
        if success {
            self.downloads_completed.inc();
        } else {
            self.downloads_failed.inc();
        }
    }
    
    /// Record indexing operation
    pub fn record_indexing_operation(&self, entries_indexed: u64) {
        self.indexing_operations.inc();
        self.indexing_entries.set(entries_indexed as i64);
    }
    
    /// Record update check
    pub fn record_update_check(&self, updates_available: bool) {
        self.updates_checked.inc();
        if updates_available {
            self.updates_applied.inc();
        }
    }
    
    /// Export metrics in Prometheus text format
    pub fn export_metrics(&self) -> Result<String> {
        let metrics_data = self.get_metrics_data();
        
        // Format as Prometheus exposition format
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
        
        output.push_str("# HELP air_cpu_usage_percent CPU usage percentage\n");
        output.push_str("# TYPE air_cpu_usage_percent gauge\n");
        output.push_str(&format!("air_cpu_usage_percent {:.2}\n", metrics_data.cpu_percent));
        
        output.push_str("# HELP air_active_connections Number of active connections\n");
        output.push_str("# TYPE air_active_connections gauge\n");
        output.push_str(&format!("air_active_connections {}\n", metrics_data.active_connections));
        
        output.push_str("# HELP air_threads_active Number of active threads\n");
        output.push_str("# TYPE air_threads_active gauge\n");
        output.push_str(&format!("air_threads_active {}\n", metrics_data.active_threads));
        
        output.push_str("# HELP air_authentication_operations_total Total authentication operations\n");
        output.push_str("# TYPE air_authentication_operations_total counter\n");
        output.push_str(&format!("air_authentication_operations_total {}\n", metrics_data.authentication_operations));
        
        output.push_str("# HELP air_authentication_failures_total Total authentication failures\n");
        output.push_str("# TYPE air_authentication_failures_total counter\n");
        output.push_str(&format!("air_authentication_failures_total {}\n", metrics_data.authentication_failures));
        
        output.push_str("# HELP air_downloads_total Total downloads initiated\n");
        output.push_str("# TYPE air_downloads_total counter\n");
        output.push_str(&format!("air_downloads_total {}\n", metrics_data.downloads_total));
        
        output.push_str("# HELP air_downloads_completed_total Total downloads completed successfully\n");
        output.push_str("# TYPE air_downloads_completed_total counter\n");
        output.push_str(&format!("air_downloads_completed_total {}\n", metrics_data.downloads_completed));
        
        output.push_str("# HELP air_downloads_failed_total Total downloads failed\n");
        output.push_str("# TYPE air_downloads_failed_total counter\n");
        output.push_str(&format!("air_downloads_failed_total {}\n", metrics_data.downloads_failed));
        
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
    pub fn get_metrics_data(&self) -> MetricsData {
        MetricsData {
            timestamp: crate::utils::CurrentTimestamp(),
            requests_total: self.requests_total.get(),
            requests_successful: self.requests_successful.get(),
            requests_failed: self.requests_failed.get(),
            errors_total: self.errors_total.get(),
            memory_bytes: self.memory_usage_bytes.get().max(0) as u64,
            cpu_percent: self.cpu_usage_percent.get() as f64,
            active_connections: self.active_connections.get().max(0) as u64,
            active_threads: self.threads_active.get().max(0) as u64,
            authentication_operations: self.authentication_operations.get(),
            authentication_failures: self.authentication_failures.get(),
            downloads_total: self.downloads_total.get(),
            downloads_completed: self.downloads_completed.get(),
            downloads_failed: self.downloads_failed.get(),
            indexing_operations: self.indexing_operations.get(),
            indexing_entries: self.indexing_entries.get().max(0) as u64,
            updates_checked: self.updates_checked.get(),
            updates_applied: self.updates_applied.get(),
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new().expect("Failed to create MetricsCollector")
    }
}

/// Structured metrics data for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsData {
    pub timestamp: u64,
    pub requests_total: u64,
    pub requests_successful: u64,
    pub requests_failed: u64,
    pub errors_total: u64,
    pub memory_bytes: u64,
    pub cpu_percent: f64,
    pub active_connections: u64,
    pub active_threads: u64,
    pub authentication_operations: u64,
    pub authentication_failures: u64,
    pub downloads_total: u64,
    pub downloads_completed: u64,
    pub downloads_failed: u64,
    pub indexing_operations: u64,
    pub indexing_entries: u64,
    pub updates_checked: u64,
    pub updates_applied: u64,
}

impl MetricsData {
    /// Calculate success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.requests_total == 0 {
            return 100.0;
        }
        (self.requests_successful as f64 / self.requests_total as f64) * 100.0
    }
    
    /// Calculate download success rate
    pub fn download_success_rate(&self) -> f64 {
        if self.downloads_total == 0 {
            return 100.0;
        }
        (self.downloads_completed as f64 / self.downloads_total as f64) * 100.0
    }
    
    /// Calculate error rate
    pub fn error_rate(&self) -> f64 {
        if self.requests_total == 0 {
            return 0.0;
        }
        (self.errors_total as f64 / self.requests_total as f64) * 100.0
    }
}

/// Global metrics collector instance
static METRICS_INSTANCE: std::sync::OnceLock<MetricsCollector> = std::sync::OnceLock::new();

/// Get or initialize the global metrics collector
pub fn get_metrics() -> &'static MetricsCollector {
    METRICS_INSTANCE.get_or_init(|| MetricsCollector::default())
}

/// Initialize the global metrics collector
pub fn initialize_metrics() -> Result<()> {
    let _collector = get_metrics();
    info!("[Metrics] Global metrics collector initialized");
    Ok(())
}
