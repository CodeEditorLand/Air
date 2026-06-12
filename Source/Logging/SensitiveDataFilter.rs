//! Sensitive data filter for log sanitization.

use std::sync::Arc;
use crate::{Result, dev_log};
use crate::Logging::SensitiveDataConfig::SensitiveDataConfig;
/// Sensitive data filter for log sanitization
#[derive(Debug, Clone)]
pub struct SensitiveDataFilter {
	enabled:bool,

	patterns:Vec<regex::Regex>,
}

impl Default for SensitiveDataFilter {
	fn default() -> Self {
		let mut patterns = Vec::new();

		// Standard sensitive patterns - simplified to avoid escaping issues
		patterns.push(regex::Regex::new(r"(?i)password[=[:space:]]+\S+").unwrap());

		patterns.push(regex::Regex::new(r"(?i)token[=[:space:]]+\S+").unwrap());

		patterns.push(regex::Regex::new(r"(?i)secret[=[:space:]]+\S+").unwrap());

		patterns.push(regex::Regex::new(r"(?i)(api|private)[_-]?key[=[:space:]]+\S+").unwrap());

		patterns.push(regex::Regex::new(r"(?i)authorization[=[:space:]]+Bearer[[:space:]]+\S+").unwrap());

		patterns.push(regex::Regex::new(r"(?i)credential[=[:space:]]+\S+").unwrap());

		Self { enabled:true, patterns }
	}
}

impl SensitiveDataFilter {
	fn new(Config:SensitiveDataConfig) -> Result<Self> {
		let mut filter = Self::default();

		filter.enabled = Config.Enabled;

		if !Config.IncludeStandardPatterns {
			filter.patterns.clear();
		}

		// Add custom patterns
		for pattern in &Config.CustomPatterns {
			match regex::Regex::new(pattern) {
				Ok(re) => filter.patterns.push(re),

				Err(e) => dev_log!("air", "warn: [Logging] Failed to compile custom regex '{}': {}", pattern, e),
			}
		}

		Ok(filter)
	}

	/// Filter sensitive data from a string
	fn Filter(&self, input:&str) -> String {
		if !self.enabled {
			return input.to_string();
		}

		let mut filtered = input.to_string();

		for pattern in &self.patterns {
			filtered = pattern.replace_all(&filtered, "[REDACTED]").to_string();
		}

		filtered
	}
}
