//! Update information with comprehensive metadata.
//!
//! Full manifest returned by the update server describing an available
//! release including download URLs, checksums, signatures, and delta
//! update information.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::PlatformMetadata::PlatformMetadata;

/// Update information with comprehensive metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
	/// Semantic version (e.g., "1.2.3")
	pub version:String,

	/// Download URL for the update package
	pub download_url:String,

	/// Release notes and changelog
	pub release_notes:String,

	/// Primary checksum (SHA256 recommended)
	pub checksum:String,

	/// Alternative checksums for verification
	pub checksums:HashMap<String, String>,

	/// Size of update package in bytes
	pub size:u64,

	/// When the update was published
	pub published_at:chrono::DateTime<chrono::Utc>,

	/// Whether this update is mandatory
	pub is_mandatory:bool,

	/// Whether update requires application restart
	pub requires_restart:bool,

	/// Minimum compatible version
	pub min_compatible_version:Option<String>,

	/// Delta update URL (if available for incremental update)
	pub delta_url:Option<String>,

	/// Delta update checksum (if available)
	pub delta_checksum:Option<String>,

	/// Delta update size (if available)
	pub delta_size:Option<u64>,

	/// Cryptographic signature (Ed25519 or PGP)
	pub signature:Option<String>,

	/// Platform-specific metadata
	pub platform_metadata:Option<PlatformMetadata>,
}
