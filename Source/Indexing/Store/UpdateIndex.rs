//! # UpdateIndex
//!
//! ## File: Indexing/Store/UpdateIndex.rs
//!
//! ## Role in Air Architecture
//!
//! Provides index update functionality for the File Indexer service,
//! handling incremental updates to the index from file watching and
//! manual trigger events.
//!
//! ## Primary Responsibility
//!
//! Update the file index in response to file system changes, maintaining
//! consistency between the in-memory index and the disk storage.
//!
//! ## Secondary Responsibilities
//!
//! - Incremental file updates
//! - File deletion handling
//! - Index content re-indexing
//! - Symbol index updates
//! - Automatic index persistence
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `tokio` - Async file I/O operations
//!
//! **Internal Modules:**
//! - `crate::Result` - Error handling type
//! - `crate::AirError` - Error types
//! - `super::super::FileIndex` - Index structure definitions
//! - `super::StoreEntry` - Index storage operations
//! - `super::ScanFile` - File scanning operations
//! - `super::UpdateState` - State update functions
//!
//! ## Dependents
//!
//! - `Indexing::Watch::WatchFile` - File watcher event handlers
//! - `Indexing::mod::FileIndexer` - Main file indexer implementation
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's incremental indexing in
//! `src/vs/workbench/services/search/common/`
//!
//! ## Security Considerations
//!
//! - Path validation before updates
//! - File size limits enforced
//! - Symbolic link handling
//!
//! ## Performance Considerations
//!
//! - Incremental updates minimize reindexing
//! - Debouncing rapid file changes
//! - Batch updates for multiple changes
//! - Efficient symbol index updates
//!
//! ## Error Handling Strategy
//!
//! Update operations log warnings for individual failures and continue,
//! ensuring a single file error doesn't halt the entire update process.
//!
//! ## Thread Safety
//!
//! Update operations acquire write locks on shared state and return
//! results for persistence.

use std::{path::PathBuf, sync::Arc, time::Duration};

use tokio::{
	sync::{RwLock, Semaphore},
	time::Instant,
};

use crate::{
	AirError,
	Configuration::IndexingConfig,
	Indexing::State::CreateState::{FileIndex, FileMetadata},
	Result,
	dev_log,
};

/// Update index for a single file
pub async fn UpdateSingleFile(
	index:&mut FileIndex,

	file_path:&PathBuf,

	config:&IndexingConfig,
) -> Result<Option<FileMetadata>> {
	let start_time = Instant::now();

	// Check if file still exists
	if !file_path.exists() {
		// File was deleted, remove from index
		crate::Indexing::State::UpdateState::RemoveFileFromIndex(index, file_path)?;

		dev_log!("indexing", "[UpdateIndex] Removed deleted file: {}", file_path.display());

		return Ok(None);
	}

	// Get current file metadata
	let current_metadata = std::fs::metadata(file_path)
		.map_err(|e| AirError::FileSystem(format!("Failed to get file metadata: {}", e)))?;

	let current_modified = current_metadata
		.modified()
		.map_err(|e| AirError::FileSystem(format!("Failed to get modification time: {}", e)))?;

	let _current_modified_time = chrono::DateTime::<chrono::Utc>::from(current_modified);

	// Check if we need to update this file
	let needs_update = match index.files.get(file_path) {
		Some(old_metadata) => {
			// Update if checksums don't match (content changed)
			let checksum = crate::Indexing::Scan::ScanFile::CalculateChecksum(
				&tokio::fs::read(file_path).await.unwrap_or_default(),
			);

			old_metadata.checksum != checksum
		},

		None => {
			// File not in index, needs to be added
			true
		},
	};

	if !needs_update {
		dev_log!("indexing", "file unchanged: {}", file_path.display());

		return Ok(index.files.get(file_path).cloned());
	}

	// Scan the file
	use crate::Indexing::{Scan::ScanFile::IndexFileInternal, State::UpdateState::UpdateIndexMetadata};

	let (metadata, symbols) = IndexFileInternal(file_path, config, &[]).await?;

	// Update the index
	crate::Indexing::State::UpdateState::RemoveFileFromIndex(index, file_path)?;

	crate::Indexing::State::UpdateState::AddFileToIndex(index, file_path.clone(), metadata.clone(), symbols)?;

	// Update index metadata
	UpdateIndexMetadata(index)?;

	let elapsed = start_time.elapsed();

	dev_log!(
		"indexing",
		"updated {} in {}ms ({} symbols)",
		file_path.display(),
		elapsed.as_millis(),
		metadata.symbol_count
	);

	Ok(Some(metadata))
}

