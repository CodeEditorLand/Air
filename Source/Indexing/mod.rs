//! # File Indexing and Search Service
//!
//! ## File: Indexing/mod.rs
//!
//! ## Role in Air Architecture
//!
//! Provides comprehensive file indexing, search, and content analysis
//! capabilities for the Land ecosystem, inspired by and compatible with
//! Visual Studio Code's search service.
//!
//! ## Primary Responsibility
//!
//! Facade module for the Indexing service, exposing the public API for
//! file indexing, search, and symbol extraction operations.
//!
//! ## Secondary Responsibilities
//!
//! - Re-export public types from submodule
//! - Provide unified FileIndexer API
//! - Coordinate between indexing subsystems
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `regex` - Regular expression search patterns
//! - `serde` - Serialization for index storage
//! - `tokio` - Async runtime for all operations
//! - `notify` - File system watching
//! - `chrono` - Timestamp management
//!
//! **Internal Modules:**
//! - `crate::Result` - Error handling type
//! - `crate::AirError` - Error types
//! - `crate::ApplicationState::ApplicationState` - Application state
//! - `crate::Configuration::ConfigurationManager` - Configuration management
//!
//! ## Dependents
//!
//! - `Indexing::FileIndexer` - Main indexer implementation
//! - `Vine::Server::AirVinegRPCService` - gRPC integration
//!
//! ## VSCode Integration
//!
//! This service integrates with VSCode's search and file service architecture:
//!
//! - References: vs/workbench/services/search
//! - File Service: vs/workbench/services/files
//!
//! The indexing system supports VSCode features:
//! - **Outline View**: Symbol extraction for class/function navigation
//! - **Go to Symbol**: Cross-file symbol search and lookup
//! - **Search Integration**: File content and name search with regex support
//! - **Workspace Search**: Multi-workspace index sharing
//!
//! ## TODO
//!
//! - [ ] Implement full ripgrep integration for ultra-fast text search
//! - [ ] Add project-level search with workspace awareness
//! - [ ] Implement search query caching
//! - [ ] Add fuzzy search with typos tolerance
//! - [ ] Implement search history and recent queries
//! - [ ] Add search result preview with context
//! - [ ] Implement parallel indexing for large directories

// Re-export public state types
pub mod State {
	pub mod CreateState;
	pub mod UpdateState;
}

// Re-export scanning modules
pub mod Scan {
	pub mod ScanDirectory;
	pub mod ScanFile;
}

// Re-export processing modules
pub mod Process {
	pub mod ProcessContent;
	pub mod ExtractSymbols;
}

// Re-export language modules
pub mod Language {
	pub mod ParseRust;
	pub mod ParseTypeScript;
}

// Re-export store modules
pub mod Store {
	pub mod StoreEntry;
	pub mod QueryIndex;
	pub mod UpdateIndex;
}

// Re-export watch module
pub mod Watch {
	pub mod WatchFile;
}

// Re-export background module
pub mod Background {
	pub mod StartWatcher;
}

// Re-export commonly used types at the root level
// Import types and functions needed for the FileIndexer implementation
use std::{collections::HashMap, path::PathBuf, sync::Arc};

use tokio::sync::{Mutex, RwLock};

use crate::{AirError, ApplicationState::ApplicationState, Configuration::ConfigurationManager, Result};

/// Maximum number of parallel indexing operations
const MAX_PARALLEL_INDEXING:usize = 10;

/// Indexing result with statistics
#[derive(Debug, Clone)]
pub struct IndexResult {
	/// Number of files successfully indexed
	pub files_indexed:u32,
	/// Total size of indexed files in bytes
	pub total_size:u64,
	/// Time taken in seconds
	pub duration_seconds:f64,
	/// Number of symbols extracted
	pub symbols_extracted:u32,
	/// Number of files with errors
	pub files_with_errors:u32,
}

/// Index statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IndexStatistics {
	pub file_count:u32,
	pub total_size:u64,
	pub total_symbols:u32,
	pub language_counts:HashMap<String, u32>,
	pub last_updated:chrono::DateTime<chrono::Utc>,
	pub index_version:String,
}

/// File indexer implementation with comprehensive search capabilities
///
/// This indexer provides:
/// - Incremental file watching with real-time updates
/// - Multi-mode search (literal, regex, fuzzy)
/// - Symbol extraction for VSCode Outline View
/// - Language detection for syntax highlighting
/// - Index corruption detection and recovery
/// - Parallel indexing with resource limits
pub struct FileIndexer {
	/// Application state
	AppState:Arc<ApplicationState>,

	/// File index with metadata and symbols
	file_index:Arc<RwLock<FileIndex>>,

	/// Index storage directory
	index_directory:PathBuf,

