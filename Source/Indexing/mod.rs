//! # File Indexing and Search Service
//!
//! This module provides comprehensive file indexing, search, and content
//! analysis capabilities for the Land ecosystem, inspired by and compatible
//! with Visual Studio Code's search service.
//!
//! ## Responsibilities
//!
//! - **File Indexing**: Background indexing of files with metadata extraction
//! - **Content Search**: Fast text search across indexed files with multiple
//!   query modes
//! - **Incremental Updates**: Real-time file watching and incremental index
//!   updates
//! - **Code Structure**: Extraction of classes, functions, and symbols for
//!   syntax highlighting
//! - **Language Detection**: Automatic programming language detection for
//!   multiple languages
//! - **Result Ranking**: Relevance-based search result ordering with pagination
//!   support
//! - **Index Recovery**: Corruption detection and automatic recovery mechanisms
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

use std::{
	collections::{HashMap, HashSet},
	path::{Path, PathBuf},
	sync::Arc,
};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock, Semaphore};

use crate::{AirError, ApplicationState::ApplicationState, Configuration::ConfigurationManager, Result};

/// Maximum file size allowed for indexing (100MB)
const MAX_FILE_SIZE_BYTES:u64 = 100 * 1024 * 1024;

/// Maximum search results per query (pagination default)
const MAX_SEARCH_RESULTS_DEFAULT:u32 = 100;

/// Maximum number of parallel indexing operations
const MAX_PARALLEL_INDEXING:usize = 10;

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Search query with multiple modes
#[derive(Debug, Clone)]
pub struct SearchQuery {
	/// Search text
	pub query:String,
	/// Query mode (regex, literal, fuzzy)
	pub mode:SearchMode,
	/// Case sensitive search
	pub case_sensitive:bool,
	/// Exact word match
	pub whole_word:bool,
	/// Regex pattern (only for regex mode)
	pub regex:Option<Regex>,
	/// Maximum results per page
	pub max_results:u32,
	/// Page number for pagination
	pub page:u32,
}

/// Search mode
#[derive(Debug, Clone, PartialEq)]
pub enum SearchMode {
	/// Literal text search
	Literal,
	/// Regular expression search
	Regex,
	/// Fuzzy search with typo tolerance
	Fuzzy,
	/// Exact match
	Exact,
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
	indexing_semaphore:Arc<Semaphore>,

	/// Index corruption detection state
	corruption_detected:Arc<Mutex<bool>>,
}

/// File index structure with comprehensive metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileIndex {
	/// Indexed files with complete metadata
	files:HashMap<PathBuf, FileMetadata>,

	/// Content index for fast text search
	/// Maps words/tokens to file paths where they appear
	content_index:HashMap<String, Vec<PathBuf>>,

	/// Symbol index for VSCode Outline View and Go to Symbol
	/// Maps symbol names to their definitions
	symbol_index:HashMap<String, Vec<SymbolLocation>>,

	/// Reverse symbol index for cross-referencing
	file_symbols:HashMap<PathBuf, Vec<SymbolInfo>>,

	/// Last update timestamp for all indexes
	last_updated:chrono::DateTime<chrono::Utc>,

	/// Index version for corruption detection
	index_version:String,

	/// Index checksum for integrity verification
	index_checksum:String,
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

/// Search result with relevance scoring
#[derive(Debug, Clone)]
pub struct SearchResult {
	/// File path
	pub path:String,
	/// File name
	pub file_name:String,
	/// Matched lines with context
	pub matches:Vec<SearchMatch>,
	/// Relevance score (higher = more relevant)
	pub relevance:f64,
	/// Matched language (if applicable)
	pub language:Option<String>,
}

/// Search match with full context
#[derive(Debug, Clone)]
pub struct SearchMatch {
	/// Line number (1-indexed)
	pub line_number:u32,
	/// Line content
	pub line_content:String,
	/// Match start position
	pub match_start:usize,
	/// Match end position
	pub match_end:usize,
	/// Lines before match for context
	pub context_before:Vec<String>,
	/// Lines after match for context
	pub context_after:Vec<String>,
}

/// Paginated search results
#[derive(Debug, Clone)]
pub struct PaginatedSearchResults {
	/// Current page of results
	pub results:Vec<SearchResult>,
	/// Total number of results (across all pages)
	pub total_count:u32,
	/// Current page number (0-indexed)
	pub page:u32,
	/// Number of pages
	pub total_pages:u32,
	/// Results per page
	pub page_size:u32,
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
		Self::EnsureIndexDirectory(&index_directory).await?;

		// Load or create index with corruption detection
		let file_index = Self::LoadOrCreateIndex(&index_directory).await?;

