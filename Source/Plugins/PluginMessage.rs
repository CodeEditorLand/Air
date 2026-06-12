//! Inter-plugin message: sender, receiver, action, data, and timestamp.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Result;

/// Inter-plugin message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMessage {
	pub id:String,

	pub from:String,

	pub to:String,

	pub action:String,

	pub data:serde_json::Value,

	pub timestamp:DateTime<Utc>,
}

impl PluginMessage {
	/// Create a new plugin message
	pub fn new(from:String, to:String, action:String, data:serde_json::Value) -> Self {
		Self { id:Uuid::new_v4().to_string(), from, to, action, data, timestamp:Utc::now() }
	}

	/// Validate message format and content
	pub fn validate(&self) -> Result<()> {
		if self.id.is_empty() {
			return Err(crate::AirError::Plugin("Message ID cannot be empty".to_string()));
		}

		if self.from.is_empty() {
			return Err(crate::AirError::Plugin("Message sender cannot be empty".to_string()));
		}

		if self.to.is_empty() {
			return Err(crate::AirError::Plugin("Message recipient cannot be empty".to_string()));
		}

		if self.action.is_empty() {
			return Err(crate::AirError::Plugin("Message action cannot be empty".to_string()));
		}

		if self.action.len() > 100 {
			return Err(crate::AirError::Plugin("Message action too long".to_string()));
		}

		Ok(())
	}
}
