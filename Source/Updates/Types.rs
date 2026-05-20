#![allow(non_snake_case, unused_variables, dead_code, unused_imports)]

//! Value types for the update lifecycle: channels, status, installation
//! states, update manifests, platform metadata, and telemetry records.
//!
//! These structs are shared between `UpdateManager` methods, IPC handlers
//! (Mountain ↔ Air), and the test suite. Keeping them in one file makes the
//! shape of the update domain legible without reading the 2600-line
//! implementation.

use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

/// Update distribution channel.
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

/// Supported OS package formats.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum PackageFormat {
	WindowsExe,
	MacOsDmg,
	LinuxAppImage,
	LinuxDeb,
	LinuxRpm,
}

/// Snapshot of a rollback point created before applying an update.
#[derive(Debug, Clone)]
pub struct RollbackState {
	pub version:String,
	pub backup_path:PathBuf,
	pub timestamp:chrono::DateTime<chrono::Utc>,
	pub checksum:String,
}

/// Fine-grained installation lifecycle state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InstallationStatus {
	NotStarted,
	CheckingPrerequisites,
	Downloading,
	Paused,
	VerifyingSignature,
	VerifyingChecksums,
	Staging,
	CreatingBackup,
	Installing,
	Completed,
	RollingBack,
	Failed(String),
}

impl InstallationStatus {
	pub fn is_cancellable(&self) -> bool {
		matches!(
			self,
			InstallationStatus::Downloading
				| InstallationStatus::Paused
				| InstallationStatus::Staging
				| InstallationStatus::NotStarted
		)
	}

	pub fn is_error(&self) -> bool { matches!(self, InstallationStatus::Failed(_)) }

	pub fn is_in_progress(&self) -> bool {
		matches!(
			self,
			InstallationStatus::CheckingPrerequisites
				| InstallationStatus::Downloading
				| InstallationStatus::VerifyingSignature
				| InstallationStatus::VerifyingChecksums
				| InstallationStatus::Staging
				| InstallationStatus::CreatingBackup
				| InstallationStatus::Installing
		)
	}
}

/// Live status snapshot surfaced to Mountain and IPC callers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatus {
	pub last_check:Option<chrono::DateTime<chrono::Utc>>,
	pub update_available:bool,
	pub current_version:String,
	pub available_version:Option<String>,
	pub download_progress:Option<f32>,
	pub installation_status:InstallationStatus,
	pub update_channel:UpdateChannel,
	pub update_size:Option<u64>,
	pub release_notes:Option<String>,
	pub requires_restart:bool,
	pub download_speed:Option<f64>,
	pub eta_seconds:Option<u64>,
	pub last_error:Option<String>,
}

/// Manifest returned by the update server for an available release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
	pub version:String,
	pub download_url:String,
	pub release_notes:String,
	pub checksum:String,
	pub checksums:HashMap<String, String>,
	pub size:u64,
	pub published_at:chrono::DateTime<chrono::Utc>,
	pub is_mandatory:bool,
	pub requires_restart:bool,
	pub min_compatible_version:Option<String>,
	pub delta_url:Option<String>,
	pub delta_checksum:Option<String>,
	pub delta_size:Option<u64>,
	pub signature:Option<String>,
	pub platform_metadata:Option<PlatformMetadata>,
}

/// Platform-specific fields inside an `UpdateInfo`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformMetadata {
	pub package_format:String,
	pub install_instructions:Vec<String>,
	pub required_disk_space:u64,
	pub requires_admin:bool,
	pub additional_params:HashMap<String, serde_json::Value>,
}

/// Analytics record emitted after every update operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTelemetry {
	pub event_id:String,
	pub current_version:String,
	pub target_version:String,
	pub channel:String,
	pub platform:String,
	pub operation:String,
	pub success:bool,
	pub duration_ms:u64,
	pub download_size:Option<u64>,
	pub error_message:Option<String>,
	pub timestamp:chrono::DateTime<chrono::Utc>,
}
