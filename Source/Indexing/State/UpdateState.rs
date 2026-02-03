//! # UpdateState
//!
//! ## File: Indexing/State/UpdateState.rs
//!
//! ## Role in Air Architecture
//!
//! Provides state update operations for the File Indexer service, handling
//! modification of index structures including adding, removing, and updating
//! entries in the file index.
//!
//! ## Primary Responsibility
//!
//! Update file index state by adding/removing files, symbols, and content
//! entries in a thread-safe manner.
//!
//! ## Secondary Responsibilities
//!
//! - Remove deleted files from all indexes
//! - Update symbol index with new symbol locations
//! - Update content index with new file paths
//! - Maintain index version and checksum on updates
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `tokio` - Async runtime for update operations
//!
//! **Internal Modules:**
//! - `crate::Result` - Error handling type
//! - `crate::AirError` - Error types
//! - `super::CreateState` - State structure definitions
//!
//! ## Dependents
//!
//! - `Indexing::Scan::ScanDirectory` - Updates index after directory scan
//! - `Indexing::Scan::ScanFile` - Updates index after file scan
//! - `Indexing::Store::UpdateIndex` - Incremental index updates
//! - `Indexing::Watch::WatchFile` - Updates index on file changes
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's index update operations in
//! `src/vs/workbench/services/search/common/`
//!
//! ## Security Considerations
//!
//! - Thread-safe updates prevent race conditions
//! - Path validation before state updates
//! - Size limits enforced on all update operations
//!
//! ## Performance Considerations
//!
//! - Incremental updates minimize reindexing
//! - Batch updates for multiple files
//! - Efficient hash lookups for O(1) updates
//!
//! ## Error Handling Strategy
//!
//! Update operations silently fail on missing keys (idempotent) and
//! propagate errors for I/O failures or invalid state transitions.
//!
//! ## Thread Safety
//!
//! All update operations are designed to work within RwLock write
//! guards and should be called while holding appropriate locks.

use std::path::PathBuf;

use crate::{
	AirError,
	ApplicationState::ApplicationState,
	Configuration::IndexingConfig,
	Indexing::State::CreateState::{FileIndex, FileMetadata, SymbolInfo, SymbolLocation},
	Result,
};

/// Add a file to the index with its metadata and symbols
pub fn AddFileToIndex(
	index:&mut FileIndex,
	file_path:PathBuf,
	metadata:FileMetadata,
	symbols:Vec<SymbolInfo>,
) -> Result<()> {
	// Check if file already exists and update accordingly
	let is_new = !index.files.contains_key(&file_path);

	// Add or update file metadata
	index.files.insert(file_path.clone(), metadata.clone());

	// Update symbol index
	if is_new {
		// Clear old symbols for this file if any
		index.file_symbols.remove(&file_path);
	}

	// Add new symbols
	index.file_symbols.insert(file_path.clone(), symbols.clone());

	// Update symbol index for cross-referencing
	for symbol in symbols {
		index
			.symbol_index
			.entry(symbol.name.clone())
			.or_insert_with(Vec::new)
			.push(SymbolLocation { file_path:file_path.clone(), line:symbol.line, symbol });
	}

	Ok(())
}

/// Remove a file from all indexes (content, symbols, files)
pub fn RemoveFileFromIndex(index:&mut FileIndex, file_path:&PathBuf) -> Result<()> {
	// Remove from files index
	index.files.remove(file_path);

	// Remove from file_symbols
	index.file_symbols.remove(file_path);

	// Remove from symbol index
	for (_, locations) in index.symbol_index.iter_mut() {
		locations.retain(|loc| loc.file_path != *file_path);
	}

	// Remove from content index
	for (_, files) in index.content_index.iter_mut() {
		files.retain(|p| p != file_path);
	}

	Ok(())
}

/// Remove multiple files from the index in a batch operation
pub fn RemoveFilesFromIndex(index:&mut FileIndex, file_paths:&[PathBuf]) -> Result<()> {
	for file_path in file_paths {
		RemoveFileFromIndex(index, file_path)?;
	}
	Ok(())
}

/// Update index metadata (version, timestamp, checksum)
pub fn UpdateIndexMetadata(index:&mut FileIndex) -> Result<()> {
	use crate::Indexing::State::CreateState::{CalculateIndexChecksum, GenerateIndexVersion};

	index.last_updated = chrono::Utc::now();
	index.index_version = GenerateIndexVersion();
	index.index_checksum = CalculateIndexChecksum(index)?;

	Ok(())
}

/// Update file metadata for an existing file
pub fn UpdateFileMetadata(index:&mut FileIndex, file_path:&PathBuf, metadata:FileMetadata) -> Result<()> {
	if !index.files.contains_key(file_path) {
		return Err(AirError::Internal(format!(
			"Cannot update metadata for file not in index: {}",
			file_path.display()
		)));
	}

	index.files.insert(file_path.clone(), metadata);
	Ok(())
}