/// Update index content for a file
pub async fn UpdateFileContent(index:&mut FileIndex, file_path:&PathBuf, metadata:&FileMetadata) -> Result<()> {
	// Only index text files
	if !metadata.mime_type.starts_with("text/") && !metadata.mime_type.contains("json") {
		return Ok(());
	}

	let content = tokio::fs::read_to_string(file_path)
		.await
		.map_err(|e| AirError::FileSystem(format!("Failed to read file content: {}", e)))?;

	// Remove file from existing content index entries
	for (_, files) in index.content_index.iter_mut() {
		files.retain(|p| p != file_path);
	}

	// Token-based indexing
	let tokens = crate::Indexing::Process::ProcessContent::TokenizeContent(&content);

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

/// Update multiple files in batch
pub async fn UpdateFilesBatch(
	index:&mut FileIndex,

	file_paths:Vec<PathBuf>,

	config:&IndexingConfig,
) -> Result<UpdateBatchResult> {
	let start_time = Instant::now();

	let mut updated_count = 0u32;

	let mut removed_count = 0u32;

	let mut error_count = 0u32;

	let mut total_size = 0u64;

	for file_path in file_paths {
		match UpdateSingleFile(index, &file_path, config).await {
			Ok(Some(metadata)) => {
				updated_count += 1;

				total_size += metadata.size;
			},

			Ok(None) => {
				removed_count += 1;
			},

			Err(e) => {
				dev_log!(
					"indexing",
					"warn: [UpdateIndex] Failed to update file {}: {}",
					file_path.display(),
					e
				);

				error_count += 1;
			},
		}
	}

	// Update index metadata
	crate::Indexing::State::UpdateState::UpdateIndexMetadata(index)?;

	Ok(UpdateBatchResult {
		updated_count,
		removed_count,
		error_count,
		total_size,
		duration_seconds:start_time.elapsed().as_secs_f64(),
	})
}

/// Batch update result
#[derive(Debug, Clone)]
pub struct UpdateBatchResult {
	pub updated_count:u32,

	pub removed_count:u32,

	pub error_count:u32,

	pub total_size:u64,

	pub duration_seconds:f64,
}

/// Debounced file update to prevent excessive re-indexing
pub struct DebouncedUpdate {
	file_path:PathBuf,

	last_seen:Instant,

	index:*const RwLock<FileIndex>,

	config:IndexingConfig,

	duration:Duration,

	pending:bool,
}

unsafe impl Send for DebouncedUpdate {}

impl DebouncedUpdate {
	pub fn new(file_path:PathBuf, index:&RwLock<FileIndex>, config:&IndexingConfig, duration:Duration) -> Self {
		Self {
			file_path,

			last_seen:Instant::now(),

			index:index as *const RwLock<FileIndex>,

			config:config.clone(),

			duration,

			pending:false,
		}
	}

	pub async fn trigger(&mut self) {
		self.last_seen = Instant::now();

		self.pending = true;
	}

	pub async fn process_if_ready(&mut self) -> Result<bool> {
		if !self.pending {
			return Ok(false);
		}

		if self.last_seen.elapsed() >= self.duration {
			self.pending = false;

			// This is unsafe but we're in a controlled context
			let index_ref = unsafe { &*self.index };

			let mut index = index_ref.write().await;

			match UpdateSingleFile(&mut index, &self.file_path, &self.config).await {
				Ok(_) => {
					dev_log!(
						"indexing",
						"[UpdateIndex] Debounced update completed: {}",
						self.file_path.display()
					);

					return Ok(true);
				},

				Err(e) => {
					dev_log!("indexing", "warn: [UpdateIndex] Debounced update failed: {}", e);

					return Err(e);
				},
			}
		}

		Ok(false)
	}

	pub fn clear_pending(&mut self) { self.pending = false; }
}

/// Update index for changed files from file watcher
pub async fn ProcessWatcherEvent(
	index:&mut FileIndex,

	event:notify::Event,

	config:&IndexingConfig,
) -> Result<WatcherEventResult> {
	let mut updated = 0u32;

	let mut removed = 0u32;

	for file_path in event.paths {
		match event.kind {
			notify::EventKind::Create(notify::event::CreateKind::File) => {
				dev_log!("indexing", "[UpdateIndex] File created: {}", file_path.display());

				if UpdateSingleFile(index, &file_path, config).await.is_ok() {
					updated += 1;
				}
			},

			notify::EventKind::Modify(notify::event::ModifyKind::Data(_))
			| notify::EventKind::Modify(notify::event::ModifyKind::Name(notify::event::RenameMode::Both)) => {
				dev_log!("indexing", "[UpdateIndex] File modified: {}", file_path.display());

				if UpdateSingleFile(index, &file_path, config).await.is_ok() {
					updated += 1;
				}
			},

			notify::EventKind::Remove(notify::event::RemoveKind::File) => {
				dev_log!("indexing", "[UpdateIndex] File removed: {}", file_path.display());

				if super::super::State::UpdateState::RemoveFileFromIndex(index, &file_path).is_ok() {
					removed += 1;
				}
			},

			_ => {},
		}
	}

	// Update index metadata
	super::super::State::UpdateState::UpdateIndexMetadata(index)?;

	Ok(WatcherEventResult { updated, removed })
}

/// Watcher event processing result
#[derive(Debug, Clone)]
pub struct WatcherEventResult {
	pub updated:u32,

	pub removed:u32,
}

/// Remove files from index that no longer exist
pub async fn CleanupRemovedFiles(index:&mut FileIndex) -> Result<u32> {
	let mut paths_to_remove = Vec::new();

	let all_paths:Vec<_> = index.files.keys().cloned().collect();

	for path in all_paths {
		if !path.exists() {
			paths_to_remove.push(path);
		}
	}

	for path in &paths_to_remove {
		super::super::State::UpdateState::RemoveFileFromIndex(index, path)?;
	}

	// Update index metadata
	super::super::State::UpdateState::UpdateIndexMetadata(index)?;

	dev_log!("indexing", "[UpdateIndex] Cleaned up {} removed files", paths_to_remove.len());

	Ok(paths_to_remove.len() as u32)
}

/// Rebuild index from scratch (full reindex)
pub async fn RebuildIndex(
	index:&mut FileIndex,

	directories:Vec<String>,

	patterns:Vec<String>,

	config:&IndexingConfig,
) -> Result<UpdateBatchResult> {
	let start_time = Instant::now();

	// Clear current index
	index.files.clear();

	index.content_index.clear();

	index.symbol_index.clear();

	index.file_symbols.clear();

	// Scan directories
	let (files_to_index, scan_result) =
		crate::Indexing::Scan::ScanDirectory::ScanDirectoriesParallel(directories, patterns, config, 10).await?;

	// Index all files
	let semaphore = Arc::new(Semaphore::new(config.MaxParallelIndexing as usize));

	let index_arc = Arc::new(RwLock::new(index.clone()));

	let mut tasks = Vec::new();

	for file_path in files_to_index {
		let permit = semaphore.clone().acquire_owned().await.unwrap();

		// Variables cloned for use in async task
		let _index_ref = index_arc.clone();

		let config_clone = config.clone();

		let task = tokio::spawn(async move {
			let _permit = permit;

			crate::Indexing::Scan::ScanFile::IndexFileInternal(&file_path, &config_clone, &[]).await
		});

		tasks.push(task);
	}

	let mut updated_count = 0u32;

	let mut total_size = 0u64;

	for task in tasks {
		match task.await {
			Ok(Ok((metadata, symbols))) => {
				let file_size = metadata.size;

				super::super::State::UpdateState::AddFileToIndex(index, metadata.path.clone(), metadata, symbols)?;

				updated_count += 1;

				total_size += file_size;
			},

			Ok(Err(e)) => {
				dev_log!("indexing", "warn: [UpdateIndex] Rebuild task failed: {}", e);
			},

			Err(e) => {
				dev_log!("indexing", "warn: [UpdateIndex] Rebuild task join failed: {}", e);
			},
		}
	}

	// Update index metadata
	super::super::State::UpdateState::UpdateIndexMetadata(index)?;

	Ok(UpdateBatchResult {
		updated_count,
		removed_count:0,
		error_count:scan_result.errors,
		total_size,
		duration_seconds:start_time.elapsed().as_secs_f64(),
	})
}

/// Validate index consistency and repair if needed
pub async fn ValidateAndRepairIndex(index:&mut FileIndex) -> Result<RepairResult> {
	let start_time = Instant::now();

	let mut repaired_files = 0u32;

	let removed_orphans;

	// Validate index consistency
	match super::super::State::UpdateState::ValidateIndexConsistency(index) {
		Ok(()) => {},

		Err(e) => {
			dev_log!("indexing", "warn: [UpdateIndex] Index validation failed: {}", e);

			repaired_files += 1;
		},
	}

	// Clean up orphaned entries
	removed_orphans = super::super::State::UpdateState::CleanupOrphanedEntries(index)?;

	// Update index metadata
	super::super::State::UpdateState::UpdateIndexMetadata(index)?;

	Ok(RepairResult {
		repaired_files,
		removed_orphans,
		duration_seconds:start_time.elapsed().as_secs_f64(),
	})
}

/// Index repair result
#[derive(Debug, Clone)]
pub struct RepairResult {
	pub repaired_files:u32,

	pub removed_orphans:u32,

	pub duration_seconds:f64,
}