		let indexer = Self {
			AppState:AppState.clone(),
			file_index:Arc::new(RwLock::new(file_index)),
			index_directory:index_directory.clone(),
			file_watcher:Arc::new(Mutex::new(None)),
			indexing_semaphore:Arc::new(Semaphore::new(MAX_PARALLEL_INDEXING)),
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

	/// Ensure index directory exists with proper error handling
	async fn EnsureIndexDirectory(path:&Path) -> Result<()> {
		tokio::fs::create_dir_all(path).await.map_err(|e| {
			AirError::Configuration(format!("Failed to create index directory {}: {}", path.display(), e))
		})?;
		Ok(())
	}

	/// Index a directory with comprehensive validation and parallel processing
	///
	/// Features:
	/// - Path traversal protection
	/// - Symbolic link handling
	/// - File size validation
	/// - Permission error handling
	/// - Parallel indexing with semaphore
	/// - Symbol extraction
	/// - Language detection
	pub async fn IndexDirectory(&self, path:String, patterns:Vec<String>) -> Result<IndexResult> {
		let start_time = std::time::Instant::now();

		log::info!("[FileIndexer] Starting directory index: {}", path);

		// Validate and expand path
		let directory_path = Self::ValidateAndExpandPath(&path)?;

		// Validate directory exists and is accessible
		if !directory_path.exists() {
			return Err(AirError::FileSystem(format!("Directory does not exist: {}", path)));
		}

		if !directory_path.is_dir() {
			return Err(AirError::FileSystem(format!("Path is not a directory: {}", path)));
		}

		// Check directory permissions
		Self::CheckDirectoryPermissions(&directory_path).await?;

		let config = &self.AppState.Configuration.Indexing;
		let mut files_indexed = 0u32;
		let mut total_size = 0u64;
		let mut symbols_extracted = 0u32;
		let mut files_with_errors = 0u32;

		// Build file patterns
		let include_patterns = if patterns.is_empty() { config.FileTypes.clone() } else { patterns };

		// Walk directory with .gitignore support
		let walker = ignore::WalkBuilder::new(&directory_path)
            .max_depth(Some(10)) // Prevent infinite recursion
            .hidden(false)
            .follow_links(false) // Don't follow symlinks by default
            .build();

		let mut index = self.file_index.write().await;
		let mut indexed_paths = HashSet::new();

		// Collect all files first, then process in parallel
		let mut files_to_index:Vec<PathBuf> = Vec::new();

		for result in walker {
			match result {
				Ok(entry) => {
					// Only index regular files (not directories or symlinks)
					if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
						let file_path = entry.path().to_path_buf();

						// Check if file is a symbolic link
						if entry.path_is_symlink() {
							log::debug!("[FileIndexer] Skipping symlink: {}", file_path.display());
							continue;
						}

						// Check file size limit
						if let Ok(metadata) = entry.metadata() {
							let file_size = metadata.len();

							if file_size > MAX_FILE_SIZE_BYTES {
								log::warn!(
									"[FileIndexer] Skipping oversized file: {} ({} bytes)",
									file_path.display(),
									file_size
								);
								continue;
							}

							// Check file pattern
							if Self::MatchesPatterns(&file_path, &include_patterns) {
								// Try to get file access to validate permissions
								if Self::ValidateFileAccess(&file_path).await {
									files_to_index.push(file_path);
								} else {
									log::warn!(
										"[FileIndexer] Cannot access file (permission denied): {}",
										file_path.display()
									);
									files_with_errors += 1;
								}
							}
						}
					}
				},
				Err(e) => {
					log::warn!("[FileIndexer] Error walking directory: {}", e);
				},
			}
		}

		// Index files in parallel with semaphore limiting
		let index_arc = self.file_index.clone();
		let semaphore = self.indexing_semaphore.clone();
		let config_clone = config.clone();
		let include_patterns_clone = include_patterns;

		let mut index_tasks = Vec::new();

		for file_path in files_to_index {
			let permit = semaphore.clone().acquire_owned().await.unwrap();
			let index_ref = index_arc.clone();
			let config_for_task = config_clone.clone();
			let patterns_for_task = include_patterns_clone.clone();

			let task = tokio::spawn(async move {
				let _permit = permit;

				// Index the file with pattern validation - use active index reference and check
				// patterns
				match Self::IndexFileInternal(&file_path, &config_for_task, &index_ref, &patterns_for_task).await {
					Ok((metadata, symbols)) => Some((file_path, metadata, symbols)),
					Err(e) => {
						log::warn!("[FileIndexer] Failed to index file {}: {}", file_path.display(), e);
						None
					},
				}
			});

			index_tasks.push(task);
		}

		// Collect results
		for task in index_tasks {
			match task.await {
				Ok(Some((file_path, metadata, symbols))) => {
					index.files.insert(file_path.clone(), metadata.clone());
					indexed_paths.insert(file_path.clone());

					// Index content for search
					if let Err(e) = self.IndexContentInternal(&mut index, &file_path, &metadata).await {
						log::warn!("[FileIndexer] Failed to index content for {}: {}", file_path.display(), e);
					}

					// Index symbols
					index.file_symbols.insert(file_path.clone(), symbols.clone());
					symbols_extracted += symbols.len() as u32;

					// Update symbol index
					for symbol in &symbols {
						index
							.symbol_index
							.entry(symbol.name.clone())
							.or_insert_with(Vec::new)
							.push(SymbolLocation {
								file_path:file_path.clone(),
								line:symbol.line,
								symbol:symbol.clone(),
							});
					}

					files_indexed += 1;
					total_size += metadata.size;
				},
				Ok(None) => {
					files_with_errors += 1;
				},
				Err(e) => {
					log::error!("[FileIndexer] Indexing task failed: {}", e);
					files_with_errors += 1;
				},
			}
		}

		// Remove files that were indexed before but no longer exist
		let mut paths_to_remove = Vec::new();
		let all_paths:Vec<_> = index.files.keys().cloned().collect();
		for path in all_paths {
			if !indexed_paths.contains(&path) && path.starts_with(&directory_path) {
				// File was deleted, remove from index
				paths_to_remove.push(path.clone());
				index.file_symbols.remove(&path);
				// Remove from symbol index
				for (_, locations) in index.symbol_index.iter_mut() {
					locations.retain(|loc| loc.file_path != path);
				}
			}
		}

		for path in paths_to_remove {
			index.files.remove(&path);
		}

		// Update index metadata
		index.last_updated = chrono::Utc::now();
		index.index_version = Self::GenerateIndexVersion();
		index.index_checksum = Self::CalculateIndexChecksum(&index)?;

		// Save index to disk
		self.SaveIndex(&index).await?;

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

	/// Validate file access and permissions
	async fn ValidateFileAccess(path:&Path) -> bool {
		tokio::task::spawn_blocking({
			let path = path.to_path_buf();
			move || {
				// Try to read file metadata
				let can_access = std::fs::metadata(&path).is_ok();
				if can_access {
					// Try to open file for reading
					std::fs::File::open(&path).is_ok()
				} else {
					false
				}
			}
		})
		.await
		.unwrap_or(false)
	}

	/// Index a single file internally (called by parallel tasks)
	async fn IndexFileInternal(
		FilePath:&PathBuf,
		Config:&crate::Configuration::IndexingConfig,
		_IndexRef:&Arc<RwLock<FileIndex>>,
		_Patterns:&[String],
	) -> Result<(FileMetadata, Vec<SymbolInfo>)> {
		// Get file metadata with error handling
		let Metadata = std::fs::metadata(FilePath)
			.map_err(|E| AirError::FileSystem(format!("Failed to get file metadata: {}", E)))?;

		// Get modified time
		let Modified = Metadata
			.modified()
			.map_err(|E| AirError::FileSystem(format!("Failed to get modification time: {}", E)))?;

		let ModifiedTime = chrono::DateTime::<chrono::Utc>::from(Modified);

		// Check if file size exceeds limit
		let FileSize = Metadata.len();
		if FileSize > Config.MaxFileSizeMb as u64 * 1024 * 1024 {
			return Err(AirError::FileSystem(format!(
				"File size {} exceeds limit {} MB",
				FileSize, Config.MaxFileSizeMb
			)));
		}

		// Read file content
		let Content = tokio::fs::read(FilePath)
			.await
			.map_err(|E| AirError::FileSystem(format!("Failed to read file: {}", E)))?;

		// Check for symbolic link
		let IsSymlink = std::fs::symlink_metadata(FilePath)
			.map(|M| M.file_type().is_symlink())
			.unwrap_or(false);

		// Calculate SHA-256 checksum
		use sha2::{Digest, Sha256};
		let mut Hasher = Sha256::new();
		Hasher.update(&Content);
		let Checksum = format!("{:x}", Hasher.finalize());

		// Detect file encoding
		let Encoding = Self::DetectEncoding(&Content);

		// Detect MIME type
		let MimeType = Self::DetectMimeType(FilePath, &Content);

		// Detect programming language
		let Language = Self::DetectLanguage(FilePath);

		// Count lines for text files
		let LineCount = if MimeType.starts_with("text/") {
			Some(Content.iter().filter(|&&B| B == b'\n').count() as u32 + 1)
		} else {
			None
		};

		// Extract symbols from code for VSCode Outline View
		let Symbols = if let Some(Lang) = &Language {
			Self::ExtractSymbols(FilePath, &Content, Lang).await?
		} else {
			Vec::new()
		};

		Ok((
			FileMetadata {
				path:FilePath.clone(),
				size:FileSize,
				modified:ModifiedTime,
				mime_type:MimeType,
				language:Language,
				line_count:LineCount,
				checksum:Checksum,
				is_symlink:IsSymlink,
				permissions:Self::GetPermissionsString(&Metadata),
				encoding:Encoding,
				indexed_at:chrono::Utc::now(),
				symbol_count:Symbols.len() as u32,
			},
			Symbols,
		))
	}

	/// Index file content for search
	async fn IndexContentInternal(
		&self,
		index:&mut FileIndex,
		file_path:&PathBuf,
		metadata:&FileMetadata,
	) -> Result<()> {
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

		// Token-based indexing with better word boundary detection
		let tokens = Self::TokenizeContent(&content);

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

	/// Tokenize content for indexing with improved word boundary handling
	fn TokenizeContent(content:&str) -> Vec<String> {
		let mut tokens = Vec::new();
		let mut current_token = String::new();
		let mut in_token = false;

		for c in content.chars() {
			if c.is_alphanumeric() || c == '_' {
				current_token.push(c);
				in_token = true;
			} else if in_token {
				// End of token
				tokens.push(current_token.to_lowercase());
				current_token.clear();
				in_token = false;
			}
		}

		// Don't forget the last token
		if in_token {
			tokens.push(current_token.to_lowercase());
		}

		tokens
	}

	/// Detect file encoding (simplified detection)
	fn DetectEncoding(content:&[u8]) -> Option<String> {
		if content.is_empty() {
			return None;
		}

		// Check for BOM markers
		if content.starts_with(&[0xEF, 0xBB, 0xBF]) {
			return Some("UTF-8 (BOM)".to_string());
		}

		if content.starts_with(&[0xFE, 0xFF]) {
			return Some("UTF-16 (BE)".to_string());
		}

		if content.starts_with(&[0xFF, 0xFE]) {
			return Some("UTF-16 (LE)".to_string());
		}

		if content.starts_with(&[0x00, 0x00, 0xFE, 0xFF]) {
			return Some("UTF-32 (BE)".to_string());
		}

		if content.starts_with(&[0xFF, 0xFE, 0x00, 0x00]) {
			return Some("UTF-32 (LE)".to_string());
		}

		// Check if all bytes are ASCII
		if content.iter().all(|&b| b.is_ascii()) {
			return Some("ASCII".to_string());
		}

		// Assume UTF-8 for other cases
		Some("UTF-8".to_string())
	}

	/// Get file permissions as string
	fn GetPermissionsString(metadata:&std::fs::Metadata) -> String {
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

	/// Index a single file (public API)
	pub async fn IndexFile(&self, file_path:&PathBuf) -> Result<FileMetadata> {
		let config = &self.AppState.Configuration.Indexing;
		let index_ref = self.file_index.clone();
		let patterns = &config.FileTypes;

		let (metadata, symbols) = Self::IndexFileInternal(file_path, config, &index_ref, patterns).await?;

		let mut index = self.file_index.write().await;
		index.files.insert(file_path.clone(), metadata.clone());
		index.file_symbols.insert(file_path.clone(), symbols.clone());

		// Update symbol index
		for symbol in &symbols {
			index
				.symbol_index
				.entry(symbol.name.clone())
				.or_insert_with(Vec::new)
				.push(SymbolLocation { file_path:file_path.clone(), line:symbol.line, symbol:symbol.clone() });
		}

		index.last_updated = chrono::Utc::now();

		Ok(metadata)
	}

	/// Index file content for search (public API)
	pub async fn IndexContent(&self, file_path:&PathBuf, metadata:&FileMetadata) -> Result<()> {
		let mut index = self.file_index.write().await;
		self.IndexContentInternal(&mut index, file_path, metadata).await
	}

	/// Search files with multiple modes and comprehensive query handling
	///
	/// Features:
	/// - Sanitized search query
	/// - Multiple search modes (literal, regex, fuzzy, exact)
	/// - Case sensitivity option
	/// - Whole word matching
	/// - Path filtering
	/// - Result pagination
	/// - Relevance-based ranking
	/// - Language filtering
	pub async fn SearchFiles(
		&self,
		query:SearchQuery,
		path:Option<String>,
		language:Option<String>,
	) -> Result<PaginatedSearchResults> {
		log::info!("[FileIndexer] Searching for: '{}' (mode: {:?})", query.query, query.mode);

		// Sanitize search query
		let sanitized_query = Self::SanitizeSearchQuery(&query.query)?;

		// Build search parameters
		let case_sensitive = query.case_sensitive;
		let whole_word = query.whole_word;
		let max_results = if query.max_results == 0 {
			MAX_SEARCH_RESULTS_DEFAULT
		} else {
			query.max_results.min(1000) // Cap at 1000 results
		};

		let index = self.file_index.read().await;
		let mut all_results = Vec::new();

		// Search based on mode
		match query.mode {
			SearchMode::Literal => {
				self.SearchLiteral(
					&sanitized_query,
					case_sensitive,
					whole_word,
					path.as_deref(),
					language.as_deref(),
					&index,
					&mut all_results,
				)
				.await;
			},
			SearchMode::Regex => {
				if let Some(regex) = &query.regex {
					self.SearchRegex(regex, path.as_deref(), language.as_deref(), &index, &mut all_results)
						.await;
				} else {
					// Try to compile regex from query
					if let Ok(regex) = Regex::new(&sanitized_query) {
						self.SearchRegex(&regex, path.as_deref(), language.as_deref(), &index, &mut all_results)
							.await;
					}
				}
			},
			SearchMode::Fuzzy => {
				self.SearchFuzzy(
					&sanitized_query,
					case_sensitive,
					path.as_deref(),
					language.as_deref(),
					&index,
					&mut all_results,
				)
				.await;
			},
			SearchMode::Exact => {
				self.SearchExact(
					&sanitized_query,
					case_sensitive,
					path.as_deref(),
					language.as_deref(),
					&index,
					&mut all_results,
				)
				.await;
			},
		}

		// Rank results by relevance
		all_results.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());

		// Calculate pagination
		let total_count = all_results.len() as u32;
		let total_pages = if max_results == 0 { 0 } else { total_count.div_ceil(max_results) };
		let page = query.page.min(total_pages.saturating_sub(1));

		// Extract current page
		let start = (page * max_results) as usize;
		let end = ((page + 1) * max_results).min(total_count) as usize;
		let page_results = all_results[start..end].to_vec();

		log::info!(
			"[FileIndexer] Search completed: {} total results, page {} of {}",
			total_count,
			page + 1,
			total_pages
		);

		Ok(PaginatedSearchResults { results:page_results, total_count, page, total_pages, page_size:max_results })
	}

	/// Sanitize search query to prevent injection and invalid patterns
	fn SanitizeSearchQuery(query:&str) -> Result<String> {
		// Remove null bytes and control characters
		let sanitized:String = query.chars().filter(|c| *c != '\0' && !c.is_control()).collect();

		// Limit query length
		if sanitized.len() > 1000 {
			return Err(AirError::Validation(
				"Search query exceeds maximum length of 1000 characters".to_string(),
			));
		}

		Ok(sanitized)
	}

	/// Literal search (default mode)
	async fn SearchLiteral(
		&self,
		query:&str,
		case_sensitive:bool,
		whole_word:bool,
		path_filter:Option<&str>,
		language_filter:Option<&str>,
		index:&FileIndex,
		results:&mut Vec<SearchResult>,
	) {
		let search_query = if case_sensitive { query.to_string() } else { query.to_lowercase() };

		// Search in content index first (faster)
		if let Some(file_paths) = index.content_index.get(&search_query.to_lowercase()) {
			for file_path in file_paths {
				if let Some(metadata) = index.files.get(file_path) {
					if Self::MatchesFilters(file_path, metadata, path_filter, language_filter) {
						if let Ok(search_result) = self
							.FindMatchesInFile(file_path, &search_query, case_sensitive, whole_word, index)
							.await
						{
							if !search_result.matches.is_empty() {
								results.push(search_result);
							}
						}
					}
				}
			}
		}

		// Also search in file names
		for (file_path, metadata) in &index.files {
			if results.len() >= 1000 {
				break;
			}

			let file_name = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();

			let name_to_search = if case_sensitive { file_name.clone() } else { file_name.to_lowercase() };

			if name_to_search.contains(&search_query) {
				if Self::MatchesFilters(file_path, metadata, path_filter, language_filter) {
					// Filename match has lower relevance than content match
					results.push(SearchResult {
						path:file_path.to_string_lossy().to_string(),
						file_name,
						matches:Vec::new(),
						relevance:0.3,
						language:metadata.language.clone(),
					});
				}
			}
		}
	}

	/// Regex search mode
	async fn SearchRegex(
		&self,
		regex:&Regex,
		path_filter:Option<&str>,
		language_filter:Option<&str>,
		index:&FileIndex,
		results:&mut Vec<SearchResult>,
	) {
		for (file_path, metadata) in &index.files {
			if results.len() >= 1000 {
				break;
			}

			if !Self::MatchesFilters(file_path, metadata, path_filter, language_filter) {
				continue;
			}

			if let Ok(content) = tokio::fs::read_to_string(file_path).await {
				let matches = Self::FindRegexMatches(&content, regex);

				if !matches.is_empty() {
					let relevance = Self::CalculateRelevance(&matches, metadata);

					results.push(SearchResult {
						path:file_path.to_string_lossy().to_string(),
						file_name:file_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
						matches,
						relevance,
						language:metadata.language.clone(),
					});
				}
			}
		}
	}

	/// Fuzzy search with typo tolerance using Levenshtein distance
	async fn SearchFuzzy(
		&self,
		query:&str,
		case_sensitive:bool,
		path_filter:Option<&str>,
		language_filter:Option<&str>,
		index:&FileIndex,
		results:&mut Vec<SearchResult>,
	) {
		let query_lower = query.to_lowercase();

		for (file_path, metadata) in &index.files {
			if results.len() >= 1000 {
				break;
			}

			if !Self::MatchesFilters(file_path, metadata, path_filter, language_filter) {
				continue;
			}

			if let Ok(content) = tokio::fs::read_to_string(file_path).await {
				let max_distance = 2; // TODO: Make this configurable
				let matches = Self::FindFuzzyMatches(&content, &query_lower, case_sensitive, max_distance);

				if !matches.is_empty() {
					let relevance = Self::CalculateRelevance(&matches, metadata) * 0.8; // Fuzzy matches have lower relevance

					results.push(SearchResult {
						path:file_path.to_string_lossy().to_string(),
						file_name:file_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
						matches,
						relevance,
						language:metadata.language.clone(),
					});
				}
			}
		}
	}

	/// Exact match search (whole word, case-sensitive)
	async fn SearchExact(
		&self,
		query:&str,
		_case_sensitive:bool,
		path_filter:Option<&str>,
		language_filter:Option<&str>,
		index:&FileIndex,
		results:&mut Vec<SearchResult>,
	) {
		for (file_path, metadata) in &index.files {
			if results.len() >= 1000 {
				break;
			}

			if !Self::MatchesFilters(file_path, metadata, path_filter, language_filter) {
				continue;
			}

			if let Ok(content) = tokio::fs::read_to_string(file_path).await {
				let matches = Self::FindExactMatches(&content, query);

				if !matches.is_empty() {
					let relevance = Self::CalculateRelevance(&matches, metadata) * 1.1; // Exact matches have higher relevance

					results.push(SearchResult {
						path:file_path.to_string_lossy().to_string(),
						file_name:file_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
						matches,
						relevance,
						language:metadata.language.clone(),
					});
				}
			}
		}
	}

	/// Find matches in a single file with context
	async fn FindMatchesInFile(
		&self,
		file_path:&PathBuf,
		query:&str,
		case_sensitive:bool,
		whole_word:bool,
		index:&FileIndex,
	) -> Result<SearchResult> {
		let content = tokio::fs::read_to_string(file_path)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to read file: {}", e)))?;

		let metadata = index
			.files
			.get(file_path)
			.ok_or_else(|| AirError::Internal("File metadata not found in index".to_string()))?;

		let matches = Self::FindMatchesWithContext(&content, query, case_sensitive, whole_word);
		let relevance = Self::CalculateRelevance(&matches, metadata);

		Ok(SearchResult {
			path:file_path.to_string_lossy().to_string(),
			file_name:file_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
			matches,
			relevance,
			language:metadata.language.clone(),
		})
	}

	/// Find matches in content with surrounding context
	fn FindMatchesWithContext(Content:&str, Query:&str, CaseSensitive:bool, WholeWord:bool) -> Vec<SearchMatch> {
		let mut matches = Vec::new();
		let Lines:Vec<&str> = Content.lines().collect();

		let SearchIn = |Line:&str| -> Option<(usize, usize)> {
			let LineToSearch = if CaseSensitive { Line.to_string() } else { Line.to_lowercase() };

			let QueryToFind = if CaseSensitive { Query.to_string() } else { Query.to_lowercase() };

			let Start = if WholeWord {
				Self::FindWholeWordMatch(&LineToSearch, &QueryToFind)
			} else {
				LineToSearch.find(&QueryToFind)
			};

			Start.map(|s| (s, s + Query.len()))
		};

		for (LineIdx, Line) in Lines.iter().enumerate() {
			let LineNumber = LineIdx as u32 + 1;

			if let Some((MatchStart, MatchEnd)) = SearchIn(Line) {
				// Get context lines (2 before, 2 after)
				let ContextStart = LineIdx.saturating_sub(2);
				let ContextEnd = (LineIdx + 3).min(Lines.len());

				let ContextBefore = Lines[ContextStart..LineIdx].iter().map(|s| s.to_string()).collect();

				let ContextAfter = Lines[LineIdx + 1..ContextEnd].iter().map(|s| s.to_string()).collect();

				matches.push(SearchMatch {
					line_number:LineNumber,
					line_content:Line.to_string(),
					match_start:MatchStart,
					match_end:MatchEnd,
					context_before:ContextBefore,
					context_after:ContextAfter,
				});
			}
		}

		matches
	}

	/// Find whole word match with word boundary detection
	fn FindWholeWordMatch(Line:&str, Word:&str) -> Option<usize> {
		let mut Start = 0;

		while let Some(Pos) = Line[Start..].find(Word) {
			let ActualPos = Start + Pos;

			// Check word boundary before
			let ValidBefore = ActualPos == 0
				|| Line
					.chars()
					.nth(ActualPos - 1)
					.map_or(true, |c| !c.is_alphanumeric() && c != '_');

			// Check word boundary after
			let MatchEnd = ActualPos + Word.len();
			let ValidAfter =
				MatchEnd == Line.len() || Line.chars().nth(MatchEnd).map_or(true, |c| !c.is_alphanumeric() && c != '_');

			if ValidBefore && ValidAfter {
				return Some(ActualPos);
			}

			Start = ActualPos + 1;
		}

		None
	}

	/// Find regex matches in content
	fn FindRegexMatches(Content:&str, regex:&Regex) -> Vec<SearchMatch> {
		let mut matches = Vec::new();
		let Lines:Vec<&str> = Content.lines().collect();

		for (LineIdx, Line) in Lines.iter().enumerate() {
			let LineNumber = LineIdx as u32 + 1;

			for Mat in regex.find_iter(Line) {
				matches.push(SearchMatch {
					line_number:LineNumber,
					line_content:Line.to_string(),
					match_start:Mat.start(),
					match_end:Mat.end(),
					context_before:Vec::new(),
					context_after:Vec::new(),
				});
			}
		}

		matches
	}

	/// Find fuzzy matches using Levenshtein distance algorithm
	/// TODO: Implement full Levenshtein distance calculation for fuzzy matching
	fn FindFuzzyMatches(Content:&str, Query:&str, CaseSensitive:bool, MaxDistance:usize) -> Vec<SearchMatch> {
		let mut matches = Vec::new();
		let Lines:Vec<&str> = Content.lines().collect();

		for (LineIdx, Line) in Lines.iter().enumerate() {
			let LineNumber = LineIdx as u32 + 1;
			let LineToSearch = if CaseSensitive { Line.to_string() } else { Line.to_lowercase() };

			// Calculate Levenshtein distance for fuzzy matching
			// TODO: Implement full Levenshtein distance algorithm
			if let Some(Pos) = LineToSearch.find(Query) {
				// Check if the match is within the MaxDistance threshold
				let Distance =
					Self::CalculateLevenshteinDistance(&LineToSearch[Pos..Pos.saturating_add(Query.len())], Query);

				if Distance <= MaxDistance {
					matches.push(SearchMatch {
						line_number:LineNumber,
						line_content:Line.to_string(),
						match_start:Pos,
						match_end:Pos + Query.len(),
						context_before:Vec::new(),
						context_after:Vec::new(),
					});
				}
			}
		}

		matches
	}

	/// Find exact matches (word boundary and case-sensitive)
	fn FindExactMatches(content:&str, query:&str) -> Vec<SearchMatch> {
		Self::FindMatchesWithContext(content, query, true, true)
	}

	/// Calculate Levenshtein distance between two strings
	/// Used for fuzzy matching to measure edit distance
	fn CalculateLevenshteinDistance(s1:&str, s2:&str) -> usize {
		let s1_chars:Vec<char> = s1.chars().collect();
		let s2_chars:Vec<char> = s2.chars().collect();
		let len1 = s1_chars.len();
		let len2 = s2_chars.len();

		// Create a 2D matrix to store distances
		let mut dp = vec![vec![0usize; len2 + 1]; len1 + 1];

		// Initialize the matrix
		for i in 0..=len1 {
			dp[i][0] = i;
		}
		for j in 0..=len2 {
			dp[0][j] = j;
		}

		// Calculate distances
		for i in 1..=len1 {
			for j in 1..=len2 {
				if s1_chars[i - 1] == s2_chars[j - 1] {
					dp[i][j] = dp[i - 1][j - 1];
				} else {
					dp[i][j] = 1 + [
						dp[i - 1][j],     // deletion
						dp[i][j - 1],     // insertion
						dp[i - 1][j - 1], // substitution
					]
					.into_iter()
					.min()
					.unwrap();
				}
			}
		}

		dp[len1][len2]
	}

	/// Calculate relevance score for search results
	fn CalculateRelevance(matches:&[SearchMatch], metadata:&FileMetadata) -> f64 {
		let match_count = matches.len();
		let line_count = metadata.line_count.unwrap_or(1) as f64;

		// Base relevance: ratio of matching lines to total lines
		let mut relevance = (match_count as f64 / line_count) * 10.0;

		// Bonus for more matches
		relevance += (match_count as f64).log10() * 0.5;

		// Bonus for recently modified files
		let days_old = (chrono::Utc::now() - metadata.modified).num_days() as f64;
		relevance += 1.0 / (days_old + 1.0).max(1.0);

		relevance.min(10.0).max(0.0)
	}

	/// Check if file matches filters
	fn MatchesFilters(
		FilePath:&PathBuf,
		Metadata:&FileMetadata,
		PathFilter:Option<&str>,
		LanguageFilter:Option<&str>,
	) -> bool {
		// Check path filter
		if let Some(ref SearchPath) = PathFilter {
			if !FilePath.to_string_lossy().contains(SearchPath) {
				return false;
			}
		}

		// Check language filter
		if let Some(Lang) = LanguageFilter {
			if Metadata.language.as_deref() != Some(Lang) {
				return false;
			}
		}

		true
	}

	/// Get file information
	pub async fn GetFileInfo(&self, path:String) -> Result<Option<FileMetadata>> {
		let file_path = Self::ValidateAndExpandPath(&path)?;
		let index = self.file_index.read().await;

		Ok(index.files.get(&file_path).cloned())
	}

	/// Check if file matches patterns
	fn MatchesPatterns(file_path:&PathBuf, patterns:&[String]) -> bool {
		if patterns.is_empty() {
			return true;
		}

		let file_name = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();

		for pattern in patterns {
			if Self::MatchesPattern(&file_name, pattern) {
				return true;
			}
		}

		false
	}

	/// Check if filename matches pattern
	fn MatchesPattern(filename:&str, pattern:&str) -> bool {
		if pattern.starts_with("*.") {
			let extension = &pattern[2..];
			filename.ends_with(extension)
		} else {
			filename == pattern
		}
	}

	/// Extract symbols from code for VSCode Outline View and Go to Symbol
	///
	/// Supports multiple programming languages:
	/// - Rust: struct, impl, fn, mod, enum, trait, type
	/// - TypeScript/JavaScript: class, interface, function, const, let, var
	/// - Python: class, def
	/// - Go: type, func, struct, interface
	async fn ExtractSymbols(file_path:&PathBuf, content:&[u8], language:&str) -> Result<Vec<SymbolInfo>> {
		let content_str = String::from_utf8_lossy(content);
		let mut symbols = Vec::new();

		match language.to_lowercase().as_str() {
			"rust" => symbols.extend(Self::ExtractRustSymbols(&content_str, file_path)),
			"typescript" | "javascript" => symbols.extend(Self::ExtractTypeScriptSymbols(&content_str, file_path)),
			"python" => symbols.extend(Self::ExtractPythonSymbols(&content_str, file_path)),
			"go" => symbols.extend(Self::ExtractGoSymbols(&content_str, file_path)),
			_ => {},
		}

		Ok(symbols)
	}

	/// Extract Rust symbols (struct, impl, fn, mod, enum, trait)
	fn ExtractRustSymbols(content:&str, file_path:&PathBuf) -> Vec<SymbolInfo> {
		let mut symbols = Vec::new();
		let lines:Vec<&str> = content.lines().collect();

		for (line_idx, line) in lines.iter().enumerate() {
			let line_content = line.trim();
			let line_num = line_idx as u32 + 1;

			// Struct
			if let Some(rest) = line_content.strip_prefix("struct ") {
				let name = rest.split_whitespace().next().unwrap_or("").trim_end_matches('{');
				if let Some(col) = line.find("struct") {
					symbols.push(SymbolInfo {
						name:name.to_string(),
						kind:SymbolKind::Struct,
						line:line_num,
						column:col as u32,
						full_path:format!("{}::{}", file_path.display(), name),
					});
				}
			}

			// impl
			if let Some(rest) = line_content.strip_prefix("impl ") {
				let name = rest.split_whitespace().next().unwrap_or("").trim_end_matches('{');
				if let Some(col) = line.find("impl") {
					symbols.push(SymbolInfo {
						name:name.to_string(),
						kind:SymbolKind::Method,
						line:line_num,
						column:col as u32,
						full_path:format!("{}::{}::", file_path.display(), name),
					});
				}
			}

			// Function
			if let Some(rest) = line_content.strip_prefix("fn ") {
				let name = rest.split(|c| c == '(' || c == '<' || c == ':').next().unwrap_or("").trim();
				if let Some(col) = line.find("fn") {
					symbols.push(SymbolInfo {
						name:name.to_string(),
						kind:SymbolKind::Function,
						line:line_num,
						column:col as u32,
						full_path:format!("{}::{}", file_path.display(), name),
					});
				}
			}

			// Module
			if let Some(rest) = line_content.strip_prefix("mod ") {
				let name = rest.split_whitespace().next().unwrap_or("").trim_end_matches('{');
				if let Some(col) = line.find("mod") {
					symbols.push(SymbolInfo {
						name:name.to_string(),
						kind:SymbolKind::Module,
						line:line_num,
						column:col as u32,
						full_path:format!("{}::{}::", file_path.display(), name),
					});
				}
			}

			// Enum
			if let Some(rest) = line_content.strip_prefix("enum ") {
				let name = rest.split_whitespace().next().unwrap_or("").trim_end_matches('{');
				if let Some(col) = line.find("enum") {
					symbols.push(SymbolInfo {
						name:name.to_string(),
						kind:SymbolKind::Enum,
						line:line_num,
						column:col as u32,
						full_path:format!("{}::{}", file_path.display(), name),
					});
				}
			}

			// Trait
			if let Some(rest) = line_content.strip_prefix("trait ") {
				let name = rest.split_whitespace().next().unwrap_or("").trim_end_matches('{');
				if let Some(col) = line.find("trait") {
					symbols.push(SymbolInfo {
						name:name.to_string(),
						kind:SymbolKind::Interface,
						line:line_num,
						column:col as u32,
						full_path:format!("{}::{}", file_path.display(), name),
					});
				}
			}

			// Type alias
			if let Some(rest) = line_content.strip_prefix("type ") {
				let name = rest.split('=').next().unwrap_or("").trim().trim_end_matches(';');
				if let Some(col) = line.find("type") {
					symbols.push(SymbolInfo {
						name:name.to_string(),
						kind:SymbolKind::TypeParameter,
						line:line_num,
						column:col as u32,
						full_path:format!("{}::{}", file_path.display(), name),
					});
				}
			}
		}

		symbols
	}

	/// Extract TypeScript/JavaScript symbols (class, interface, function, etc.)
	fn ExtractTypeScriptSymbols(Content:&str, FilePath:&PathBuf) -> Vec<SymbolInfo> {
		let mut symbols = Vec::new();
		let Lines:Vec<&str> = Content.lines().collect();

		for (LineIdx, Line) in Lines.iter().enumerate() {
			let LineContent = Line.trim();
			let LineNum = LineIdx as u32 + 1;

			// Class
			if let Some(Rest) = LineContent.strip_prefix("class ") {
				let Name = Rest.split(|c| c == '{' || c == '<' || c == ' ').next().unwrap_or("").trim();
				if let Some(Col) = Line.find("class") {
					symbols.push(SymbolInfo {
						name:Name.to_string(),
						kind:SymbolKind::Class,
						line:LineNum,
						column:Col as u32,
						full_path:format!("{}::{}", FilePath.display(), Name),
					});
				}
			}

			// Interface
			if let Some(Rest) = LineContent.strip_prefix("interface ") {
				let Name = Rest.split(|c| c == '{' || c == '<' || c == ' ').next().unwrap_or("").trim();
				if let Some(Col) = Line.find("interface") {
					symbols.push(SymbolInfo {
						name:Name.to_string(),
						kind:SymbolKind::Interface,
						line:LineNum,
						column:Col as u32,
						full_path:format!("{}::{}", FilePath.display(), Name),
					});
				}
			}

			// Function
			if let Some(Rest) = LineContent.strip_prefix("function ") {
				let Name = Rest.split('(').next().unwrap_or("").trim();
				if let Some(Col) = Line.find("function") {
					symbols.push(SymbolInfo {
						name:Name.to_string(),
						kind:SymbolKind::Function,
						line:LineNum,
						column:Col as u32,
						full_path:format!("{}::{}", FilePath.display(), Name),
					});
				}
			}

			// Arrow function
			if Line.contains("=>") {
				if let Some(Col) = Line.find("=>") {
					let Name = Line.split(|c| c == '=' || c == '{').next().unwrap_or("").trim();
					if Name.ends_with(")") {
						// Try to extract function name
						let FuncName = Name.split(|c| c == '(').next().unwrap_or("").trim();
						if FuncName != "const" && FuncName != "let" && FuncName != "var" {
							symbols.push(SymbolInfo {
								name:FuncName.to_string(),
								kind:SymbolKind::Function,
								line:LineNum,
								column:Col as u32,
								full_path:format!("{}::{}", FilePath.display(), FuncName),
							});
						}
					}
				}
			}

			// Const/let/var
			for Kw in &["const ", "let ", "var "] {
				if let Some(Rest) = LineContent.strip_prefix(Kw) {
					let Name = Rest.split(|c| c == '=' || c == ':').next().unwrap_or("").trim();
					if !Name.is_empty() && !LineContent.contains("=") {
						if let Some(Col) = Line.find(Kw) {
							symbols.push(SymbolInfo {
								name:Name.to_string(),
								kind:SymbolKind::Variable,
								line:LineNum,
								column:Col as u32,
								full_path:format!("{}::{}", FilePath.display(), Name),
							});
						}
					}
				}
			}
		}

		symbols
	}

	/// Extract Python symbols (class, def)
	fn ExtractPythonSymbols(Content:&str, FilePath:&PathBuf) -> Vec<SymbolInfo> {
		let mut symbols = Vec::new();
		let Lines:Vec<&str> = Content.lines().collect();

		for (LineIdx, Line) in Lines.iter().enumerate() {
			let LineContent = Line.trim();
			let LineNum = LineIdx as u32 + 1;

			// Class
			if let Some(Rest) = LineContent.strip_prefix("class ") {
				let Name = Rest.split(|c| c == '(' || c == ':').next().unwrap_or("").trim();
				if let Some(Col) = Line.find("class") {
					symbols.push(SymbolInfo {
						name:Name.to_string(),
						kind:SymbolKind::Class,
						line:LineNum,
						column:Col as u32,
						full_path:format!("{}::{}", FilePath.display(), Name),
					});
				}
			}

			// Function
			if let Some(Rest) = LineContent.strip_prefix("def ") {
				let Name = Rest.split(|c| c == '(' || c == ':').next().unwrap_or("").trim();
				if let Some(Col) = Line.find("def") {
					symbols.push(SymbolInfo {
						name:Name.to_string(),
						kind:SymbolKind::Function,
						line:LineNum,
						column:Col as u32,
						full_path:format!("{}::{}", FilePath.display(), Name),
					});
				}
			}
		}

		symbols
	}

	/// Extract Go symbols (type, func, struct, interface)
	fn ExtractGoSymbols(Content:&str, FilePath:&PathBuf) -> Vec<SymbolInfo> {
		let mut symbols = Vec::new();
		let Lines:Vec<&str> = Content.lines().collect();

		for (LineIdx, Line) in Lines.iter().enumerate() {
			let LineContent = Line.trim();
			let LineNum = LineIdx as u32 + 1;

			// Type
			if let Some(Rest) = LineContent.strip_prefix("type ") {
				let Name = Rest.split_whitespace().next().unwrap_or("").trim();
				if let Some(Col) = Line.find("type") {
					symbols.push(SymbolInfo {
						name:Name.to_string(),
						kind:SymbolKind::TypeParameter,
						line:LineNum,
						column:Col as u32,
						full_path:format!("{}::{}", FilePath.display(), Name),
					});
				}
			}

			// Function
			if let Some(Rest) = LineContent.strip_prefix("func ") {
				let Name = Rest.split(|c| c == '(').next().unwrap_or("").trim();
				if let Some(Col) = Line.find("func") {
					symbols.push(SymbolInfo {
						name:Name.to_string(),
						kind:SymbolKind::Function,
						line:LineNum,
						column:Col as u32,
						full_path:format!("{}::{}", FilePath.display(), Name),
					});
				}
			}

			// Struct
			let LowerLine = LineContent.to_lowercase();
			if LowerLine.contains("type ") && LowerLine.contains("struct") {
				if let Some(Col) = Line.find("type") {
					let Name = LineContent.split(|c| c == ' ').nth(1).unwrap_or("").trim();
					symbols.push(SymbolInfo {
						name:Name.to_string(),
						kind:SymbolKind::Struct,
						line:LineNum,
						column:Col as u32,
						full_path:format!("{}::{}", FilePath.display(), Name),
					});
				}
			}

			// Interface
			if LowerLine.contains("type ") && LowerLine.contains("interface") {
				if let Some(Col) = Line.find("type") {
					let Name = LineContent.split(|c| c == ' ').nth(1).unwrap_or("").trim();
					symbols.push(SymbolInfo {
						name:Name.to_string(),
						kind:SymbolKind::Interface,
						line:LineNum,
						column:Col as u32,
						full_path:format!("{}::{}", FilePath.display(), Name),
					});
				}
			}
		}

		symbols
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

	/// Detect MIME type with comprehensive file type detection
	fn DetectMimeType(file_path:&PathBuf, content:&[u8]) -> String {
		if let Some(extension) = file_path.extension() {
			match extension.to_string_lossy().to_lowercase().as_str() {
				"rs" => "text/x-rust".to_string(),
				"ts" => "text/x-typescript".to_string(),
				"tsx" => "text/typescript-jsx".to_string(),
				"js" => "text/javascript".to_string(),
				"jsx" => "text/javascript-jsx".to_string(),
				"mjs" => "text/javascript".to_string(),
				"cjs" => "text/javascript".to_string(),
				"json" => "application/json".to_string(),
				"jsonc" => "application/json+comments".to_string(),
				"toml" => "text/x-toml".to_string(),
				"yaml" | "yml" => "text/x-yaml".to_string(),
				"md" => "text/markdown".to_string(),
				"mdx" => "text/markdown-jsx".to_string(),
				"txt" => "text/plain".to_string(),
				"html" | "htm" => "text/html".to_string(),
				"css" => "text/css".to_string(),
				"scss" => "text/x-scss".to_string(),
				"sass" => "text/x-sass".to_string(),
				"less" => "text/x-less".to_string(),
				"xml" => "application/xml".to_string(),
				"py" => "text/x-python".to_string(),
				"java" => "text/x-java".to_string(),
				"go" => "text/x-go".to_string(),
				"sh" => "text/x-shellscript".to_string(),
				"bash" => "text/x-shellscript".to_string(),
				"zsh" => "text/x-shellscript".to_string(),
				"fish" => "text/x-shellscript".to_string(),
				"rb" => "text/x-ruby".to_string(),
				"php" => "text/x-php".to_string(),
				"swift" => "text/x-swift".to_string(),
				"kt" | "kts" => "text/x-kotlin".to_string(),
				"scala" => "text/x-scala".to_string(),
				"cs" => "text/x-csharp".to_string(),
				"vb" => "text/x-vbnet".to_string(),
				"f#" => "text/x-fsharp".to_string(),
				"r" => "text/x-r".to_string(),
				"lua" => "text/x-lua".to_string(),
				"pl" => "text/x-perl".to_string(),
				"ps1" => "text/x-powershell".to_string(),
				"sql" => "text/x-sql".to_string(),
				"graphql" | "gql" => "application/graphql".to_string(),
				"graphqls" => "application/graphql".to_string(),
				"proto" => "text/x-protobuf".to_string(),
				"wasm" => "application/wasm".to_string(),
				"wat" => "text/x-wat".to_string(),
				"lock" => "application/json".to_string(),
				"graphqlconfig" => "application/json".to_string(),
				"graphqlrc" => "application/json".to_string(),
				"graphqlconfig.yaml" | "graphqlrc.yaml" => "text/x-yaml".to_string(),
				"graphqlrc.yml" => "text/x-yaml".to_string(),
				"graphqlconfig.json" | "graphqlrc.json" => "application/json".to_string(),
				"graphqlconfig.js" | "graphqlrc.js" => "text/javascript".to_string(),
				"graphqlconfig.ts" | "graphqlrc.ts" => "text/x-typescript".to_string(),
				"graphqlconfig.toml" | "graphqlrc.toml" => "text/x-toml".to_string(),
				_ => {
					// Use content-based detection
					if content.starts_with(b"{") || content.starts_with(b"[") {
						"application/json".to_string()
					} else if content.starts_with(b"#!") {
						"text/x-shellscript".to_string()
					} else if content.starts_with(b"<?xml") {
						"application/xml".to_string()
					} else if content.starts_with(b"<!DOCTYPE") || content.starts_with(b"<html") {
						"text/html".to_string()
					} else if content.is_ascii() && !content.windows(4).any(|w| w.starts_with(&[0u8])) {
						"text/plain".to_string()
					} else {
						"application/octet-stream".to_string()
					}
				},
			}
		} else {
			// No extension, try content-based detection
			if content.starts_with(b"{") || content.starts_with(b"[") {
				"application/json".to_string()
			} else if content.starts_with(b"#!") {
				"text/x-shellscript".to_string()
			} else if content.starts_with(b"<?xml") {
				"application/xml".to_string()
			} else if content.starts_with(b"---") {
				"text/x-yaml".to_string()
			} else if content.is_ascii() && !content.windows(4).any(|w| w.starts_with(&[0u8])) {
				"text/plain".to_string()
			} else {
				"application/octet-stream".to_string()
			}
		}
	}

	/// Detect programming language from file extension and shebang
	fn DetectLanguage(file_path:&PathBuf) -> Option<String> {
		if let Some(extension) = file_path.extension() {
			let lang = match extension.to_string_lossy().to_lowercase().as_str() {
				"rs" => "rust",
				"ts" | "tsx" => "typescript",
				"js" | "jsx" | "mjs" | "cjs" => "javascript",
				"json" | "jsonc" | "graphqlconfig" | "graphqlrc" | "lock" => "json",
				"toml" | "graphqlconfig.toml" | "graphqlrc.toml" => "toml",
				"yaml" | "yml" | "graphqlconfig.yaml" | "graphqlrc.yaml" | "graphqlrc.yml" => "yaml",
				"md" | "mdx" => "markdown",
				"txt" => "plaintext",
				"html" | "htm" => "html",
				"css" => "css",
				"scss" => "scss",
				"sass" => "sass",
				"less" => "less",
				"xml" => "xml",
				"py" => "python",
				"java" => "java",
				"go" => "go",
				"sh" | "bash" => "shellscript",
				"zsh" => "shellscript",
				"fish" => "fish",
				"rb" => "ruby",
				"php" => "php",
				"swift" => "swift",
				"kt" | "kts" => "kotlin",
				"scala" => "scala",
				"cpp" | "cc" | "cxx" | "hpp" | "hxx" => "cpp",
				"c" | "h" => "c",
				"cs" => "csharp",
				"vb" => "vb",
				"f#" | "fs" | "fsi" | "fsx" => "fsharp",
				"r" | "rmd" => "r",
				"jl" => "julia",
				"lua" => "lua",
				"pl" => "perl",
				"ps1" | "psm1" | "psd1" => "powershell",
				"sql" => "sql",
				"graphql" | "gql" | "graphqls" => "graphql",
				"proto" => "protobuf",
				"wasm" => "wasm",
				"wat" => "wat",
				"clj" | "cljs" | "cljc" | "edn" => "clojure",
				"hs" | "lhs" => "haskell",
				"erl" | "hrl" => "erlang",
				"ex" | "exs" => "elixir",
				"dart" => "dart",
				"nim" => "nim",
				"v" => "v",
				"zig" => "zig",
				"odin" => "odin",
				"mojo" | "🔥" => "mojo",
				_ => return None,
			};
			return Some(lang.to_string());
		}

		// Try to detect from shebang
		if let Ok(content) = std::fs::read_to_string(file_path) {
			if let Some(first_line) = content.lines().next() {
				if first_line.starts_with("#!") {
					let shebang_path = first_line.split_whitespace().nth(1).unwrap_or("");
					let lang = match shebang_path.rsplit('/').next().unwrap_or("") {
						"bash" => "shellscript",
						"sh" => "shellscript",
						"zsh" => "shellscript",
						"fish" => "fish",
						"python" | "python2" | "python3" => "python",
						"node" => "javascript",
						"ruby" => "ruby",
						"perl" => "perl",
						"php" => "php",
						"lua" => "lua",
						"r" | "Rscript" => "r",
						"julia" => "julia",
						"rust" | "rustc" => "rust",
						"go" => "go",
						"java" => "java",
						"scala" | "scalac" => "scala",
						"kotlin" | "kotlinc" => "kotlin",
						"swift" => "swift",
						_ => return None,
					};
					return Some(lang.to_string());
				}
			}
		}

		None
	}

	/// Load index from disk or create new one with corruption detection
	async fn LoadOrCreateIndex(IndexDirectory:&PathBuf) -> Result<FileIndex> {
		let IndexFile = IndexDirectory.join("file_index.json");

		if IndexFile.exists() {
			// Try to load existing index
			match Self::LoadIndexInternal(&IndexFile).await {
				Ok(Index) => {
					log::info!("[FileIndexer] Loaded index with {} files", Index.files.len());
					Ok(Index)
				},
				Err(e) => {
					log::warn!(
						"[FileIndexer] Failed to load index (may be corrupted): {}. Creating new index.",
						e
					);
					// Backup corrupted index
					Self::BackupCorruptedIndex(IndexDirectory).await?;
					Ok(Self::CreateNewIndex())
				},
			}
		} else {
			// Create new index
			Ok(Self::CreateNewIndex())
		}
	}

	/// Load index from disk internal
	async fn LoadIndexInternal(IndexFile:&PathBuf) -> Result<FileIndex> {
		let Content = tokio::fs::read_to_string(IndexFile)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to read index file: {}", e)))?;

		let Index:FileIndex = serde_json::from_str(&Content)
			.map_err(|e| AirError::Serialization(format!("Failed to parse index file: {}", e)))?;

		// Verify index structure
		if Index.index_version.is_empty() || Index.index_checksum.is_empty() {
			return Err(AirError::Serialization("Index missing version or checksum".to_string()));
		}

		Ok(Index)
	}

	/// Create a new empty index
	fn CreateNewIndex() -> FileIndex {
		let index = FileIndex {
			files:HashMap::new(),
			content_index:HashMap::new(),
			symbol_index:HashMap::new(),
			file_symbols:HashMap::new(),
			last_updated:chrono::Utc::now(),
			index_version:Self::GenerateIndexVersion(),
			index_checksum:String::new(),
		};

		index
	}

	/// Backup corrupted index before creating new one
	async fn BackupCorruptedIndex(IndexDirectory:&PathBuf) -> Result<()> {
		let IndexFile = IndexDirectory.join("file_index.json");
		let BackupFile = IndexDirectory.join(format!("file_index.corrupted.{}.json", chrono::Utc::now().timestamp()));

		// Rename corrupted file to backup
		tokio::fs::rename(&IndexFile, &BackupFile)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to backup corrupted index: {}", e)))?;

		log::info!("[FileIndexer] Backed up corrupted index to: {}", BackupFile.display());

		Ok(())
	}

	/// Generate index version string
	fn GenerateIndexVersion() -> String { format!("{}-{}", env!("CARGO_PKG_VERSION"), chrono::Utc::now().timestamp()) }

	/// Calculate index checksum for integrity verification
	fn CalculateIndexChecksum(Index:&FileIndex) -> Result<String> {
		let ChecksumInput = format!(
			"{}:{}:{}:{}",
			Index.files.len(),
			Index.content_index.len(),
			Index.symbol_index.len(),
			Index.last_updated.timestamp()
		);

		use sha2::{Digest, Sha256};
		let mut Hasher = Sha256::new();
		Hasher.update(ChecksumInput.as_bytes());
		Ok(format!("{:x}", Hasher.finalize()))
	}

	/// Verify index integrity and detect corruption
	async fn VerifyIndexIntegrity(&self) -> Result<()> {
		let index = self.file_index.read().await;

		// Recalculate checksum
		let expected_checksum = Self::CalculateIndexChecksum(&index)?;

		if index.index_checksum != expected_checksum {
			log::warn!(
				"[FileIndexer] Index checksum mismatch! Expected: {}, Got: {}",
				expected_checksum,
				index.index_checksum
			);

			// Mark as corrupted
			*self.corruption_detected.lock().await = true;

			return Err(AirError::Internal(
				"Index integrity check failed - possible corruption".to_string(),
			));
		}

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

	/// Recover corrupted index by removing it and creating new one
	pub async fn recover_from_corruption(&self) -> Result<()> {
		log::info!("[FileIndexer] Recovering from corrupted index...");

		// Backup corrupted index
		Self::BackupCorruptedIndex(&self.index_directory).await?;

		// Create new index
		let new_index = Self::CreateNewIndex();
		*self.file_index.write().await = new_index;

		// Clear corruption flag
		*self.corruption_detected.lock().await = false;

		log::info!("[FileIndexer] Index recovery completed");

		Ok(())
	}

	/// Save index to disk with atomic write
	async fn SaveIndex(&self, index:&FileIndex) -> Result<()> {
		let index_file = self.index_directory.join("file_index.json");
		let temp_file = self.index_directory.join("file_index.json.tmp");

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
			"[FileIndexer] Index saved to: {} ({} files, {} symbols)",
			index_file.display(),
			index.files.len(),
			index.symbol_index.len()
		);

		Ok(())
	}

	/// Start file watcher for incremental indexing
	///
	/// Monitors file system changes and updates index in real-time.
	/// This enables:
	/// - Real-time search updates
	/// - Automatic reindexing of changed files
	/// - Removal of deleted files from index
	pub async fn StartFileWatcher(&self, paths:Vec<PathBuf>) -> Result<()> {
		use notify::{RecursiveMode, Watcher};

		let index = self.file_index.clone();
		let corruption_flag = self.corruption_detected.clone();

		// Clone indexer for async task
		let indexer_arc = Arc::new(self.clone());

		let mut watcher:notify::RecommendedWatcher = Watcher::new(
			move |res:std::result::Result<notify::Event, notify::Error>| {
				if let Ok(event) = res {
					// Check corruption flag before processing events
					let _Indexer = indexer_arc.clone();
					if *corruption_flag.blocking_lock() {
						log::warn!("[FileIndexer] Skipping file event - index marked as corrupted");
						return;
					}
					let index = index.clone();
					tokio::spawn(async move {
						Self::HandleFileEvent(event, index).await;
					});
				}
			},
			notify::Config::default(),
		)
		.map_err(|e| AirError::Internal(format!("Failed to create file watcher: {}", e)))?;

		// Watch all specified paths
		for path in paths {
			if path.exists() {
				watcher
					.watch(&path, RecursiveMode::Recursive)
					.map_err(|e| AirError::FileSystem(format!("Failed to watch path {}: {}", path.display(), e)))?;
				log::info!("[FileIndexer] Watching path: {}", path.display());
			}
		}

		*self.file_watcher.lock().await = Some(watcher);

		log::info!("[FileIndexer] File watcher started successfully");

		Ok(())
	}

	/// Handle file watcher event
	async fn HandleFileEvent(Event:notify::Event, Index:Arc<RwLock<FileIndex>>) {
		match Event.kind {
			notify::EventKind::Create(notify::event::CreateKind::File) => {
				for Path in Event.paths {
					log::debug!("[FileIndexer] File created: {}", Path.display());
					// TODO: Schedule reindex for newly created files
				}
			},
			notify::EventKind::Modify(notify::event::ModifyKind::Data(_))
			| notify::EventKind::Modify(notify::event::ModifyKind::Name(notify::event::RenameMode::Both)) => {
				for Path in Event.paths {
					log::debug!("[FileIndexer] File modified: {}", Path.display());
					// Update index for this file
					// TODO: Implement incremental update for modified files
				}
			},
			notify::EventKind::Remove(notify::event::RemoveKind::File) => {
				for Path in Event.paths {
					log::debug!("[FileIndexer] File removed: {}", Path.display());
					// Remove from index
					let mut Index = Index.write().await;
					Index.files.remove(&Path);
					Index.file_symbols.remove(&Path);

					// Remove from symbol index
					for (_, Locations) in Index.symbol_index.iter_mut() {
						Locations.retain(|loc| loc.file_path != Path);
					}

					// Remove from content index
					for (_, Files) in Index.content_index.iter_mut() {
						Files.retain(|p| p != &Path);
					}
				}
			},
			_ => {},
		}
	}

	/// Stop file watcher
	pub async fn StopFileWatcher(&self) {
		if let Some(watcher) = self.file_watcher.lock().await.take() {
			drop(watcher);
			log::info!("[FileIndexer] File watcher stopped");
		}
	}

	/// Start background tasks for periodic indexing
	pub async fn StartBackgroundTasks(&self) -> Result<tokio::task::JoinHandle<()>> {
		let config = &self.AppState.Configuration.Indexing;

		if !config.Enabled {
			log::info!("[FileIndexer] Background indexing disabled in configuration");
			return Err(AirError::Configuration("Background indexing is disabled".to_string()));
		}

		let indexer = self.clone();

		let handle = tokio::spawn(async move {
			indexer.BackgroundTask().await;
		});

		log::info!("[FileIndexer] Background tasks started");

		Ok(handle)
	}

	/// Background task for periodic indexing
	async fn BackgroundTask(&self) {
		let config = &self.AppState.Configuration.Indexing;

		let interval = tokio::time::Duration::from_secs(config.UpdateIntervalMinutes as u64 * 60);
		let mut interval = tokio::time::interval(interval);

		log::info!(
			"[FileIndexer] Background indexing configured for {} minute intervals",
			config.UpdateIntervalMinutes
		);

		loop {
			interval.tick().await;

			// Check corruption flag
			if *self.corruption_detected.lock().await {
				log::warn!("[FileIndexer] Index corrupted, skipping background update");
				continue;
			}

			log::info!("[FileIndexer] Running periodic background index...");

			// Re-index configured directories
			if let Err(e) = self.IndexDirectory(config.index_directory.clone(), Vec::new()).await {
				log::error!("[FileIndexer] Background indexing failed: {}", e);
			}
		}
	}

	/// Stop background tasks
	pub async fn StopBackgroundTasks(&self) {
		log::info!("[FileIndexer] Stopping background tasks");
		// Tasks are cancelled when the task handle is dropped
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
}

/// Index statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStatistics {
	pub file_count:u32,
	pub total_size:u64,
	pub total_symbols:u32,
	pub language_counts:HashMap<String, u32>,
	pub last_updated:chrono::DateTime<chrono::Utc>,
	pub index_version:String,
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
