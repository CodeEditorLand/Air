use serde::{Deserialize, Serialize};

/// Security severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecuritySeverity {
	Informational,

	Warning,

	Error,

	Critical,
}
