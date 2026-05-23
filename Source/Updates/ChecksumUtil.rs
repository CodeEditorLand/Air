#![allow(unused_variables, dead_code, unused_imports)]

//! Standalone checksum calculation helpers for update integrity verification.
//!
//! These free functions mirror the private methods on `UpdateManager` so the
//! same hashing logic can be unit-tested without constructing a full manager
//! and can be reused by other modules (e.g. Downloader) without cross-crate
//! duplication.

use sha2::{Digest, Sha256, Sha512};

/// SHA-256 hex digest of `data`.
pub fn sha256_hex(data:&[u8]) -> String {
	let mut h = Sha256::new();
	h.update(data);
	hex::encode(h.finalize())
}

/// SHA-512 hex digest of `data`.
pub fn sha512_hex(data:&[u8]) -> String {
	let mut h = Sha512::new();
	h.update(data);
	hex::encode(h.finalize())
}

/// MD5 hex digest of `data`.
pub fn md5_hex(data:&[u8]) -> String {
	let digest = md5::compute(data);
	format!("{:x}", digest)
}

/// CRC-32 hex digest of `data` (8 hex digits, zero-padded).
pub fn crc32_hex(data:&[u8]) -> String {
	let crc = crc32fast::hash(data);
	format!("{:08x}", crc)
}

/// SHA-256 hex digest of a file at `path`.
pub async fn sha256_file(path:&std::path::Path) -> Result<String, std::io::Error> {
	let content = tokio::fs::read(path).await?;
	Ok(sha256_hex(&content))
}

/// Verify `data` against an expected hex digest using the named algorithm.
/// Supported algorithms: `sha256`, `sha512`, `md5`, `crc32`.
/// Returns `true` on match, `false` on mismatch or unknown algorithm.
pub fn verify(data:&[u8], algorithm:&str, expected:&str) -> bool {
	let actual = match algorithm.to_lowercase().as_str() {
		"sha256" => sha256_hex(data),
		"sha512" => sha512_hex(data),
		"md5" => md5_hex(data),
		"crc32" => crc32_hex(data),
		_ => return false,
	};
	actual == expected
}
