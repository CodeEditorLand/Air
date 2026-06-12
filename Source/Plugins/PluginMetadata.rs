//! Plugin metadata: id, name, version, description, author, version
//! compatibility, dependencies, and capabilities.

use serde::{Deserialize, Serialize};

use crate::Plugins::PluginDependency::PluginDependency;

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
	pub id:String,

	pub name:String,

	pub version:String,

	pub description:String,

	pub author:String,

	pub MinAirVersion:String,

	pub MaxAirVersion:Option<String>,

	pub dependencies:Vec<PluginDependency>,

	pub capabilities:Vec<String>,
}
