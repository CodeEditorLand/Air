//! Configuration for sensitive data redaction in log output.

use serde::{Deserialize, Serialize};
/// Sensitive data patterns for redaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveDataConfig {
	/// Enable automatic sensitive data redaction
	pub Enabled:bool,

	/// Custom patterns to redact (regex)
	pub CustomPatterns:Vec<String>,

	/// Standard patterns to include (password, token, secret, etc.)
	pub IncludeStandardPatterns:bool,
}

impl Default for SensitiveDataConfig {
	fn default() -> Self { Self { Enabled:true, CustomPatterns:Vec::new(), IncludeStandardPatterns:true } }
}
