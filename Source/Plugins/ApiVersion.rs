//! API version compatibility management for the plugin system.
//!
//! `ApiVersion` represents a semver triple with an optional pre-release tag.
//! Compatibility follows the semver major-version rule: plugins with the same
//! major version and a minor version ≥ the host's are considered compatible.
//!
//! `ApiVersionManager` tracks the current host version and the set of
//! explicitly registered compatible versions.

use serde::{Deserialize, Serialize};

use crate::Result;

/// Semver-style API version.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiVersion {

	pub major:u32,

	pub minor:u32,

	pub patch:u32,

	pub PreRelease:Option<String>,
}

impl ApiVersion {

	/// The current host API version.
	pub fn current() -> Self { Self { major:1, minor:0, patch:0, PreRelease:None } }

	/// Parse from `"major.minor.patch"` or `"major.minor.patch.pre"`.
	pub fn parse(version:&str) -> Result<Self> {
		let parts:Vec<&str> = version.split('.').collect();

		if parts.len() < 3 {
			return Err(crate::AirError::Plugin("Invalid version format".to_string()));
		}

		Ok(Self {
			major:parts[0]
				.parse()
				.map_err(|_| crate::AirError::Plugin("Invalid major version".to_string()))?,
			minor:parts[1]
				.parse()
				.map_err(|_| crate::AirError::Plugin("Invalid minor version".to_string()))?,
			patch:parts[2]
				.parse()
				.map_err(|_| crate::AirError::Plugin("Invalid patch version".to_string()))?,
			PreRelease:parts.get(3).map(|s| s.to_string()),
		})
	}

	/// `true` when `other` is compatible: same major, minor ≥ self.minor.
	pub fn IsCompatible(&self, other:&ApiVersion) -> bool { self.major == other.major && other.minor >= self.minor }
}

/// Tracks the current host API version and a set of compatible peer versions.
pub struct ApiVersionManager {

	CurrentVersion:ApiVersion,

	CompatibleVersions:Vec<ApiVersion>,
}

impl ApiVersionManager {

	pub fn new() -> Self {
		let current = ApiVersion::current();

		Self { CompatibleVersions:vec![current.clone()], CurrentVersion:current }
	}

	pub fn current(&self) -> &ApiVersion { &self.CurrentVersion }

	pub fn IsCompatible(&self, version:&ApiVersion) -> bool { self.CurrentVersion.IsCompatible(version) }

	/// Register `version` as compatible if it passes the compatibility check.
	pub fn register_compatible(&mut self, version:ApiVersion) {
		if self.IsCompatible(&version) && !self.CompatibleVersions.contains(&version) {
			self.CompatibleVersions.push(version);
		}
	}
}

impl Default for ApiVersionManager {

	fn default() -> Self { Self::new() }
}
