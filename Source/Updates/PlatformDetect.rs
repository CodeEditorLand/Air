#![allow(unused_variables, dead_code, unused_imports)]

//! Compile-time platform/architecture detection for update packaging.
//!
//! `detect_platform()` returns a `PlatformInfo` describing the current OS,
//! CPU architecture, and the appropriate package format for update binaries.
//! All values are resolved with `cfg!` at compile time so there is no runtime
//! overhead and the logic is testable per target triple.

/// Resolved platform description used to choose an update package format.
#[derive(Debug, Clone)]
pub struct PlatformInfo {
	pub platform:&'static str,
	pub arch:&'static str,
	pub package_format:PackageFormat,
}

/// OS-native package format for update delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageFormat {
	WindowsExe,
	MacOsDmg,
	LinuxAppImage,
	LinuxDeb,
	LinuxRpm,
}

impl PackageFormat {
	pub fn extension(&self) -> &'static str {
		match self {
			PackageFormat::WindowsExe => "exe",
			PackageFormat::MacOsDmg => "dmg",
			PackageFormat::LinuxAppImage => "AppImage",
			PackageFormat::LinuxDeb => "deb",
			PackageFormat::LinuxRpm => "rpm",
		}
	}
}

/// Detect the current compile-target platform and preferred package format.
pub fn detect_platform() -> PlatformInfo {
	let platform = if cfg!(target_os = "windows") {
		"windows"
	} else if cfg!(target_os = "macos") {
		"macos"
	} else if cfg!(target_os = "linux") {
		"linux"
	} else {
		"unknown"
	};

	let arch = if cfg!(target_arch = "x86_64") {
		"x64"
	} else if cfg!(target_arch = "aarch64") {
		"arm64"
	} else if cfg!(target_arch = "x86") {
		"ia32"
	} else {
		"unknown"
	};

	let package_format = match (platform, arch) {
		("windows", _) => PackageFormat::WindowsExe,
		("macos", _) => PackageFormat::MacOsDmg,
		("linux", _) => PackageFormat::LinuxAppImage,
		_ => PackageFormat::LinuxAppImage,
	};

	PlatformInfo { platform, arch, package_format }
}
