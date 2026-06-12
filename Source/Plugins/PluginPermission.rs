//! Plugin permission variants: Filesystem, Network, System, InterPlugin, and Custom.

use serde::{Deserialize, Serialize};

/// Plugin permission
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PluginPermission {
	/// Access filesystem
	Filesystem { read:bool, write:bool, paths:Vec<String> },

	/// Access network
	Network { outbound:bool, inbound:bool, hosts:Vec<String> },

	/// Access system resources
	System { cpu:bool, memory:bool },

	/// Access other plugins
	InterPlugin { plugins:Vec<String>, actions:Vec<String> },

	/// Custom permission
	Custom(String),
}
