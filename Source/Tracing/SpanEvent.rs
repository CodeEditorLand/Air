use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Event within a span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
	pub timestamp:u64,

	pub name:String,

	pub attributes:HashMap<String, String>,
}
