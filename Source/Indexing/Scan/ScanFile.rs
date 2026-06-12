//! # ScanFile
//!
//! ## File: Indexing/Scan/ScanFile.rs
//!
//! ## Role in Air Architecture
//!
//! Provides individual file scanning functionality for the File Indexer
//! service, handling reading, metadata extraction, and categorization of files
//! for indexing.
//!
//! ## Primary Responsibility
//!
//! Scan individual files to extract metadata, content, and prepare them for
//! indexing operations.
//!
//! ## Secondary Responsibilities
//!
//! - File access validation and permission checking
//! - Encoding detection for text files
//! - Language detection for code files
//! - File size validation
//! - Symbolic link detection
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `tokio` - Async file I/O operations
//! - `sha2` - Checksum calculation for file integrity
//!
//! **Internal Modules:**
//! - `crate::Result` - Error handling type
//! - `crate::AirError` - Error types
//! - `crate::Configuration::AirConfiguration::IndexingConfig` - Indexing configuration
//! - `super::super::State::CreateState` - State structure definitions
//! - `super::Process::ProcessContent` - Content processing operations
//!
//! ## Dependents
//!
//! - `Indexing::Scan::ScanDirectory` - Batch file processing
//! - `Indexing::Watch::WatchFile` - Individual file change handling
//! - `Indexing::mod::FileIndexer` - Main file indexer implementation
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's file scanning in
//! `src/vs/workbench/services/files/`
//!
//! ## Security Considerations
//!
//! - Path canonicalization before access
//! - File size limits enforced
//! - Timeout protection for I/O operations
//! - Permission checking before reads
//!
//! ## Performance Considerations
//!
//! - Asynchronous file reading
//! - Batch processing operations
//! - Memory-efficient streaming for large files
//! - Cached metadata when available
//!
//! ## Error Handling Strategy
//!
//! File scanning returns Results with detailed error messages about
//! why a file cannot be scanned or accessed. Errors are logged and
//! individual file failures don't halt batch operations.
//!
//! ## Thread Safety
//!
//! File scanning operations are designed for parallel execution and
use std::{
	path::PathBuf,
	time::{Duration, Instant},
};

/// produce results that can be safely merged into shared state.
use crate::dev_log;
use crate::{
	AirError,
	Configuration::AirConfiguration::IndexingConfig,
	Indexing::{
		Process::{
			ExtractSymbols::ExtractSymbols,
			ProcessContent::{DetectEncoding, DetectLanguage, DetectMimeType},
		},
		State::CreateState::{FileMetadata, SymbolInfo},
	},
	Result,
};

/// Index a single file internally with comprehensive validation
///
/// Called by parallel tasks during directory scanning
/// and includes:
/// - File metadata extraction
/// - Size validation
/// - SHA-256 checksum calculation
/// - Encoding detection
/// - MIME type detection
/// - Language detection
/// - Symbol extraction for code files
pub async fn IndexFileInternal(
	file_path:&PathBuf,

	config:&IndexingConfig,

	_patterns:&[String],
) -> Result<(FileMetadata, Vec<SymbolInfo>)> {
	let start_time = Instant::now();

	// Get file metadata with error handling
	let metadata = std::fs::metadata(file_path)
		.map_err(|e| AirError::FileSystem(format!("Failed to get file metadata: {}", e)))?;

	// Get modified time
	let modified = metadata
		.modified()
		.map_err(|e| AirError::FileSystem(format!("Failed to get modification time: {}", e)))?;

	let modified_time = chrono::DateTime::<chrono::Utc>::from(modified);

	// Check if file size exceeds limit
	let file_size = metadata.len();

	if file_size > config.MaxFileSizeMb as u64 * 1024 * 1024 {
		return Err(AirError::FileSystem(format!(
			"File size {} exceeds limit {} MB",
			file_size, config.MaxFileSizeMb
		)));
	}

	// File read with timeout protection
	let content = tokio::time::timeout(Duration::from_secs(30), tokio::fs::read(file_path))
		.await
		.map_err(|_| AirError::FileSystem(format!("Timeout reading file: {} (30s limit)", file_path.display())))?
		.map_err(|e| AirError::FileSystem(format!("Failed to read file: {}", e)))?;

	// Check for symbolic link
	let is_symlink = std::fs::symlink_metadata(file_path)
		.map(|m| m.file_type().is_symlink())
		.unwrap_or(false);

	// Calculate SHA-256 checksum
	let checksum = CalculateChecksum(&content);

	// Detect file encoding
	let encoding = DetectEncoding(&content);

	// Detect MIME type
	let mime_type = DetectMimeType(file_path, &content);

	// Detect programming language
	let language = DetectLanguage(file_path);

	// Count lines for text files
	let line_count = if mime_type.starts_with("text/") {
		Some(content.iter().filter(|&&b| b == b'\n').count() as u32 + 1)
	} else {
		None
	};

	// Extract symbols from code for VSCode Outline View
	let symbols = if let Some(lang) = &language {
		ExtractSymbols(file_path, &content, lang).await?
	} else {
		Vec::new()
	};

	let permissions = GetPermissionsString(&metadata);

	let elapsed = start_time.elapsed();

	dev_log!(
		"indexing",
		"indexed {} in {}ms ({} symbols)",
		file_path.display(),
		elapsed.as_millis(),
		symbols.len()
	);

	Ok((
		FileMetadata {
			path:file_path.clone(),
			size:file_size,
			modified:modified_time,
			mime_type,
			language,
			line_count,
			checksum,
			is_symlink,
			permissions,
			encoding,
			indexed_at:chrono::Utc::now(),
			symbol_count:symbols.len() as u32,
		},
		symbols,
	))
}

