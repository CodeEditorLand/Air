use serde::{Deserialize, Serialize};

/// Resource warning types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceWarningType {
	HighMemoryUsage,

	HighCPUUsage,

	LowDiskSpace,

	ConnectionPoolExhausted,

	ThreadPoolExhausted,

	HighLatency,

	HighErrorRate,

	DatabaseConnectivityIssue,
}
