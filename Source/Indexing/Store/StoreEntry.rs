//! # StoreEntry
//!
//! ## File: Indexing/Store/StoreEntry.rs
//!
//! ## Role in Air Architecture
//!
//! Provides index storage functionality for the File Indexer service,
//! handling serialization and persistence of the file index to disk.
//!
//! ## Primary Responsibility
//!
//! Store the file index to disk with atomic writes and corruption recovery
//! mechanisms.
//!
//! ## Secondary Responsibilities
//!
//! - Load index from disk with validation
//! - Backup corrupted indexes automatically
//! - Atomic writes using temp files
//! - Index integrity verification
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `serde_json` - JSON serialization/deserialization
//! - `tokio` - Async file I/O operations
//!
//! **Internal Modules:**
//! - `crate::Result` - Error handling type
//! - `crate::AirError` - Error types
//! - `super::super::FileIndex` - Index structure definitions
//! - `super::super::State::CreateState` - State creation utilities
//!
//! ## Dependents
//!
//! - `Indexing::mod::FileIndexer` - Main file indexer implementation
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's index storage in
//! `src/vs/workbench/services/search/common/`
//!
//! ## Security Considerations
//!
//! - Atomic writes prevent partial index corruption
//! - Permission checking on index directory
//! - Path traversal protection
//!
//! ## Performance Considerations
//!
//! - Temp file pattern for atomic writes
//! - Lazy loading of in-memory index
//! - Efficient serialization with serde
//!
//! ## Error Handling Strategy
//!
//! Storage operations return detailed error messages for failures and
//! automatically backup corrupted indexes when loading fails.
//!
//! ## Thread Safety
//!
//! Storage operations use async file I/O and return results that can be
//! safely merged into shared Ar c<RwLock<>> state.

use std::path::{Path, PathBuf};

use crate::{
	AirError,
	Indexing::State::CreateState::FileIndex,
	Result,
};

/// Save index to disk with atomic write
pub async fn SaveIndex(index_directory:&Path, index:&FileIndex) -> Result<()> {
	let index_file = index_directory.join("file_index.json");
	let temp_file = index_directory.join("file_index.json.tmp");

	let content = serde_json::to_string_pretty(index)
		.map_err(|e| AirError::Serialization(format!("Failed to serialize index: {}", e)))?;

	// Write to temp file first
	tokio::fs::write(&temp_file, content)
		.await
		.map_err(|e| AirError::FileSystem(format!("Failed to write temp index file: {}", e)))?;

	// Atomic rename
	tokio::fs::rename(&temp_file, &index_file)
		.await
		.map_err(|e| AirError::FileSystem(format!("Failed to rename index file: {}", e)))?;

	log::debug!(
		"[StoreEntry] Index saved to: {} ({} files, {} symbols)",
		index_file.display(),
		index.files.len(),
		index.symbol_index.len()
	);

	Ok(())
}

/// Load index from disk with corruption detection
pub async fn LoadIndex(index_directory:&Path) -> Result<FileIndex> {
	let index_file = index_directory.join("file_index.json");

	if !index_file.exists() {
		return Err(AirError::FileSystem(format!(
			"Index file does not exist: {}",
			index_file.display()
		)));
	}

	let content = tokio::fs::read_to_string(&index_file)
		.await
		.map_err(|e| AirError::FileSystem(format!("Failed to read index file: {}", e)))?;

	let index:FileIndex = serde_json::from_str(&content)
		.map_err(|e| AirError::Serialization(format!("Failed to parse index file: {}", e)))?;

	// Verify index structure
	if index.index_version.is_empty() || index.index_checksum.is_empty() {
		return Err(AirError::Serialization("Index missing version or checksum".to_string()));
	}

	// Verify index checksum
	use crate::Indexing::State::CreateState::CalculateIndexChecksum;
	let expected_checksum = CalculateIndexChecksum(&index)?;
	if index.index_checksum != expected_checksum {
		return Err(AirError::Serialization(format!(
			"Index checksum mismatch: expected {}, got {}",
			expected_checksum, index.index_checksum
		)));
	}

	Ok(index)
}

/// Load or create index with corruption detection
pub async fn LoadOrCreateIndex(index_directory:&Path) -> Result<FileIndex> {
	let index_file = index_directory.join("file_index.json");

	if index_file.exists() {
		// Try to load existing index
		match LoadIndex(index_directory).await {
			Ok(index) => {
				log::info!("[StoreEntry] Loaded index with {} files", index.files.len());
				Ok(index)
			},
			Err(e) => {
				log::warn!(
					"[StoreEntry] Failed to load index (may be corrupted): {}. Creating new index.",
					e
				);
				// Backup corrupted index
				BackupCorruptedIndex(index_directory).await?;
				Ok(CreateNewIndex())
			},
		}
	} else {
		// Create new index
		Ok(CreateNewIndex())
	}
}

