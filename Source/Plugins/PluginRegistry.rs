//! Plugin registry entry: wraps a Plugin with state, timestamps, and sandbox config.

use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::Plugins::Plugin::Plugin;
use crate::Plugins::PluginSandboxConfig::PluginSandboxConfig;
use crate::Plugins::PluginState::PluginState;

/// Plugin registry entry
pub struct PluginRegistry {
	pub plugin:Arc<Box<dyn Plugin>>,

	pub state:PluginState,

	pub StartedAt:Option<DateTime<Utc>>,

	pub LoadedAt:Option<DateTime<Utc>>,

	pub error:Option<String>,

	pub sandbox:PluginSandboxConfig,
}
