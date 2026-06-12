use serde::{Deserialize, Serialize};

/// Span status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpanStatus {
	Started,

	Active,

	Completed,

	Failed,

	Cancelled,
}
