use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{SecurityEventType, SecuritySeverity};

/// Security event audit log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
	/// Event timestamp
	pub Timestamp:u64,

	/// Event type
	pub EventType:SecurityEventType,

	/// Event severity
	pub Severity:SecuritySeverity,

	/// Source IP address (if applicable)
	pub SourceIp:Option<String>,

	/// Client ID (if applicable)
	pub ClientId:Option<String>,

	/// Event details
	pub Details:String,

	/// Additional metadata
	pub Metadata:HashMap<String, String>,
}