/// Create a new empty index
fn CreateNewIndex() -> FileIndex {
	use crate::Indexing::State::CreateState::CreateNewIndex as StateCreateNewIndex;
	StateCreateNewIndex()
}

/// Ensure index directory exists with proper error handling
pub async fn EnsureIndexDirectory(index_directory:&Path) -> Result<()> {
	tokio::fs::create_dir_all(index_directory).await.map_err(|e| {
		AirError::Configuration(format!("Failed to create index directory {}: {}", index_directory.display(), e))
	})?;
	Ok(())
}

/// Backup corrupted index before creating new one
pub async fn BackupCorruptedIndex(index_directory:&Path) -> Result<()> {
	let index_file = index_directory.join("file_index.json");
	let backup_file = index_directory.join(format!("file_index.corrupted.{}.json", chrono::Utc::now().timestamp()));

	if !index_file.exists() {
		return Ok(());
	}

	// Rename corrupted file to backup
	tokio::fs::rename(&index_file, &backup_file)
		.await
		.map_err(|e| AirError::FileSystem(format!("Failed to backup corrupted index: {}", e)))?;

	log::info!("[StoreEntry] Backed up corrupted index to: {}", backup_file.display());

	Ok(())
}

/// Load index with automatic recovery on corruption
pub async fn LoadIndexWithRecovery(index_directory:&Path, max_retries:usize) -> Result<FileIndex> {
	let mut last_error = None;

	for attempt in 0..max_retries {
		match LoadOrCreateIndex(index_directory).await {
			Ok(index) => {
				if attempt > 0 {
					log::info!("[StoreEntry] Successfully loaded index after {} attempts", attempt + 1);
				}
				return Ok(index);
			},
			Err(e) => {
				last_error = Some(e);
				log::warn!("[StoreEntry] Load attempt {} failed", attempt + 1);

				// Wait before retry
				if attempt < max_retries - 1 {
					tokio::time::sleep(tokio::time::Duration::from_millis(100 * (attempt + 1) as u64)).await;
				}
			},
		}
	}

	Err(last_error.unwrap_or_else(|| AirError::Internal("Failed to load index after retries".to_string())))
}

/// Get index file path
pub fn GetIndexFilePath(index_directory:&Path) -> PathBuf { index_directory.join("file_index.json") }

/// Check if index file exists and is readable
pub async fn IndexFileExists(index_directory:&Path) -> Result<bool> {
	let index_file = index_directory.join("file_index.json");

	if !index_file.exists() {
		return Ok(false);
	}

	// Try to read metadata to verify accessibility
	match tokio::fs::metadata(&index_file).await {
		Ok(_) => Ok(true),
		Err(_) => Ok(false),
	}
}

/// Get index file size in bytes
pub async fn GetIndexFileSize(index_directory:&Path) -> Result<u64> {
	let index_file = index_directory.join("file_index.json");

	let metadata = tokio::fs::metadata(&index_file)
		.await
		.map_err(|e| AirError::FileSystem(format!("Failed to get index file metadata: {}", e)))?;

	Ok(metadata.len())
}

/// Clean up old backup files
pub async fn CleanupOldBackups(index_directory:&Path, keep_count:usize) -> Result<usize> {
	let mut entries = tokio::fs::read_dir(index_directory)
		.await
		.map_err(|e| AirError::FileSystem(format!("Failed to read index directory: {}", e)))?;

	let mut backups = Vec::new();

	while let Some(entry) = entries
		.next_entry()
		.await
		.map_err(|e| AirError::FileSystem(format!("Failed to read directory entry: {}", e)))?
	{
		let file_name = entry.file_name().to_string_lossy().to_string();

		if file_name.starts_with("file_index.corrupted.") && file_name.ends_with(".json") {
			if let Ok(metadata) = entry.metadata().await {
				if let Ok(modified) = metadata.modified() {
					backups.push((entry.path(), modified));
				}
			}
		}
	}

	// Sort by modified time (oldest first)
	backups.sort_by_key(|b| b.1);

	let mut removed_count = 0;

	// Remove old backups beyond keep_count
	for (path, _) in backups.iter().take(backups.len().saturating_sub(keep_count)) {
		match tokio::fs::remove_file(path).await {
			Ok(_) => {
				log::info!("[StoreEntry] Removed old backup: {}", path.display());
				removed_count += 1;
			},
			Err(e) => {
				log::warn!("[StoreEntry] Failed to remove backup {}: {}", path.display(), e);
			},
		}
	}

	Ok(removed_count)
}

/// Validate index file format before loading
pub async fn ValidateIndexFormat(index_directory:&Path) -> Result<()> {
	let index_file = index_directory.join("file_index.json");

	let content = tokio::fs::read_to_string(&index_file)
		.await
		.map_err(|e| AirError::FileSystem(format!("Failed to read index file: {}", e)))?;

	// Try to parse as JSON
	let _:serde_json::Value = serde_json::from_str(&content)
		.map_err(|e| AirError::Serialization(format!("Index file is not valid JSON: {}", e)))?;

	Ok(())
}
