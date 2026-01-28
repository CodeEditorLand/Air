//! # File Indexing Service
//!
//! Handles background file indexing and search operations for the Land ecosystem.
//! Provides fast file search, content indexing, and metadata extraction.

use std::{collections::HashMap, path::PathBuf, sync::Arc};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

use crate::{ApplicationState::ApplicationState, Result, AirError, Configuration::ConfigurationManager};

/// File indexer implementation
pub struct FileIndexer {
    /// Application state
    app_state: Arc<ApplicationState>,
    
    /// File index
    file_index: Arc<RwLock<FileIndex>>,
    
    /// Index storage directory
    index_directory: PathBuf,
    
    /// File watcher
    file_watcher: Arc<Mutex<Option<notify::RecommendedWatcher>>>,
}

/// File index structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileIndex {
    /// Indexed files with metadata
    files: HashMap<PathBuf, FileMetadata>,
    
    /// Content index for search
    content_index: HashMap<String, Vec<PathBuf>>,
    
    /// Last update timestamp
    last_updated: chrono::DateTime<chrono::Utc>,
}

/// File metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    pub modified: chrono::DateTime<chrono::Utc>,
    pub mime_type: String,
    pub language: Option<String>,
    pub line_count: Option<u32>,
    pub checksum: String,
}

/// Indexing result
#[derive(Debug, Clone)]
pub struct IndexResult {
    pub files_indexed: u32,
    pub total_size: u64,
    pub duration_seconds: f64,
}

/// Search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: String,
    pub matches: Vec<SearchMatch>,
}

/// Search match
#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub line_number: u32,
    pub line_content: String,
    pub match_start: usize,
    pub match_end: usize,
}

impl FileIndexer {
    /// Create a new file indexer
    pub async fn new(app_state: Arc<ApplicationState>) -> Result<Self> {
        let config = &app_state.configuration.indexing;
        
        // Expand index directory path
        let index_directory = ConfigurationManager::expand_path(&config.index_directory)?;
        
        // Create index directory if it doesn't exist
        tokio::fs::create_dir_all(&index_directory).await
            .map_err(|e| AirError::Configuration(format!("Failed to create index directory: {}", e)))?;
        
        // Load or create index
        let file_index = Self::load_index(&index_directory).await?;
        
        let indexer = Self {
            app_state,
            file_index: Arc::new(RwLock::new(file_index)),
            index_directory,
            file_watcher: Arc::new(Mutex::new(None)),
        };
        
        // Initialize service status
        indexer.app_state.update_service_status("indexing", crate::ApplicationState::ServiceStatus::Running)
            .await
            .map_err(|e| AirError::Internal(e.to_string()))?;
        
        Ok(indexer)
    }
    
    /// Index a directory
    pub async fn index_directory(&self, path: String, patterns: Vec<String>) -> Result<IndexResult> {
        let start_time = std::time::Instant::now();
        
        log::info!("[FileIndexer] Starting directory index: {}", path);
        
        let directory_path = expand_path(&path)?;
        
        if !directory_path.exists() {
            return Err(AirError::FileSystem(format!("Directory does not exist: {}", path)));
        }
        
        if !directory_path.is_dir() {
            return Err(AirError::FileSystem(format!("Path is not a directory: {}", path)));
        }
        
        let config = &self.app_state.configuration.indexing;
        let mut files_indexed = 0;
        let mut total_size = 0;
        
        // Build file patterns
        let include_patterns = if patterns.is_empty() {
            config.file_types.clone()
        } else {
            patterns
        };
        
        // Walk directory
        let walker = ignore::WalkBuilder::new(&directory_path)
            .max_depth(Some(10)) // Limit depth to prevent infinite recursion
            .build();
        
        let mut index = self.file_index.write().await;
        
        for result in walker {
            match result {
                Ok(entry) => {
                    if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                        let file_path = entry.path().to_path_buf();
                        
                        // Check file size limit
                        if let Ok(metadata) = entry.metadata() {
                            let file_size = metadata.len();
                            
                            if file_size > config.max_file_size_mb as u64 * 1024 * 1024 {
                                continue; // Skip files that are too large
                            }
                            
                            // Check file pattern
                            if Self::matches_patterns(&file_path, &include_patterns) {
                                match self.index_file(&file_path).await {
                                    Ok(metadata) => {
                                        index.files.insert(file_path.clone(), metadata.clone());
                                        
                                        // Index content for search
                                        self.index_content(&mut index, &file_path, &metadata).await?;
                                        
                                        files_indexed += 1;
                                        total_size += file_size;
                                    },
                                    Err(e) => {
                                        log::warn!("[FileIndexer] Failed to index file {}: {}", file_path.display(), e);
                                    }
                                }
                            }
                        }
                    }
                },
                Err(e) => {
                    log::warn!("[FileIndexer] Error walking directory: {}", e);
                }
            }
        }
        
        index.last_updated = chrono::Utc::now();
        
        // Save index
        self.save_index(&index).await?;
        
        let duration = start_time.elapsed().as_secs_f64();
        
