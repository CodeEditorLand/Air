//! Plugin capability and permission descriptor.

use serde::{Deserialize, Serialize};

/// Plugin capability and permission descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCapability {
	pub name:String,

	pub description:String,

	pub RequiredPermissions:Vec<String>,
}
