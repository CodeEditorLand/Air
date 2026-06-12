//! Plugin state tracking: Unloaded, Loaded, Starting, Running, Stopping, Error.

use serde::{Deserialize, Serialize};

/// Plugin state tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginState {
	#[serde(rename = "unloaded")]
	Unloaded,

	#[serde(rename = "loaded")]
	Loaded,

	#[serde(rename = "starting")]
	Starting,

	#[serde(rename = "running")]
	Running,

	#[serde(rename = "stopping")]
	Stopping,

	#[serde(rename = "error")]
	Error,
}
