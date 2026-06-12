//! # WatchFile
//!
//! ## File: Indexing/Watch/WatchFile.rs
//!
//! ## Role in Air Architecture
//!
//! Provides file watching functionality for the File Indexer service,
//! handling file system events for incremental index updates.
//!
//! ## Primary Responsibility
//!
//! Handle file system change events and trigger index updates for
//! created, modified, and deleted files.
//!
//! ## Secondary Responsibilities
//!
//! - File creation event handling
//! - File modification event handling
//! - File deletion event handling
//! - Directory change event handling
//! - Event debouncing for rapid changes
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `notify` - File system watching
//! - `tokio` - Async runtime for event handling
//!
//! **Internal Modules:**
//! - `crate::Result` - Error handling type
//! - `crate::AirError` - Error types
//! - `super::super::FileIndex` - Index structure definitions
//! - `super::super::Store::UpdateIndex` - Index update operations
//!
//! ## Dependents
//!
//! - `Indexing::Background::StartWatcher` - Watcher setup and management
//! - `Indexing::mod::FileIndexer` - Main file indexer implementation
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's file watching in
//! `src/vs/base/node/watcher/`
//!
//! ## Security Considerations
//!
//! - Path validation before watching
//! - Symbolic link following disabled
//! - Permission checking on watch paths
//!
//! ## Performance Considerations
//!
//! - Event debouncing prevents excessive updates
//! - Batch processing of multiple events
//! - Efficient event filtering
//!
//! ## Error Handling Strategy
//!
//! Event operations log warnings for individual errors and continue,
//! ensuring a single event failure doesn't stop the watcher.
//!
//! ## Thread Safety
//!
//! Event handlers acquire write locks on shared state and process
//! events asynchronously to avoid blocking the watcher loop.

use std::path::PathBuf;

use tokio::sync::{Mutex, RwLock};

use crate::{
	AirError,
	Configuration::AirConfiguration::IndexingConfig,
	Indexing::State::CreateState::FileIndex,
	Result,
	dev_log,
};

/// Handle file watcher event for incremental indexing
///
/// Processes file system events and updates the index accordingly.
/// accordingly:
/// - File Created: Index the new file
/// - File Modified: Re-index the modified file
/// - File Removed: Remove from index
pub async fn HandleFileEvent(event:notify::Event, index_arc:&RwLock<FileIndex>, config:&IndexingConfig) -> Result<()> {
	match event.kind {
		notify::EventKind::Create(notify::event::CreateKind::File) => {
			for path in event.paths {
				dev_log!("indexing", "[WatchFile] File created: {}", path.display());

				let mut index = index_arc.write().await;

				if let Err(e) = crate::Indexing::Store::UpdateIndex::UpdateSingleFile(&mut index, &path, config).await {
					dev_log!(
						"indexing",
						"warn: [WatchFile] Failed to index new file {}: {}",
						path.display(),
						e
					);
				}
			}
		},

		notify::EventKind::Modify(notify::event::ModifyKind::Data(_))
		| notify::EventKind::Modify(notify::event::ModifyKind::Name(notify::event::RenameMode::Both)) => {
			for path in event.paths {
				dev_log!("indexing", "[WatchFile] File modified: {}", path.display());

				let mut index = index_arc.write().await;

				if let Err(e) = crate::Indexing::Store::UpdateIndex::UpdateSingleFile(&mut index, &path, config).await {
					dev_log!(
						"indexing",
						"warn: [WatchFile] Failed to re-index modified file {}: {}",
						path.display(),
						e
					);
				}
			}
		},

		notify::EventKind::Remove(notify::event::RemoveKind::File) => {
			for path in event.paths {
				dev_log!("indexing", "[WatchFile] File removed: {}", path.display());

				let mut index = index_arc.write().await;

				if let Err(e) = crate::Indexing::State::UpdateState::RemoveFileFromIndex(&mut index, &path) {
					dev_log!(
						"indexing",
						"warn: [WatchFile] Failed to remove file from index {}: {}",
						path.display(),
						e
					);
				}
			}
		},

		notify::EventKind::Create(notify::event::CreateKind::Folder) => {
			for path in event.paths {
				dev_log!("indexing", "[WatchFile] Directory created: {}", path.display()); // Directories themselves don't need indexing, just their

				// contents
			}
		},

		notify::EventKind::Remove(notify::event::RemoveKind::Folder) => {
			for path in event.paths {
				dev_log!("indexing", "[WatchFile] Directory removed: {}", path.display()); // Remove all files from this directory

				let mut index = index_arc.write().await;

				let mut paths_to_remove = Vec::new();

				for indexed_path in index.files.keys() {
					if indexed_path.starts_with(&path) {
						paths_to_remove.push(indexed_path.clone());
					}
				}

				for indexed_path in paths_to_remove {
					if let Err(e) = crate::Indexing::State::UpdateState::RemoveFileFromIndex(&mut index, &indexed_path)
					{
						dev_log!(
							"indexing",
							"warn: [WatchFile] Failed to remove file {}: {}",
							indexed_path.display(),
							e
						);
					}
				}
			}
		},

		_ => {
			// Ignore other event types
			dev_log!("indexing", "ignored event kind: {:?}", event.kind);
		},
	}

	Ok(())
}

