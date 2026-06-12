use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::Tracing::{SpanEvent::SpanEvent, SpanStatus::SpanStatus};

/// A single span in a trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
	pub span_id:String,

	pub trace_id:String,

	pub parent_span_id:Option<String>,

	pub operation_name:String,

	pub start_time:u64,

	pub end_time:Option<u64>,

	pub status:SpanStatus,

	pub attributes:HashMap<String, String>,

	pub events:Vec<SpanEvent>,

	pub error:Option<String>,

	pub duration_ms:Option<u64>,
}
