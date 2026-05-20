#![allow(non_snake_case, unused_variables, dead_code, unused_imports)]

//! Response DTO structs returned by daemon IPC calls and serialised to
//! stdout by `OutputFormatter`. Keeping these separate from the parser and
//! handler code makes the wire contract easy to review and unit-test.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Overall daemon + services status.
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
	pub daemon_running:bool,
	pub uptime_secs:u64,
	pub version:String,
	pub services:HashMap<String, ServiceStatus>,
	pub timestamp:String,
}

/// Per-service status inside a `StatusResponse`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceStatus {
	pub name:String,
	pub running:bool,
	pub health:ServiceHealth,
	pub uptime_secs:u64,
	pub error:Option<String>,
}

/// Coarse service health classification.
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "UPPERCASE")]
pub enum ServiceHealth {
	Healthy,
	Degraded,
	Unhealthy,
	Unknown,
}

/// Aggregated performance metrics snapshot.
#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
	pub timestamp:String,
	pub memory_used_mb:f64,
	pub memory_available_mb:f64,
	pub cpu_usage_percent:f64,
	pub disk_used_mb:u64,
	pub disk_available_mb:u64,
	pub active_connections:u32,
	pub processed_requests:u64,
	pub failed_requests:u64,
	pub service_metrics:HashMap<String, ServiceMetrics>,
}

/// Per-service request latency counters.
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceMetrics {
	pub name:String,
	pub requests_total:u64,
	pub requests_success:u64,
	pub requests_failed:u64,
	pub average_latency_ms:f64,
	pub p99_latency_ms:f64,
}

/// Result of a health-check sweep across all services.
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckResponse {
	pub overall_healthy:bool,
	pub overall_health_percentage:f64,
	pub services:HashMap<String, ServiceHealthDetail>,
	pub timestamp:String,
}

/// Fine-grained health detail for one service.
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceHealthDetail {
	pub name:String,
	pub healthy:bool,
	pub response_time_ms:u64,
	pub last_check:String,
	pub details:String,
}

/// Configuration key/value read result.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigResponse {
	pub key:Option<String>,
	pub value:serde_json::Value,
	pub path:String,
	pub modified:String,
}

/// Single structured log line.
#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntry {
	pub timestamp:DateTime<Utc>,
	pub level:String,
	pub service:Option<String>,
	pub message:String,
	pub context:Option<serde_json::Value>,
}

/// Active IPC connection metadata.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionInfo {
	pub id:String,
	pub remote_address:String,
	pub connected_at:DateTime<Utc>,
	pub service:Option<String>,
	pub active:bool,
}

/// Full daemon state snapshot (debug dump).
#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonState {
	pub timestamp:DateTime<Utc>,
	pub version:String,
	pub uptime_secs:u64,
	pub services:HashMap<String, serde_json::Value>,
	pub connections:Vec<ConnectionInfo>,
	pub plugin_state:serde_json::Value,
}