/// Validate file access and permissions before scanning
pub async fn ValidateFileAccess(file_path:&PathBuf) -> bool {
	tokio::task::spawn_blocking({
		let file_path = file_path.to_path_buf();

		move || {
			// Try to read file metadata
			let can_access = std::fs::metadata(&file_path).is_ok();

			if can_access {
				// Try to open file for reading
				std::fs::File::open(&file_path).is_ok()
			} else {
				false
			}
		}
	})
	.await
	.unwrap_or(false)
}

/// Calculate SHA-256 checksum for file content
pub fn CalculateChecksum(content:&[u8]) -> String {
	// sha2 0.11 moved `Digest::finalize()` to `hybrid_array::Array`, which has
	// no `LowerHex` impl (the old `GenericArray` did). `hex::encode` over the
	// byte output is the drop-in replacement - same lowercase hex string,
	// same length. `hex` is already a workspace dependency of Air.
	use sha2::{Digest, Sha256};

	let mut hasher = Sha256::new();

	hasher.update(content);

	hex::encode(hasher.finalize())
}

/// Get file permissions as string
#[cfg(unix)]
pub fn GetPermissionsString(metadata:&std::fs::Metadata) -> String {
	use std::os::unix::fs::PermissionsExt;

	let mode = metadata.permissions().mode();

	let mut perms = String::new();

	// Read permission
	perms.push(if mode & 0o400 != 0 { 'r' } else { '-' });

	// Write permission
	perms.push(if mode & 0o200 != 0 { 'w' } else { '-' });

	// Execute permission
	perms.push(if mode & 0o100 != 0 { 'x' } else { '-' });

	// Group permissions
	perms.push(if mode & 0o040 != 0 { 'r' } else { '-' });

	perms.push(if mode & 0o020 != 0 { 'w' } else { '-' });

	perms.push(if mode & 0o010 != 0 { 'x' } else { '-' });

	// Other permissions
	perms.push(if mode & 0o004 != 0 { 'r' } else { '-' });

	perms.push(if mode & 0o002 != 0 { 'w' } else { '-' });

	perms.push(if mode & 0o001 != 0 { 'x' } else { '-' });

	perms
}

/// Get file permissions as string for non-Unix systems
#[cfg(not(unix))]
pub fn GetPermissionsString(_metadata:&std::fs::Metadata) -> String { "--------".to_string() }

/// Scan file and return just the metadata (without symbols)
pub async fn ScanFileMetadata(file_path:&PathBuf) -> Result<FileMetadata> {
	let metadata = std::fs::metadata(file_path)
		.map_err(|e| AirError::FileSystem(format!("Failed to get file metadata: {}", e)))?;

	let modified = metadata
		.modified()
		.map_err(|e| AirError::FileSystem(format!("Failed to get modification time: {}", e)))?;

	let modified_time = chrono::DateTime::<chrono::Utc>::from(modified);

	Ok(FileMetadata {
		path:file_path.clone(),
		size:metadata.len(),
		modified:modified_time,
		mime_type:"application/octet-stream".to_string(),
		language:None,
		line_count:None,
		checksum:String::new(),
		is_symlink:metadata.file_type().is_symlink(),
		permissions:GetPermissionsString(&metadata),
		encoding:None,
		indexed_at:chrono::Utc::now(),
		symbol_count:0,
	})
}

/// Check if file has been modified since last indexed
pub fn FileModifiedSince(file_path:&PathBuf, last_indexed:chrono::DateTime<chrono::Utc>) -> Result<bool> {
	let metadata = std::fs::metadata(file_path)
		.map_err(|e| AirError::FileSystem(format!("Failed to get file metadata: {}", e)))?;

	let modified = metadata
		.modified()
		.map_err(|e| AirError::FileSystem(format!("Failed to get modification time: {}", e)))?;

	let modified_time = chrono::DateTime::<chrono::Utc>::from(modified);

	Ok(modified_time > last_indexed)
}

/// Get file size with error handling
pub async fn GetFileSize(file_path:&PathBuf) -> Result<u64> {
	tokio::task::spawn_blocking({
		let file_path = file_path.to_path_buf();

		move || {
			let metadata = std::fs::metadata(&file_path)
				.map_err(|e| AirError::FileSystem(format!("Failed to get file metadata: {}", e)))?;

			Ok(metadata.len())
		}
	})
	.await?
}

/// Check if file is text-based (likely to be code or documentation)
pub fn IsTextFile(metadata:&FileMetadata) -> bool {
	metadata.mime_type.starts_with("text/")
		|| metadata.mime_type.contains("json")
		|| metadata.mime_type.contains("xml")
		|| metadata.mime_type.contains("yaml")
		|| metadata.mime_type.contains("toml")
		|| metadata.language.is_some()
}

/// Check if file is binary (not suitable for indexing)
pub fn IsBinaryFile(metadata:&FileMetadata) -> bool {
	!IsTextFile(metadata)
		|| metadata.mime_type == "application/octet-stream"
		|| metadata.mime_type == "application/zip"
		|| metadata.mime_type == "application/x-tar"
		|| metadata.mime_type == "application/x-gzip"
		|| metadata.mime_type == "application/x-bzip2"
}
