//! Plugin validation result: Valid, Invalid, or Warning.

use serde::{Deserialize, Serialize};

/// Plugin validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginValidationResult {
	Valid,

	Invalid(String),

	Warning(String),
}