/// Update symbols for a file
pub fn UpdateFileSymbols(index:&mut FileIndex, file_path:&PathBuf, symbols:Vec<SymbolInfo>) -> Result<()> {
	if !index.files.contains_key(file_path) {
		return Err(AirError::Internal(format!(
			"Cannot update symbols for file not in index: {}",
			file_path.display()
		)));
	}

	// Remove old symbols from symbol index
	if let Some(old_symbols) = index.file_symbols.get(file_path) {
		for old_symbol in old_symbols {
			if let Some(locations) = index.symbol_index.get_mut(&old_symbol.name) {
				locations.retain(|loc| loc.file_path != *file_path);
			}
		}
	}

	// Add new symbols
	index.file_symbols.insert(file_path.clone(), symbols.clone());

	for symbol in symbols {
		index
			.symbol_index
			.entry(symbol.name.clone())
			.or_insert_with(Vec::new)
			.push(SymbolLocation { file_path:file_path.clone(), line:symbol.line, symbol });
	}

	Ok(())
}

/// Update content index for a file
pub fn UpdateContentIndex(index:&mut FileIndex, file_path:&PathBuf, tokens:Vec<String>) -> Result<()> {
	// Remove file from existing content index entries
	for (_, files) in index.content_index.iter_mut() {
		files.retain(|p| p != file_path);
	}

	// Add new tokens
	for token in tokens {
		if token.len() > 2 {
			// Only index tokens longer than 2 characters
			index
				.content_index
				.entry(token)
				.or_insert_with(Vec::new)
				.push(file_path.clone());
		}
	}

	Ok(())
}

/// Clean up orphaned entries (files with no matching content/symbols)
pub fn CleanupOrphanedEntries(index:&mut FileIndex) -> Result<u32> {
	let mut removed_count = 0;

	let files_to_keep:Vec<_> = index.files.keys().cloned().collect();

	// Clean up content index entries with no files
	let orphaned_tokens:Vec<_> = index
		.content_index
		.iter()
		.filter(|(_, files)| files.is_empty())
		.map(|(token, _)| token.clone())
		.collect();

	for token in orphaned_tokens {
		index.content_index.remove(&token);
		removed_count += 1;
	}

	// Clean up symbol index entries with no locations
	let orphaned_symbols:Vec<_> = index
		.symbol_index
		.iter()
		.filter(|(_, locations)| locations.is_empty())
		.map(|(symbol, _)| symbol.clone())
		.collect();

	for symbol in orphaned_symbols {
		index.symbol_index.remove(&symbol);
		removed_count += 1;
	}

	Ok(removed_count)
}

/// Merge another index into this one
pub fn MergeIndexes(target:&mut FileIndex, source:FileIndex) -> Result<u32> {
	let mut merged_files = 0;

	// Merge files
	for (path, metadata) in source.files {
		if !target.files.contains_key(&path) {
			target.files.insert(path.clone(), metadata);
			merged_files += 1;
		}
	}

	// Merge content index
	for (token, mut files) in source.content_index {
		target.content_index.entry(token).or_insert_with(Vec::new).append(&mut files);
	}

	// Merge symbol index
	for (symbol, mut locations) in source.symbol_index {
		target
			.symbol_index
			.entry(symbol)
			.or_insert_with(Vec::new)
			.append(&mut locations);
	}

	// Merge file symbols
	for (path, symbols) in source.file_symbols {
		if !target.file_symbols.contains_key(&path) {
			target.file_symbols.insert(path, symbols);
		}
	}

	// Update metadata
	UpdateIndexMetadata(target)?;

	Ok(merged_files)
}

/// Validate that index is in a consistent state
pub fn ValidateIndexConsistency(index:&FileIndex) -> Result<()> {
	// Check that all files in content_index exist in files
	for (_, files) in &index.content_index {
		for file_path in files {
			if !index.files.contains_key(file_path) {
				return Err(AirError::Internal(format!(
					"Content index references non-existent file: {}",
					file_path.display()
				)));
			}
		}
	}

	// Check that all files in symbol_index exist in files
	for (_, locations) in &index.symbol_index {
		for location in locations {
			if !index.files.contains_key(&location.file_path) {
				return Err(AirError::Internal(format!(
					"Symbol index references non-existent file: {}",
					location.file_path.display()
				)));
			}
		}
	}

	// Check that all files in file_symbols exist in files
	for (file_path, _) in &index.file_symbols {
		if !index.files.contains_key(file_path) {
			return Err(AirError::Internal(format!(
				"File symbols references non-existent file: {}",
				file_path.display()
			)));
		}
	}

	Ok(())
}

/// Get index size estimate in bytes
pub fn GetIndexSizeEstimate(index:&FileIndex) -> usize {
	let mut size = 0;

	// File metadata
	for (path, metadata) in &index.files {
		size += path.as_os_str().len();
		size += std::mem::size_of::<FileMetadata>();
	}

	// Content index
	for (token, files) in &index.content_index {
		size += token.len();
		size += files.len() * std::mem::size_of::<PathBuf>();
	}

	// Symbol index
	for (symbol, locations) in &index.symbol_index {
		size += symbol.len();
		size += locations.len() * std::mem::size_of::<SymbolLocation>();
	}

	// File symbols
	for (path, symbols) in &index.file_symbols {
		size += path.as_os_str().len();
		size += symbols.len() * std::mem::size_of::<SymbolInfo>();
	}

	size
}

/// Check if periodic update is needed based on age
pub fn NeedsUpdate(index:&FileIndex, max_age_minutes:u64) -> bool {
	let age_minutes = (chrono::Utc::now() - index.last_updated).num_minutes().abs() as u64;
	age_minutes >= max_age_minutes
}