/// Debounced file change handler
///
/// Prevents rapid successive changes from causing excessive re-indexing
pub struct DebouncedEventHandler {
	pending_changes:Mutex<std::collections::HashMap<PathBuf, FileChangeInfo>>,
}

impl DebouncedEventHandler {
	pub fn new() -> Self { Self { pending_changes:Mutex::new(std::collections::HashMap::new()) } }

	/// Add a file change event
	pub async fn AddChange(&self, path:PathBuf, change_type:FileChangeType) {
		let mut pending = self.pending_changes.lock().await;

		let now = std::time::Instant::now();

		match pending.get_mut(&path) {
			Some(change_info) => {
				change_info.last_seen = now;

				change_info.change_type = change_type.max(change_info.change_type);

				change_info.suppressed_count += 1;
			},

			None => {
				pending.insert(
					path.clone(),
					FileChangeInfo { path:path.clone(), change_type, last_seen:now, suppressed_count:0 },
				);
			},
		}
	}

	/// Process pending changes older than the specified duration
	pub async fn ProcessPendingChanges(
		&self,

		age_cutoff:std::time::Duration,

		index_arc:&RwLock<FileIndex>,

		config:&IndexingConfig,
	) -> Result<Vec<ProcessedChange>> {
		let mut processed = Vec::new();

		let expired_paths = {
			let mut pending = self.pending_changes.lock().await;

			let mut expired = Vec::new();

			for (path, change_info) in pending.iter() {
				if change_info.last_seen.elapsed() >= age_cutoff {
					expired.push((path.clone(), change_info.clone()));
				}
			}

			// Remove expired entries
			for (path, _) in &expired {
				pending.remove(path);
			}

			expired
		};

		for (path, change_info) in expired_paths {
			dev_log!(
				"indexing",
				"[WatchFile] Processing debounced change for {} (suppressed: {})",
				path.display(),
				change_info.suppressed_count
			);

			let result = match change_info.change_type {
				FileChangeType::Created => {
					let mut index = index_arc.write().await;

					crate::Indexing::Store::UpdateIndex::UpdateSingleFile(&mut index, &path, config)
						.await
						.map(|_| ProcessedChangeResult::Success)
						.unwrap_or(ProcessedChangeResult::Failed)
				},

				FileChangeType::Modified => {
					let mut index = index_arc.write().await;

					super::super::Store::UpdateIndex::UpdateSingleFile(&mut index, &path, config)
						.await
						.map(|_| ProcessedChangeResult::Success)
						.unwrap_or(ProcessedChangeResult::Failed)
				},

				FileChangeType::Removed => {
					let mut index = index_arc.write().await;

					crate::Indexing::State::UpdateState::RemoveFileFromIndex(&mut index, &path)
						.map(|_| ProcessedChangeResult::Success)
						.unwrap_or(ProcessedChangeResult::Failed)
				},
			};

			processed.push(ProcessedChange {
				path,
				change_type:change_info.change_type,
				suppressed_count:change_info.suppressed_count,
				result,
			});
		}

		Ok(processed)
	}

