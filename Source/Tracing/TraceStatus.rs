use serde::{Deserialize, Serialize};

/// Trace status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TraceStatus {
	InProgress,

	Completed,

	Failed,

	Cancelled,
}
