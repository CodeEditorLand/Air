//! Installation status with detailed state tracking.
//!
//! Models the complete lifecycle of an update operation from initial
//! check through download, verification, installation, and rollback.

use serde::{Deserialize, Serialize};

/// Installation status with detailed state tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InstallationStatus {
	/// No update operation in progress
	NotStarted,

	/// Verifying disk space and prerequisites
	CheckingPrerequisites,

	/// Downloading update package
	Downloading,

	/// Download paused (resumable)
	Paused,

	/// Verifying cryptographic signatures
	VerifyingSignature,

	/// Verifying checksums (SHA256, MD5, etc.)
	VerifyingChecksums,

	/// Staging update for pre-installation verification
	Staging,

	/// Creating backup before applying update
	CreatingBackup,

	/// Installing update
	Installing,

	/// Installation completed, awaiting restart
	Completed,

	/// Rolling back due to installation failure
	RollingBack,

	/// Installation failed with error message
	Failed(String),
}

impl InstallationStatus {
	/// Check if the current status allows cancellation
	pub fn is_cancellable(&self) -> bool {
		matches!(
			self,
			InstallationStatus::Downloading
				| InstallationStatus::Paused
				| InstallationStatus::Staging
				| InstallationStatus::NotStarted
		)
	}

	/// Check if the current status represents an error
	pub fn is_error(&self) -> bool { matches!(self, InstallationStatus::Failed(_)) }

	/// Check if the current status represents progress
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