	/// Clear all pending changes
	pub async fn ClearPending(&self) -> usize {
		let mut pending = self.pending_changes.lock().await;

		let count = pending.len();

		pending.clear();

		count
	}

	/// Get the number of pending changes
	pub async fn PendingCount(&self) -> usize {
		let pending = self.pending_changes.lock().await;

		pending.len()
	}
}

impl Default for DebouncedEventHandler {
	fn default() -> Self { Self::new() }
}

/// File change type for debouncing
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileChangeType {
	Created,

	Modified,

	Removed,
}

impl FileChangeType {
	pub fn max(self, other:Self) -> Self {
		// Removed takes precedence over Modified, which takes precedence over Created
		match (self, other) {
			(Self::Removed, _) | (_, Self::Removed) => Self::Removed,

			(Self::Modified, _) | (_, Self::Modified) => Self::Modified,

			(Self::Created, Self::Created) => Self::Created,
		}
	}
}

/// File change information for debouncing
#[derive(Debug, Clone)]
struct FileChangeInfo {
	path:PathBuf,

	change_type:FileChangeType,

	last_seen:std::time::Instant,

	suppressed_count:usize,
}

/// Result of processing a debounced change
#[derive(Debug, Clone)]
pub enum ProcessedChangeResult {
	Success,

	Failed,
}

/// Describes a processed file change
#[derive(Debug, Clone)]
pub struct ProcessedChange {
	pub path:PathBuf,

	pub change_type:FileChangeType,

	pub suppressed_count:usize,

	pub result:ProcessedChangeResult,
}

/// Convert notify event kind to FileChangeType
pub fn EventKindToChangeType(kind:notify::EventKind) -> Option<FileChangeType> {
	match kind {
		notify::EventKind::Create(_) => Some(FileChangeType::Created),

		notify::EventKind::Modify(_) => Some(FileChangeType::Modified),

		notify::EventKind::Remove(_) => Some(FileChangeType::Removed),

		_ => None,
	}
}

/// Check if a path should be watched (not in ignored paths)
pub fn ShouldWatchPath(path:&PathBuf, ignored_patterns:&[String]) -> bool {
	let path_str = path.to_string_lossy();

	// Check against ignore patterns
	for pattern in ignored_patterns {
		if path_str.contains(pattern) {
			return false;
		}
	}

	true
}

/// Get default ignored patterns for file watching
pub fn GetDefaultIgnoredPatterns() -> Vec<String> {
	vec![
		"node_modules".to_string(),
		"target".to_string(),
		".git".to_string(),
		".svn".to_string(),
		".hg".to_string(),
		".bzr".to_string(),
		"dist".to_string(),
		"build".to_string(),
		".next".to_string(),
		".nuxt".to_string(),
		"__pycache__".to_string(),
		"*.pyc".to_string(),
		".venv".to_string(),
		"venv".to_string(),
		"env".to_string(),
		".env".to_string(),
		".idea".to_string(),
		".vscode".to_string(),
		".DS_Store".to_string(),
		"Thumbs.db".to_string(),
		"*.swp".to_string(),
		"*.tmp".to_string(),
	]
}

/// Validate that a watch path exists and is accessible
pub fn ValidateWatchPath(path:&PathBuf) -> Result<()> {
	if !path.exists() {
		return Err(AirError::FileSystem(format!("Watch path does not exist: {}", path.display())));
	}

	if !path.is_dir() {
		return Err(AirError::FileSystem(format!(
			"Watch path is not a directory: {}",
			path.display()
		)));
	}

	// Check read access
	std::fs::read_dir(path)
		.map_err(|e| AirError::FileSystem(format!("Cannot access watch path {}: {}", path.display(), e)))?;

	Ok(())
}
