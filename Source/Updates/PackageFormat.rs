//! Supported package formats by platform.
//!
//! Each variant corresponds to a platform-native packaging format
//! used for update delivery.

/// Supported package formats by platform
#[derive(Debug, Clone, Copy)]
pub(super) enum PackageFormat {
	WindowsExe,

	MacOsDmg,

	LinuxAppImage,

	LinuxDeb,

	LinuxRpm,
}
