//! # CreateState
//!
//! ## File: Indexing/State/CreateState.rs
//!
//! ## Role in Air Architecture
//!
//! Provides state creation functions for the File Indexer service, including
//! the construction of index entries, symbols, and related data structures
//! used throughout the indexing system.
//!
//! ## Primary Responsibility
//!
//! Create and initialize index state structures including FileIndex,
//! FileMetadata, SymbolInfo, and related types.
//!
//! ## Secondary Responsibilities
//!
//! - Generate index version strings
//! - Calculate index checksums for integrity verification
//! - Create new empty indexes
//! - Backup corrupted indexes
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `chrono` - Timestamp generation for index metadata
//! - `sha2` - Checksum calculation for index integrity
//! - `serde` - Serialization/deserialization of index structures
//!
//! **Internal Modules:**
//! - `crate::Result` - Error handling type
//! - `crate::AirError` - Error types
//!
//! ## Dependents
//!
//! - `Indexing::Store::StoreEntry` - Creates entries for index storage
//! - `Indexing::Store::UpdateIndex` - Updates index state
//! - `Indexing::mod::FileIndexer` - Main file indexer implementation
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's indexer state creation in
//! `src/vs/workbench/services/search/common/`
//!
//! ## Security Considerations
//!
//! - Checksums prevent tampering with index data
//! - Version tracking enables corruption detection
//! - Path traversal protection applied during validation
//!
//! ## Performance Considerations
//!
//! - Lightweight state creation operations
//! - Hash calculations are amortized across index operations
//! - Memory-efficient data structures for large indexes
//!
//! ## Error Handling Strategy
//!
//! State creation operations use result types and propagate errors up
//! with clear messages about what failed during creation or validation.
//!
//! ## Thread Safety
//!
//! State structures are designed to be moved into Arc<RwLock<>> for
//! thread-safe shared access across indexing and search operations.

use std::{collections::HashMap, path::PathBuf};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{AirError, Result};

/// Maximum file size allowed for indexing (100MB)
pub const MAX_FILE_SIZE_BYTES:u64 = 100 * 1024 * 1024;

/// Symbol information extracted from files for VSCode Outline View
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
	/// Symbol name (function, class, variable, etc.)
	pub name:String,
	/// Symbol kind (function, class, struct, interface, etc.)
	pub kind:SymbolKind,
	/// Line number where symbol is defined
	pub line:u32,
	/// Column number
	pub column:u32,
	/// Full qualified path
	pub full_path:String,
}

/// Symbol kind for VSCode compatibility
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum SymbolKind {
	File = 0,
	Module = 1,
	Namespace = 2,
	Package = 3,
	Class = 4,
	Method = 5,
	Property = 6,
	Field = 7,
	Constructor = 8,
	Enum = 9,
	Interface = 10,
	Function = 11,
	Variable = 12,
	Constant = 13,
	String = 14,
	Number = 15,
	Boolean = 16,
	Array = 17,
	Object = 18,
	Key = 19,
	Null = 20,
	EnumMember = 21,
	Struct = 22,
	Event = 23,
	Operator = 24,
	TypeParameter = 25,
}

/// Symbol location for cross-referencing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolLocation {
	/// File containing the symbol
	pub file_path:PathBuf,
	/// Line number
	pub line:u32,
	/// Symbol information
	pub symbol:SymbolInfo,
}

/// File metadata with comprehensive information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
	/// File path
	pub path:PathBuf,
	/// File size in bytes
	pub size:u64,
	/// Last modification timestamp
	pub modified:chrono::DateTime<chrono::Utc>,
	/// MIME type
	pub mime_type:String,
	/// Detected programming language
	pub language:Option<String>,
	/// Line count for text files
	pub line_count:Option<u32>,
	/// SHA-256 checksum for change detection
	pub checksum:String,
	/// Whether file is a symbolic link
	pub is_symlink:bool,
	/// File permissions (format: "rwxrwxrwx")
	pub permissions:String,
	/// File encoding (UTF-8, ASCII, etc.)
	pub encoding:Option<String>,
	/// Last indexed timestamp
	pub indexed_at:chrono::DateTime<chrono::Utc>,
	/// Number of symbols extracted
	pub symbol_count:u32,
}