	/// File watcher for incremental updates
	file_watcher:Arc<Mutex<Option<notify::RecommendedWatcher>>>,

	/// Semaphore for limiting parallel indexing operations
	indexing_semaphore:Arc<tokio::sync::Semaphore>,

	/// Index corruption detection state
	corruption_detected:Arc<Mutex<bool>>,
}

impl FileIndexer {
	/// Create a new file indexer with comprehensive setup
	///
	/// Initializes the indexer with:
	/// - Index directory creation
	/// - Existing index loading or fresh creation
	/// - Index corruption detection
	/// - Service status initialization
	pub async fn new(AppState:Arc<ApplicationState>) -> Result<Self> {
		let config = &AppState.Configuration.Indexing;

		// Expand index directory path with validation
		let index_directory = Self::ValidateAndExpandPath(&config.IndexDirectory)?;

		// Create index directory if it doesn't exist with error handling
		Store::EnsureIndexDirectory(&index_directory).await?;

		// Load or create index with corruption detection
		let file_index = Store::LoadOrCreateIndex(&index_directory).await?;

		let indexer = Self {
			AppState:AppState.clone(),
			file_index:Arc::new(RwLock::new(file_index)),
			index_directory:index_directory.clone(),
			file_watcher:Arc::new(Mutex::new(None)),
			indexing_semaphore:Arc::new(tokio::sync::Semaphore::new(MAX_PARALLEL_INDEXING)),
			corruption_detected:Arc::new(Mutex::new(false)),
		};

		// Verify index integrity
		indexer.VerifyIndexIntegrity().await?;

		// Initialize service status
		indexer
			.AppState
			.UpdateServiceStatus("indexing", crate::ApplicationState::ServiceStatus::Running)
			.await
			.map_err(|e| AirError::Internal(e.to_string()))?;

		log::info!("[FileIndexer] Initialized with index directory: {}", index_directory.display());

		Ok(indexer)
	}

	/// Validate and expand path with traversal protection
	fn ValidateAndExpandPath(path:&str) -> Result<PathBuf> {
		let expanded = ConfigurationManager::ExpandPath(path)?;

		// Prevent path traversal attacks
		let path_str = expanded.to_string_lossy();
		if path_str.contains("..") {
			return Err(AirError::FileSystem("Path contains invalid traversal sequence".to_string()));
		}

		Ok(expanded)
	}

	/// Verify index integrity and detect corruption
	async fn VerifyIndexIntegrity(&self) -> Result<()> {
		let index = self.file_index.read().await;

		// Check consistency
		State::ValidateIndexConsistency(&index)?;

		// Verify all indexed files exist
		let mut missing_files = 0;
		for file_path in index.files.keys() {
			if !file_path.exists() {
				missing_files += 1;
			}
		}

		if missing_files > 0 {
			log::warn!("[FileIndexer] Found {} missing files in index", missing_files);
		}

		log::info!("[FileIndexer] Index integrity verified successfully");

		Ok(())
	}

	/// Index a directory with comprehensive validation and parallel processing
	pub async fn IndexDirectory(&self, path:String, patterns:Vec<String>) -> Result<IndexResult> {
		let start_time = std::time::Instant::now();

		log::info!("[FileIndexer] Starting directory index: {}", path);

		let config = &self.AppState.Configuration.Indexing;

		// Scan directory
		let (files_to_index, scan_result) =
			Scan::ScanDirectoriesParallel(vec![path], patterns.clone(), config, MAX_PARALLEL_INDEXING).await?;

		// Index files in parallel
		let index_arc = self.file_index.clone();
		let semaphore = self.indexing_semaphore.clone();
		let config_clone = config.clone();
		let mut index_tasks = Vec::new();

		for file_path in files_to_index {
			let permit = semaphore.clone().acquire_owned().await.unwrap();
			let index_ref = index_arc.clone();
			let config_for_task = config_clone.clone();

			let task = tokio::spawn(async move {
				let _permit = permit;
				Scan::IndexFileInternal(&file_path, &config_for_task, &index_ref, &[]).await
			});

			index_tasks.push(task);
		}

		// Collect results
		let mut index = self.file_index.write().await;
		let mut indexed_paths = std::collections::HashSet::new();
		let mut files_indexed = 0u32;
		let mut total_size = 0u64;
		let mut symbols_extracted = 0u32;
		let mut files_with_errors = 0u32;

		for task in index_tasks {
			match task.await {
				Ok(Ok((metadata, symbols))) => {
					let file_path = metadata.path.clone();

					index.files.insert(file_path.clone(), metadata.clone());
					indexed_paths.insert(file_path.clone());

					// Index content for search
					if let Err(e) = Store::UpdateFileContent(&mut index, &file_path, &metadata).await {
						log::warn!("[FileIndexer] Failed to index content for {}: {}", file_path.display(), e);
					}

					// Index symbols
					index.file_symbols.insert(file_path.clone(), symbols.clone());
					symbols_extracted += symbols.len() as u32;

					// Update symbol index
					for symbol in symbols {
						index
							.symbol_index
							.entry(symbol.name.clone())
							.or_insert_with(Vec::new)
							.push(State::SymbolLocation { file_path:file_path.clone(), line:symbol.line, symbol });
					}

					files_indexed += 1;
					total_size += metadata.size;
				},
				Ok(Err(_)) => {
					files_with_errors += 1;
				},
				Err(e) => {
					log::error!("[FileIndexer] Indexing task failed: {}", e);
					files_with_errors += 1;
				},
			}
		}

		// Remove files that were indexed before but no longer exist
		Scan::ScanAndRemoveDeleted(&mut index, &ScanDirectory::ValidateAndExpandPath(&path)?).await?;

		// Update index metadata
		State::UpdateIndexMetadata(&mut index)?;

		// Save index to disk
		Store::SaveIndex(&self.index_directory, &index).await?;

		let duration = start_time.elapsed().as_secs_f64();

		log::info!(
			"[FileIndexer] Indexing completed: {} files, {} bytes, {} symbols, {} errors in {:.2}s",
			files_indexed,
			total_size,
			symbols_extracted,
			files_with_errors,
			duration
		);

		Ok(IndexResult {
			files_indexed,
			total_size,
			duration_seconds:duration,
			symbols_extracted,
			files_with_errors,
		})
	}

