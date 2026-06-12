use std::path::Path;

use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::{AirError, Result};

/// Checksum verification for file integrity
pub struct ChecksumVerifier;

impl ChecksumVerifier {
	/// Create a new ChecksumVerifier
	pub fn New() -> Self { Self }

	/// Calculate SHA-256 checksum of a file
	pub async fn CalculateSha256(&self, file_path:&Path) -> Result<String> {
		let content = tokio::fs::read(file_path)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to read file: {}", e)))?;

		let mut hasher = Sha256::new();

		hasher.update(&content);

		// sha2 0.11: see note in Indexing/Scan/ScanFile.rs - `hex::encode`
		// replaces the removed `LowerHex` impl on the digest output.
		let checksum = hex::encode(hasher.finalize());

		Ok(checksum)
	}

	/// Verify file checksum with constant-time comparison
	pub async fn VerifySha256(&self, file_path:&Path, expected_checksum:&str) -> Result<bool> {
		let actual = self.CalculateSha256(file_path).await?;

		// Use constant-time comparison
		let actual_bytes = actual.as_bytes();

		let expected_bytes = expected_checksum.as_bytes();

		let result = actual_bytes.ct_eq(expected_bytes);

		Ok(result.into())
	}

	/// Calculate checksum from bytes
	pub fn CalculateSha256Bytes(&self, data:&[u8]) -> String {
		let mut hasher = Sha256::new();

		hasher.update(data);

		hex::encode(hasher.finalize())
	}

	/// Calculate MD5 checksum (legacy support)
	pub async fn CalculateMd5(&self, file_path:&Path) -> Result<String> {
		let content = tokio::fs::read(file_path)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to read file: {}", e)))?;

		let digest = md5::compute(&content);

		Ok(format!("{:x}", digest))
	}

	/// Constant-time compare two checksum strings
	pub fn ConstantTimeCompare(&self, a:&str, b:&str) -> bool {
		if a.len() != b.len() {
			return false;
		}

		a.as_bytes().ct_eq(b.as_bytes()).into()
	}
}
