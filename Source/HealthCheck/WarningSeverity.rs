use serde::{Deserialize, Serialize};

/// Warning severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WarningSeverity {
	Low,

	Medium,

	High,

	Critical,
}
