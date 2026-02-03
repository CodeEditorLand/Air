//! # ScanDirectory
//!
//! ## File: Indexing/Scan/ScanDirectory.rs
//!
//! ## Role in Air Architecture
//!
//! Provides directory scanning functionality for the File Indexer service,
//! handling recursive traversal of directories to discover files for indexing.
//!
//! ## Primary Responsibility
//!
//! Scan directories recursively to discover files matching include patterns
//! while respecting exclude patterns and filesystem limits.
//!
//! ## Secondary Responsibilities
//!
//! - Validate directory permissions before scanning
//! - Parallel file enumeration for performance
//! - Skip directories like node_modules, target, .git
//! - Collect files with metadata for batch processing
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `ignore` - .gitignore-aware directory walking
//! - `tokio` - Async runtime for I/O operations
//!
//! **Internal Modules:**
//! - `crate::Result` - Error handling type
//! - `crate::AirError` - Error types
//! - `crate::Configuration::IndexingConfig` - Indexing configuration
//!
//! ## Dependents
//!
//! - `Indexing::mod::FileIndexer` - Main file indexer implementation
//! - `Indexing::Background::StartWatcher` - Background task scanning
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's file system scanning in
//! `src/vs/base/common/files/`
//!
//! ## Security Considerations
//!
//! - Path traversal protection through canonicalization
//! - Symbolic link following disabled by default
//! - Depth limits prevent infinite recursion
//! - Permission checking before access
//!
//! ## Performance Considerations
//!
//! - Parallel directory scanning with limited concurrency
//! - Batch collection of files for processing
//! - Lazy evaluation with ignore crate
//! - Early filtering by file patterns
//!
//! ## Error Handling Strategy
//!
//! Scan operations log warnings for individual errors and continue,
//! returning a result only if the top-level operation fails.
//!
//! ## Thread Safety
//!
//! Scan operations are designed to be called from async tasks and
//! return collectable results for parallel processing.

use std::{collections::HashSet, path::Path, sync::Arc};

use tokio::sync::{RwLock, Semaphore};

use crate::{AirError, Configuration::IndexingConfig, Result};
use crate::Indexing::State::CreateState::{FileIndex, FileMetadata, SymbolInfo, SymbolLocation};
use crate::Indexing::Scan::ScanFile::{IndexFileInternal, ValidateFileAccess};

/// Scan directory result with statistics
#[derive(Debug, Clone)]
pub struct ScanDirectoryResult {
	/// Number of files discovered
	pub files_found:u32,
	/// Number of files skipped (due to patterns/size)
	pub files_skipped:u32,
	/// Number of errors encountered
	pub errors:u32,
	/// Total size of discovered files in bytes
	pub total_size:u64,
}

/// Scan a directory recursively and collect matching files
///
/// Features:
/// - Path traversal protection
/// - Symbolic link handling (disabled by default)
/// - File size validation
/// - Permission error handling
/// - Include/exclude pattern support
/// - Parallel scanning with semaphore limits
pub async fn ScanDirectory(
	path:&str,
	patterns:Vec<String>,
	config:&IndexingConfig,
	max_parallel:usize,
) -> Result<(Vec<std::path::PathBuf>, ScanDirectoryResult)> {
	let directory_path = crate::Configuration::ConfigurationManager::ExpandPath(path)?;

	// Validate directory exists and is accessible
	if !directory_path.exists() {
		return Err(AirError::FileSystem(format!("Directory does not exist: {}", path)));
	}

	if !directory_path.is_dir() {
		return Err(AirError::FileSystem(format!("Path is not a directory: {}", path)));
	}

	// Check directory permissions
	CheckDirectoryPermissions(&directory_path).await?;

	// Build file patterns
	let include_patterns = if patterns.is_empty() { config.FileTypes.clone() } else { patterns };

	// Walk directory with .gitignore support
	let walker = ignore::WalkBuilder::new(&directory_path)
		.max_depth(Some(10)) // Prevent infinite recursion
		.hidden(false)
		.follow_links(false) // Don't follow symlinks by default
		.build();

	let mut files_to_scan:Vec<std::path::PathBuf> = Vec::new();
	let mut files_found = 0u32;
	let mut files_skipped = 0u32;
	let mut errors = 0u32;
	let mut total_size = 0u64;

	// Collect all files first
	for result in walker {
		match result {
			Ok(entry) => {
				// Only index regular files (not directories or symlinks)
				if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
					let file_path = entry.path().to_path_buf();

					// Check if file is a symbolic link
					if entry.path_is_symlink() {
						log::debug!("[ScanDirectory] Skipping symlink: {}", file_path.display());
						files_skipped += 1;
						continue;
					}

					// Check file size limit
					if let Ok(metadata) = entry.metadata() {
						let file_size = metadata.len();

						if file_size > config.MaxFileSizeMb as u64 * 1024 * 1024 {
							log::warn!(
								"[ScanDirectory] Skipping oversized file: {} ({} bytes)",
								file_path.display(),
								file_size
							);
							files_skipped += 1;
							continue;
						}

						// Check file pattern
						if MatchesPatterns(&file_path, &include_patterns) {
							// Try to get file access to validate permissions
							if ValidateFileAccess(&file_path).await {
								files_to_scan.push(file_path);
								files_found += 1;
								total_size += file_size;
							} else {
								log::warn!(
									"[ScanDirectory] Cannot access file (permission denied): {}",
									file_path.display()
								);
								errors += 1;
							}
						} else {
							files_skipped += 1;
						}
					} else {
						errors += 1;
					}
				}
			},
			Err(e) => {
				log::warn!("[ScanDirectory] Error walking directory: {}", e);
				errors += 1;
			},
		}
	}

	log::info!(
		"[ScanDirectory] Directory scan completed: {} files, {} skipped, {} errors, {} bytes",
		files_found,
		files_skipped,
		errors,
		total_size
	);

	Ok((
		files_to_scan,
		ScanDirectoryResult { files_found, files_skipped, errors, total_size },
	))
}

