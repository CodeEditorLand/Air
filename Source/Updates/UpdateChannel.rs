//! Update channel configuration.
//!
//! Controls which release channel the update manager queries:
//! Stable, Insiders, or Preview.

use serde::{Deserialize, Serialize};

/// Update channel configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum UpdateChannel {
	Stable,

	Insiders,

	Preview,
}

impl UpdateChannel {
	pub fn as_str(&self) -> &'static str {
		match self {
			UpdateChannel::Stable => "stable",

			UpdateChannel::Insiders => "insiders",

			UpdateChannel::Preview => "preview",
		}
	}
}
