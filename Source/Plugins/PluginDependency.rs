//! Plugin dependency specification: required or optional dependency with
//! version range.

use serde::{Deserialize, Serialize};

/// Plugin dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
	pub PluginId:String,

	pub MinVersion:String,

	pub MaxVersion:Option<String>,

	pub optional:bool,
}