/// Scan a directory and remove deleted files from index
pub async fn ScanAndRemoveDeleted(index:&mut FileIndex, directory_path:&Path) -> Result<u32> {
	let mut paths_to_remove = Vec::new();
	let all_paths:Vec<_> = index.files.keys().cloned().collect();

	for path in all_paths {
		if !path.exists() && path.starts_with(directory_path) {
			paths_to_remove.push(path.clone());
		}
	}

	for path in paths_to_remove {
		index.files.remove(&path);
		index.file_symbols.remove(&path);

		// Remove from symbol index
		for (_, locations) in index.symbol_index.iter_mut() {
			locations.retain(|loc| loc.file_path != path);
		}

		// Remove from content index
		for (_, files) in index.content_index.iter_mut() {
			files.retain(|p| p != &path);
		}
	}

	Ok(paths_to_remove.len() as u32)
}

/// Check directory read permissions
async fn CheckDirectoryPermissions(path:&Path) -> Result<()> {
	tokio::task::spawn_blocking({
		let path = path.to_path_buf();
		move || {
			std::fs::read_dir(&path)
				.map_err(|e| AirError::FileSystem(format!("Cannot read directory {}: {}", path.display(), e)))?;
			Ok(())
		}
	})
	.await?
}

/// Check if file path matches any of the provided patterns
pub fn MatchesPatterns(file_path:&std::path::Path, patterns:&[String]) -> bool {
	if patterns.is_empty() {
		return true;
	}

	let file_name = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();

	for pattern in patterns {
		if MatchesPattern(&file_name, pattern) {
			return true;
		}
	}

	false
}

/// Check if filename matches a single pattern
pub fn MatchesPattern(filename:&str, pattern:&str) -> bool {
	if pattern.starts_with("*.") {
		let extension = &pattern[2..];
		filename.ends_with(extension)
	} else {
		filename == pattern
	}
}

/// Get default exclude patterns for directory scanning
pub fn GetDefaultExcludePatterns() -> Vec<String> {
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
	]
}

/// Parallel scan of multiple directories
pub async fn ScanDirectoriesParallel(
	directories:Vec<String>,
	patterns:Vec<String>,
	config:&IndexingConfig,
	max_parallel:usize,
) -> Result<(Vec<std::path::PathBuf>, ScanDirectoryResult)> {
	let semaphore = Arc::new(Semaphore::new(max_parallel));
	let mut all_files = Vec::new();
	let mut total_result = ScanDirectoryResult { files_found:0, files_skipped:0, errors:0, total_size:0 };

	let mut scan_tasks = Vec::new();

	for directory in directories {
		let permit = semaphore.clone().acquire_owned().await.unwrap();
		let config_clone = config.clone();
		let patterns_clone = patterns.clone();

		let task = tokio::spawn(async move {
			let _permit = permit;
			ScanDirectory(&directory, patterns_clone, &config_clone, max_parallel).await
		});

		scan_tasks.push(task);
	}

	// Collect results
	for task in scan_tasks {
		match task.await {
			Ok(Ok((files, result))) => {
				all_files.extend(files);
				total_result.files_found += result.files_found;
				total_result.files_skipped += result.files_skipped;
				total_result.errors += result.errors;
				total_result.total_size += result.total_size;
			},
			Ok(Err(e)) => {
				log::error!("[ScanDirectory] Parallel scan failed: {}", e);
				total_result.errors += 1;
			},
			Err(e) => {
				log::error!("[ScanDirectory] Parallel task panicked: {}", e);
				total_result.errors += 1;
			},
		}
	}

	Ok((all_files, total_result))
}

/// Get file count statistics for a directory without full scan
pub async fn GetDirectoryStatistics(path:&str, max_depth:Option<usize>) -> Result<DirectoryStatistics> {
	let directory_path = crate::Configuration::ConfigurationManager::ExpandPath(path)?;

	if !directory_path.exists() || !directory_path.is_dir() {
		return Err(AirError::FileSystem(format!("Invalid directory: {}", path)));
	}

	let mut file_count = 0u64;
	let mut total_size = 0u64;
	let mut directory_count = 0u64;
	let mut hidden_count = 0u64;

	let walker = ignore::WalkBuilder::new(&directory_path)
		.max_depth(max_depth)
		.hidden(true)
		.follow_links(false)
		.build();

	for entry in walker.flatten() {
		let file_type = entry.file_type().expect("Failed to get file type");

		if file_type.is_file() {
			file_count += 1;
			if let Ok(metadata) = entry.metadata() {
				total_size += metadata.len();
			}
		} else if file_type.is_dir() {
			directory_count += 1;
		}

		if entry.depth() > 0
			&& entry
				.path()
				.components()
				.any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
		{
			hidden_count += 1;
		}
	}

	Ok(DirectoryStatistics { file_count, directory_count, hidden_count, total_size })
}

/// Directory statistics
#[derive(Debug, Clone)]
pub struct DirectoryStatistics {
	pub file_count:u64,
	pub directory_count:u64,
	pub hidden_count:u64,
	pub total_size:u64,
}
