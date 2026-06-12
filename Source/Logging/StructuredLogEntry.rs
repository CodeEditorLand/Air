//! Structured log entry for validation.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{Result, dev_log};

/// Structured log entry for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredLogEntry {
	pub Timestamp:u64,

	pub Level:String,

	pub Message:String,

	pub RequestId:Option<String>,

	pub TraceId:Option<String>,

	pub SpanId:Option<String>,

	pub Operation:Option<String>,

	pub UserId:Option<String>,

	pub Metadata:HashMap<String, String>,
}

impl StructuredLogEntry {
	/// Validate log entry structure
	pub fn Validate(&self) -> Result<()> {
		if self.Level.is_empty() {
			return Err("log level cannot be empty".into());
		}

		if self.Message.is_empty() {
			return Err("log message cannot be empty".into());
		}

		if !["TRACE", "DEBUG", "INFO", "WARN", "ERROR"].contains(&self.Level.as_str()) {
			return Err(format!("invalid log level: {}", self.Level).into());
		}

		if self.Message.len() > 10000 {
			// Max 10KB message
			return Err("log message too large".into());
		}

		Ok(())
	}
}
