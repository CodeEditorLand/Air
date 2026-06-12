//! Platform-specific update configuration.
//!
//! Stores the current platform, architecture, and preferred package
//! format for update delivery.

use super::PackageFormat::PackageFormat;

/// Platform-specific update configuration
#[derive(Debug, Clone)]
pub(super) struct PlatformConfig {
	pub(super) platform: String,

	pub(super) arch: String,

	pub(super) package_format: PackageFormat,
}
