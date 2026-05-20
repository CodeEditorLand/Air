#![allow(non_snake_case, unused_variables, dead_code, unused_imports)]

//! Plugin event bus: publish/subscribe channel for plugin lifecycle events.
//!
//! Handlers register via `register_handler` and receive every subsequent
//! `emit` call. Errors from individual handlers are logged but do not abort
//! delivery to other handlers.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{Result, dev_log};

/// Events published through the bus during the plugin lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginEvent {
	Loaded { plugin_id:String },
	Started { plugin_id:String },
	Stopped { plugin_id:String },
	Unloaded { plugin_id:String },
	Error { plugin_id:String, error:String },
	Message { from:String, to:String, action:String },
	ConfigChanged { old:serde_json::Value, new:serde_json::Value },
}

/// Async handler trait for plugin events.
#[async_trait]
pub trait PluginEventHandler: Send + Sync {
	async fn Event(&self, event:&PluginEvent) -> Result<()>;
}

/// Fan-out event bus: each `emit` delivers to all registered handlers.
pub struct PluginEventBus {
	handlers:Arc<RwLock<Vec<Box<dyn PluginEventHandler>>>>,
}

impl PluginEventBus {
	pub fn new() -> Self { Self { handlers:Arc::new(RwLock::new(vec![])) } }

	pub async fn register_handler(&self, handler:Box<dyn PluginEventHandler>) {
		self.handlers.write().await.push(handler);
	}

	/// Deliver `event` to every registered handler. Handler errors are logged
	/// and do not prevent delivery to subsequent handlers.
	pub async fn emit(&self, event:PluginEvent) {
		let handlers = self.handlers.read().await;
		for handler in handlers.iter() {
			if let Err(Error) = handler.Event(&event).await {
				dev_log!("extensions", "error: [PluginEventBus] Event handler error: {}", Error);
			}
		}
	}
}

impl Default for PluginEventBus {
	fn default() -> Self { Self::new() }
}
