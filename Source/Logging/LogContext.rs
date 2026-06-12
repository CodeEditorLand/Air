//! Structured logging context with request ID and trace propagation.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Result, Utility::GenerateRequestId, dev_log};

/// Context for structured logging with request IDs and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogContext {
	pub RequestId:String,

	pub TraceId:String,

	pub SpanId:String,

	pub UserId:Option<String>,

	pub SessionId:Option<String>,

	pub Operation:String,

	pub Metadata:HashMap<String, String>,
}

impl LogContext {
	/// Create a new log context
	pub fn New(Operation:impl Into<String>) -> Self {
		let RequestId = crate::Utility::GenerateRequestId();

		let TraceId = crate::Utility::GenerateRequestId();

		let SpanId = uuid::Uuid::new_v4().to_string();

		Self {
			RequestId,

			TraceId,

			SpanId,

			UserId:None,

			SessionId:None,

			Operation:Operation.into(),

			Metadata:HashMap::new(),
		}
	}

	/// Validate log context for required fields
	pub fn Validate(&self) -> Result<()> {
		if self.RequestId.is_empty() {
			return Err("RequestId cannot be empty".into());
		}

		if self.TraceId.is_empty() {
			return Err("TraceId cannot be empty".into());
		}

		if self.Operation.is_empty() {
			return Err("Operation cannot be empty".into());
		}

		Ok(())
	}

	/// Set user ID in context
	pub fn WithUserId(mut self, UserId:String) -> Self {
		self.UserId = Some(UserId);

		self
	}

	/// Set session ID in context
	pub fn WithSessionId(mut self, SessionId:String) -> Self {
		self.SessionId = Some(SessionId);

		self
	}

	/// Add metadata to context
	pub fn WithMetadata(mut self, Key:String, Value:String) -> Self {
		self.Metadata.insert(Key, Value);

		self
	}

	/// Add multiple metadata entries
	pub fn WithMetadataMap(mut self, Metadata:HashMap<String, String>) -> Self {
		self.Metadata.extend(Metadata);

		self
	}
}

thread_local! {

	static LOG_CONTEXT: std::cell::RefCell<Option<LogContext>> = std::cell::RefCell::new(None);
}

pub fn SetLogContext(Context:LogContext) {
	if let Err(e) = Context.Validate() {
		dev_log!("air", "error: [Logging] Invalid log context provided: {:?}", e);

		return;
	}

	LOG_CONTEXT.with(|ctx| {
		*ctx.borrow_mut() = Some(Context);
	});
}

/// Get the current log context
pub fn GetLogContext() -> Option<LogContext> { LOG_CONTEXT.with(|Context| Context.borrow().clone()) }

/// Clear the log context for the current thread
pub fn ClearLogContext() {
	LOG_CONTEXT.with(|Context| {
		*Context.borrow_mut() = None;
	});
}
