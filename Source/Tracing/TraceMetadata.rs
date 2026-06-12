use serde::{Deserialize, Serialize};

use crate::Tracing::TraceStatus::TraceStatus;

/// Distributed trace metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceMetadata {
	pub trace_id:String,

	pub root_span_id:String,

	pub total_spans:usize,

	pub root_operation:String,

	pub start_time:u64,

	pub end_time:Option<u64>,

	pub total_duration_ms:Option<u64>,

	pub status:TraceStatus,
}
