//! Bulkhead statistics for metrics export.
//!
//! Snapshot of `BulkheadExecutor` load and counters.

use serde::{Deserialize, Serialize};

/// Bulkhead statistics for metrics export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkheadStatistics {
	pub name:String,

	pub current_concurrent:u32,

	pub current_queue:u32,

	pub max_concurrent:usize,

	pub max_queue:usize,

	pub total_rejected:u64,

	pub total_completed:u64,

	pub total_timed_out:u64,
}
