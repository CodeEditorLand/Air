use serde::{Deserialize, Serialize};

/// Resource usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
	pub MemoryUsageMb:f64,

	pub CPUUsagePercent:f64,

	pub DiskUsageMb:f64,

	pub NetworkUsageMbps:f64,

	pub LastUpdated:u64,
}