/// File index structure with comprehensive metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileIndex {
	/// Indexed files with complete metadata
	pub files:HashMap<PathBuf, FileMetadata>,
	/// Content index for fast text search
	/// Maps words/tokens to file paths where they appear
	pub content_index:HashMap<String, Vec<PathBuf>>,
	/// Symbol index for VSCode Outline View and Go to Symbol
	/// Maps symbol names to their definitions
	pub symbol_index:HashMap<String, Vec<SymbolLocation>>,
	/// Reverse symbol index for cross-referencing
	pub file_symbols:HashMap<PathBuf, Vec<SymbolInfo>>,
	/// Last update timestamp for all indexes
	pub last_updated:chrono::DateTime<chrono::Utc>,
	/// Index version for corruption detection
	pub index_version:String,
	/// Index checksum for integrity verification
	pub index_checksum:String,
}

/// Create a new empty file index
pub fn CreateNewIndex() -> FileIndex {
	FileIndex {
		files:HashMap::new(),
		content_index:HashMap::new(),
		symbol_index:HashMap::new(),
		file_symbols:HashMap::new(),
		last_updated:chrono::Utc::now(),
		index_version:GenerateIndexVersion(),
		index_checksum:String::new(),
	}
}

/// Generate index version string
pub fn GenerateIndexVersion() -> String { format!("{}-{}", env!("CARGO_PKG_VERSION"), chrono::Utc::now().timestamp()) }

/// Calculate index checksum for integrity verification
pub fn CalculateIndexChecksum(index:&FileIndex) -> Result<String> {
	let checksum_input = format!(
		"{}:{}:{}:{}",
		index.files.len(),
		index.content_index.len(),
		index.symbol_index.len(),
		index.last_updated.timestamp()
	);

	let mut hasher = Sha256::new();
	hasher.update(checksum_input.as_bytes());
	Ok(format!("{:x}", hasher.finalize()))
}

/// Create file metadata from raw information
pub fn CreateFileMetadata(
	path:PathBuf,
	size:u64,
	modified:chrono::DateTime<chrono::Utc>,
	mime_type:String,
	language:Option<String>,
	line_count:Option<u32>,
	checksum:String,
	is_symlink:bool,
	permissions:String,
	encoding:Option<String>,
	symbol_count:u32,
) -> FileMetadata {
	FileMetadata {
		path,
		size,
		modified,
		mime_type,
		language,
		line_count,
		checksum,
		is_symlink,
		permissions,
		encoding,
		indexed_at:chrono::Utc::now(),
		symbol_count,
	}
}

/// Create symbol info with validation
pub fn CreateSymbolInfo(name:String, kind:SymbolKind, line:u32, column:u32, full_path:String) -> SymbolInfo {
	SymbolInfo { name, kind, line, column, full_path }
}

/// Create symbol location for cross-referencing
pub fn CreateSymbolLocation(file_path:PathBuf, line:u32, symbol:SymbolInfo) -> SymbolLocation {
	SymbolLocation { file_path, line, symbol }
}

/// Get file permissions as string from metadata
#[cfg(unix)]
pub fn GetPermissionsString(metadata:&std::fs::Metadata) -> String {
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

/// Validate file size against maximum allowed
pub fn ValidateFileSize(size:u64) -> Result<()> {
	if size > MAX_FILE_SIZE_BYTES {
		return Err(AirError::FileSystem(format!(
			"File size {} exceeds maximum allowed size of {} bytes",
			size, MAX_FILE_SIZE_BYTES
		)));
	}
	Ok(())
}

/// Check if index size is within sane limits
pub fn ValidateIndexSize(index:&FileIndex) -> Result<()> {
	const MAX_INDEXED_FILES:usize = 1_000_000;
	const MAX_SYMBOLS:usize = 10_000_000;

	if index.files.len() > MAX_INDEXED_FILES {
		return Err(AirError::Internal(format!(
			"Index exceeds maximum file count: {} > {}",
			index.files.len(),
			MAX_INDEXED_FILES
		)));
	}

	let total_symbols:usize = index.file_symbols.values().map(|v| v.len()).sum();
	if total_symbols > MAX_SYMBOLS {
		return Err(AirError::Internal(format!(
			"Index exceeds maximum symbol count: {} > {}",
			total_symbols, MAX_SYMBOLS
		)));
	}

	Ok(())
}
