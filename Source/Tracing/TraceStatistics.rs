use serde::{Deserialize, Serialize};

/// Trace statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStatistics {
	pub total_traces:u64,

	pub total_spans:u64,

	pub completed_spans:u64,

	pub failed_spans:u64,

	pub in_progress_spans:u64,
}