        log::info!("[FileIndexer] Indexing completed: {} files, {} bytes in {:.2}s", files_indexed, total_size, duration);
        
        Ok(IndexResult {
            files_indexed,
            total_size,
            duration_seconds: duration,
        })
    }
    
    /// Index a single file
    async fn index_file(&self, file_path: &PathBuf) -> Result<FileMetadata> {
        let metadata = std::fs::metadata(file_path)
            .map_err(|e| AirError::FileSystem(format!("Failed to get file metadata: {}", e)))?;
        
        let modified = metadata.modified()
            .map_err(|e| AirError::FileSystem(format!("Failed to get modification time: {}", e)))?;
        
        let modified_time = chrono::DateTime::<chrono::Utc>::from(modified);
        
        // Calculate checksum
        let content = tokio::fs::read(file_path).await
            .map_err(|e| AirError::FileSystem(format!("Failed to read file: {}", e)))?;
        
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let checksum = format!("{:x}", hasher.finalize());
        
        // Detect MIME type
        let mime_type = Self::detect_mime_type(file_path, &content);
        
        // Detect programming language
        let language = Self::detect_language(file_path);
        
        // Count lines for text files
        let line_count = if mime_type.starts_with("text/") {
            Some(content.iter().filter(|&&b| b == b'\n').count() as u32 + 1)
        } else {
            None
        };
        
        Ok(FileMetadata {
            path: file_path.clone(),
            size: metadata.len(),
            modified: modified_time,
            mime_type,
            language,
            line_count,
            checksum,
        })
    }
    
    /// Index file content for search
    async fn index_content(&self, index: &mut FileIndex, file_path: &PathBuf, metadata: &FileMetadata) -> Result<()> {
        if !metadata.mime_type.starts_with("text/") {
            return Ok(()); // Only index text files
        }
        
        let content = tokio::fs::read_to_string(file_path).await
            .map_err(|e| AirError::FileSystem(format!("Failed to read file content: {}", e)))?;
        
        // Simple word-based indexing
        for word in content.split_whitespace() {
            let word = word.to_lowercase();
            
            if word.len() > 2 { // Only index words longer than 2 characters
                index.content_index
                    .entry(word)
                    .or_insert_with(Vec::new)
                    .push(file_path.clone());
            }
        }
        
        Ok(())
    }
    
    /// Search files
    pub async fn search_files(&self, query: String, path: Option<String>, max_results: u32) -> Result<Vec<SearchResult>> {
        log::info!("[FileIndexer] Searching for: '{}'", query);
        
        let index = self.file_index.read().await;
        let query = query.to_lowercase();
        let mut results = Vec::new();
        
        // Search in content index
        if let Some(file_paths) = index.content_index.get(&query) {
            for file_path in file_paths.iter().take(max_results as usize) {
                if let Some(metadata) = index.files.get(file_path) {
                    // Check path filter
                    if let Some(ref search_path) = path {
                        if !file_path.to_string_lossy().contains(search_path) {
                            continue;
                        }
                    }
                    
                    // Read file and find matches
                    if let Ok(content) = tokio::fs::read_to_string(file_path).await {
                        let matches = Self::find_matches_in_content(&content, &query);
                        
                        if !matches.is_empty() {
                            results.push(SearchResult {
                                path: file_path.to_string_lossy().to_string(),
                                matches,
                            });
                        }
                    }
                }
            }
        }
        
        // Also search in file names
        for (file_path, metadata) in &index.files {
            if results.len() >= max_results as usize {
                break;
            }
            
            // Check path filter
            if let Some(ref search_path) = path {
                if !file_path.to_string_lossy().contains(search_path) {
                    continue;
                }
            }
            
            let file_name = file_path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            
            if file_name.contains(&query) {
                results.push(SearchResult {
                    path: file_path.to_string_lossy().to_string(),
                    matches: Vec::new(), // No content matches, just filename match
                });
            }
        }
        
        log::info!("[FileIndexer] Search completed: {} results", results.len());
        
        Ok(results)
    }
    
    /// Find matches in file content
    fn find_matches_in_content(content: &str, query: &str) -> Vec<SearchMatch> {
        let mut matches = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        for (line_idx, line) in lines.iter().enumerate() {
            let line_number = line_idx as u32 + 1;
            let line_lower = line.to_lowercase();
            
            if let Some(match_start) = line_lower.find(query) {
                let match_end = match_start + query.len();
                
                matches.push(SearchMatch {
                    line_number,
                    line_content: line.to_string(),
                    match_start,
                    match_end,
                });
            }
        }
        
        matches
    }
    
    /// Get file information
    pub async fn get_file_info(&self, path: String) -> Result<Option<FileMetadata>> {
        let file_path = expand_path(&path)?;
        let index = self.file_index.read().await;
        
        Ok(index.files.get(&file_path).cloned())
    }
    
    /// Check if file matches patterns
    fn matches_patterns(file_path: &PathBuf, patterns: &[String]) -> bool {
        if patterns.is_empty() {
            return true;
        }
        
        let file_name = file_path.file_name()
            .unwrap_or_default()
            .to_string_lossy();
        
        for pattern in patterns {
            if Self::matches_pattern(&file_name, pattern) {
                return true;
            }
        }
        
        false
    }
    
    /// Check if filename matches pattern
    fn matches_pattern(filename: &str, pattern: &str) -> bool {
        if pattern.starts_with("*.") {
            let extension = &pattern[2..];
            filename.ends_with(extension)
        } else {
            filename == pattern
        }
    }
    
    /// Detect MIME type
    fn detect_mime_type(file_path: &PathBuf, content: &[u8]) -> String {
        if let Some(extension) = file_path.extension() {
            match extension.to_string_lossy().to_lowercase().as_str() {
                "rs" => "text/x-rust".to_string(),
                "ts" => "text/x-typescript".to_string(),
                "js" => "text/javascript".to_string(),
                "json" => "application/json".to_string(),
                "toml" => "text/x-toml".to_string(),
                "md" => "text/markdown".to_string(),
                "txt" => "text/plain".to_string(),
                "html" => "text/html".to_string(),
                "css" => "text/css".to_string(),
                "xml" => "application/xml".to_string(),
                _ => {
                    // Use content-based detection
                    if content.starts_with(b"{") || content.starts_with(b"[") {
                        "application/json".to_string()
                    } else if content.starts_with(b"#!") {
                        "text/x-shellscript".to_string()
                    } else {
                        "application/octet-stream".to_string()
                    }
                }
            }
        } else {
            "application/octet-stream".to_string()
        }
    }
    
    /// Detect programming language
    fn detect_language(file_path: &PathBuf) -> Option<String> {
        if let Some(extension) = file_path.extension() {
            match extension.to_string_lossy().to_lowercase().as_str() {
                "rs" => Some("rust".to_string()),
                "ts" => Some("typescript".to_string()),
                "js" => Some("javascript".to_string()),
                "json" => Some("json".to_string()),
                "toml" => Some("toml".to_string()),
                "md" => Some("markdown".to_string()),
                "py" => Some("python".to_string()),
                "java" => Some("java".to_string()),
                "cpp" | "cc" | "cxx" => Some("cpp".to_string()),
                "c" => Some("c".to_string()),
                "go" => Some("go".to_string()),
                "rb" => Some("ruby".to_string()),
                "php" => Some("php".to_string()),
                _ => None,
            }
        } else {
            None
        }
    }
    
    /// Load index from disk
    async fn load_index(index_directory: &PathBuf) -> Result<FileIndex> {
        let index_file = index_directory.join("file_index.json");
        
        if index_file.exists() {
            let content = tokio::fs::read_to_string(&index_file).await
                .map_err(|e| AirError::FileSystem(format!("Failed to read index file: {}", e)))?;
            
            serde_json::from_str(&content)
                .map_err(|e| AirError::Serialization(format!("Failed to parse index file: {}", e)))
        } else {
            Ok(FileIndex {
                files: HashMap::new(),
                content_index: HashMap::new(),
                last_updated: chrono::Utc::now(),
            })
        }
    }
    
    /// Save index to disk
    async fn save_index(&self, index: &FileIndex) -> Result<()> {
        let index_file = self.index_directory.join("file_index.json");
        
        let content = serde_json::to_string_pretty(index)
            .map_err(|e| AirError::Serialization(format!("Failed to serialize index: {}", e)))?;
        
        tokio::fs::write(&index_file, content).await
            .map_err(|e| AirError::FileSystem(format!("Failed to write index file: {}", e)))?;
        
        Ok(())
    }
    
    /// Start background tasks
    pub async fn start_background_tasks(&self) -> Result<tokio::task::JoinHandle<()>> {
        let indexer = self.clone();
        
        let handle = tokio::spawn(async move {
            indexer.background_task().await;
        });
        
        Ok(handle)
    }
    
    /// Background task for periodic indexing
    async fn background_task(&self) {
        let config = &self.app_state.configuration.indexing;
        
        if !config.enabled {
            return;
        }
        
        let interval = tokio::time::Duration::from_secs(config.update_interval_minutes as u64 * 60);
        let mut interval = tokio::time::interval(interval);
        
        loop {
            interval.tick().await;
            
            // Re-index common directories
            let common_dirs = [
                dirs::home_dir().map(|p| p.to_string_lossy().to_string()),
                dirs::document_dir().map(|p| p.to_string_lossy().to_string()),
                dirs::download_dir().map(|p| p.to_string_lossy().to_string()),
            ];
            
            for dir in common_dirs.into_iter().flatten() {
                if let Err(e) = self.index_directory(dir.clone(), Vec::new()).await {
                    log::warn!("[FileIndexer] Background indexing failed for {}: {}", dir, e);
                }
            }
        }
    }
    
    /// Stop background tasks
    pub async fn stop_background_tasks(&self) {
        log::info!("[FileIndexer] Stopping background tasks");
    }
}

impl Clone for FileIndexer {
    fn clone(&self) -> Self {
        Self {
            app_state: self.app_state.clone(),
            file_index: self.file_index.clone(),
            index_directory: self.index_directory.clone(),
            file_watcher: self.file_watcher.clone(),
        }
    }
}