	/// Search files with multiple modes
	pub async fn SearchFiles(
		&self,
		query:SearchQuery,
		path:Option<String>,
		language:Option<String>,
	) -> Result<PaginatedSearchResults> {
		let index = self.file_index.read().await;
		Store::QueryIndexSearch(&index, query, path, language).await
	}

	/// Search symbols across all files (for VSCode Go to Symbol)
	pub async fn SearchSymbols(&self, query:&str, max_results:u32) -> Result<Vec<SymbolInfo>> {
		let index = self.file_index.read().await;
		let query_lower = query.to_lowercase();
		let mut results = Vec::new();

		for (symbol_name, locations) in &index.symbol_index {
			if symbol_name.to_lowercase().contains(&query_lower) {
				for loc in locations.iter().take(max_results as usize) {
					results.push(loc.symbol.clone());
					if results.len() >= max_results as usize {
						break;
					}
				}
			}
		}

		Ok(results)
	}

	/// Get symbols for a specific file (for VSCode Outline View)
	pub async fn GetFileSymbols(&self, file_path:&PathBuf) -> Result<Vec<SymbolInfo>> {
		let index = self.file_index.read().await;
		Ok(index.file_symbols.get(file_path).cloned().unwrap_or_default())
	}

	/// Get file information
	pub async fn GetFileInfo(&self, path:String) -> Result<Option<FileMetadata>> {
		let file_path = Self::ValidateAndExpandPath(&path)?;
		let index = self.file_index.read().await;

		Ok(index.files.get(&file_path).cloned())
	}

	/// Get index statistics
	pub async fn GetIndexStatistics(&self) -> Result<IndexStatistics> {
		let index = self.file_index.read().await;

		let mut language_counts:HashMap<String, u32> = HashMap::new();
		let total_size = index.files.values().map(|m| m.size).sum();
		let total_symbols = index.files.values().map(|m| m.symbol_count).sum();

		for metadata in index.files.values() {
			if let Some(lang) = &metadata.language {
				*language_counts.entry(lang.clone()).or_insert(0) += 1;
			}
		}

		Ok(IndexStatistics {
			file_count:index.files.len() as u32,
			total_size,
			total_symbols,
			language_counts,
			last_updated:index.last_updated,
			index_version:index.index_version.clone(),
		})
	}

	/// Recover corrupted index
	pub async fn recover_from_corruption(&self) -> Result<()> {
		log::info!("[FileIndexer] Recovering from corrupted index...");

		// Backup corrupted index
		Store::BackupCorruptedIndex(&self.index_directory).await?;

		// Create new index
		let new_index = State::CreateNewIndex();
		*self.file_index.write().await = new_index;

		// Clear corruption flag
		*self.corruption_detected.lock().await = false;

		log::info!("[FileIndexer] Index recovery completed");

		Ok(())
	}
}

impl Clone for FileIndexer {
	fn clone(&self) -> Self {
		Self {
			AppState:self.AppState.clone(),
			file_index:self.file_index.clone(),
			index_directory:self.index_directory.clone(),
			file_watcher:self.file_watcher.clone(),
			indexing_semaphore:self.indexing_semaphore.clone(),
			corruption_detected:self.corruption_detected.clone(),
		}
	}
}
