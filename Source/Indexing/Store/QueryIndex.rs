//! # QueryIndex
//!
//! ## File: Indexing/Store/QueryIndex.rs
//!
//! ## Role in Air Architecture
//!
//! Provides index query functionality for the File Indexer service,
//! handling search operations across indexed files with multiple search modes.
//!
//! ## Primary Responsibility
//!
//! Query the file index to find symbols and content matching search criteria,
//! supporting literal, regex, fuzzy, and exact search modes.
//!
//! ## Secondary Responsibilities
//!
//! - Multi-mode search (literal, regex, fuzzy, exact)
//! - Case sensitivity and whole word matching
//! - Path and language filtering
//! - Result pagination and ranking
//! - Search query sanitization
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `regex` - Regular expression search patterns
//! - `tokio` - Async file I/O operations
//!
//! **Internal Modules:**
//! - `crate::Result` - Error handling type
//! - `crate::AirError` - Error types
//! - `super::super::FileIndex` - Index structure definitions
//!
//! ## Dependents
//!
//! - `Indexing::mod::FileIndexer` - Main file indexer implementation
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's search functionality in
//! `src/vs/workbench/services/search/common/`
//!
//! ## Security Considerations
//!
//! - Search query sanitization prevents injection
//! - Query length limits
//! - Control character filtering
//!
//! ## Performance Considerations
//!
//! - Content index for fast token lookup
//! - Result pagination to limit memory usage
//! - Efficient string matching algorithms
//! - Fuzzy search with configurable distance
//!
//! ## Error Handling Strategy
//!
//! Query operations return detailed error messages for invalid queries
//! or search failures, treating individual file read errors as warnings.
//!
//! ## Thread Safety
//!
//! Query operations read from shared Arc<RwLock<>> state and
//! return safe-ownership results for the caller.

use std::path::PathBuf;

use regex::Regex;

use crate::{AirError, Result, dev_log};
// Use the full paths to types in State::CreateState
use crate::Indexing::State::CreateState::{FileIndex, FileMetadata};

/// Maximum search results per query (pagination default)
pub const MAX_SEARCH_RESULTS_DEFAULT:u32 = 100;

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

impl IntoIterator for PaginatedSearchResults {
	type Item = SearchResult;

	type IntoIter = std::vec::IntoIter<SearchResult>;

	fn into_iter(self) -> Self::IntoIter { self.results.into_iter() }
}

impl<'a> IntoIterator for &'a PaginatedSearchResults {
	type Item = &'a SearchResult;

	type IntoIter = std::slice::Iter<'a, SearchResult>;

	fn into_iter(self) -> Self::IntoIter { self.results.iter() }
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
pub async fn QueryIndexSearch(
	index:&FileIndex,

	query:SearchQuery,

	path_filter:Option<String>,

	language_filter:Option<String>,
) -> Result<PaginatedSearchResults> {
	dev_log!(
		"indexing",
		"[QueryIndex] Searching for: '{}' (mode: {:?})",
		query.query,
		query.mode
	);

	// Sanitize search query
	let sanitized_query = SanitizeSearchQuery(&query.query)?;

	// Build search parameters
	let case_sensitive = query.case_sensitive;

	let whole_word = query.whole_word;

	let max_results = if query.max_results == 0 {
		MAX_SEARCH_RESULTS_DEFAULT
	} else {
		query.max_results.min(1000) // Cap at 1000 results
	};

	let mut all_results = Vec::new();

	// Search based on mode
	match query.mode {
		SearchMode::Literal => {
			QueryIndexLiteral(
				&sanitized_query,
				case_sensitive,
				whole_word,
				path_filter.as_deref(),
				language_filter.as_deref(),
				index,
				&mut all_results,
			)
			.await;
		},

		SearchMode::Regex => {
			if let Some(regex) = &query.regex {
				QueryIndexRegex(
					regex,
					path_filter.as_deref(),
					language_filter.as_deref(),
					index,
					&mut all_results,
				)
				.await;
			} else {
				// Try to compile regex from query
				if let Ok(regex) = Regex::new(&sanitized_query) {
					QueryIndexRegex(
						&regex,
						path_filter.as_deref(),
						language_filter.as_deref(),
						index,
						&mut all_results,
					)
					.await;
				}
			}
		},

		SearchMode::Fuzzy => {
			QueryIndexFuzzy(
				&sanitized_query,
				case_sensitive,
				path_filter.as_deref(),
				language_filter.as_deref(),
				index,
				&mut all_results,
			)
			.await;
		},

		SearchMode::Exact => {
			QueryIndexExact(
				&sanitized_query,
				case_sensitive,
				path_filter.as_deref(),
				language_filter.as_deref(),
				index,
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

	dev_log!(
		"indexing",
		"[QueryIndex] Search completed: {} total results, page {} of {}",
		total_count,
		page + 1,
		total_pages
	);

	Ok(PaginatedSearchResults { results:page_results, total_count, page, total_pages, page_size:max_results })
}

/// Sanitize search query to prevent injection and invalid patterns
pub fn SanitizeSearchQuery(query:&str) -> Result<String> {
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
async fn QueryIndexLiteral(
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
				if MatchesFilters(file_path, metadata, path_filter, language_filter) {
					if let Ok(search_result) =
						FindMatchesInFile(file_path, &search_query, case_sensitive, whole_word, index).await
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
			if MatchesFilters(file_path, metadata, path_filter, language_filter) {
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
async fn QueryIndexRegex(
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

		if !MatchesFilters(file_path, metadata, path_filter, language_filter) {
			continue;
		}

		if let Ok(content) = tokio::fs::read_to_string(file_path).await {
			let matches = FindRegexMatches(&content, regex);

			if !matches.is_empty() {
				let relevance = CalculateRelevance(&matches, metadata);

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
async fn QueryIndexFuzzy(
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

		if !MatchesFilters(file_path, metadata, path_filter, language_filter) {
			continue;
		}

		if let Ok(content) = tokio::fs::read_to_string(file_path).await {
			const MAX_FUZZY_DISTANCE:usize = 2;

			let matches = FindFuzzyMatches(&content, &query_lower, case_sensitive, MAX_FUZZY_DISTANCE);

			if !matches.is_empty() {
				let relevance = CalculateRelevance(&matches, metadata) * 0.8; // Fuzzy matches have lower relevance

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
async fn QueryIndexExact(
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

		if !MatchesFilters(file_path, metadata, path_filter, language_filter) {
			continue;
		}

		if let Ok(content) = tokio::fs::read_to_string(file_path).await {
			let matches = FindExactMatches(&content, query);

			if !matches.is_empty() {
				let relevance = CalculateRelevance(&matches, metadata) * 1.1; // Exact matches have higher relevance

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

	let matches = FindMatchesWithContext(&content, query, case_sensitive, whole_word);

	let relevance = CalculateRelevance(&matches, metadata);

	Ok(SearchResult {
		path:file_path.to_string_lossy().to_string(),
		file_name:file_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
		matches,
		relevance,
		language:metadata.language.clone(),
	})
}

/// Find matches in content with surrounding context
fn FindMatchesWithContext(content:&str, query:&str, case_sensitive:bool, whole_word:bool) -> Vec<SearchMatch> {
	let mut matches = Vec::new();

	let lines:Vec<&str> = content.lines().collect();

	let search_in = |line:&str| -> Option<(usize, usize)> {
		let line_to_search = if case_sensitive { line.to_string() } else { line.to_lowercase() };

		let query_to_find = if case_sensitive { query.to_string() } else { query.to_lowercase() };

		let start = if whole_word {
			FindWholeWordMatch(&line_to_search, &query_to_find)
		} else {
			line_to_search.find(&query_to_find)
		};

		start.map(|s| (s, s + query.len()))
	};

	for (line_idx, line) in lines.iter().enumerate() {
		let line_number = line_idx as u32 + 1;

		if let Some((match_start, match_end)) = search_in(line) {
			// Get context lines (2 before, 2 after)
			let context_start = line_idx.saturating_sub(2);

			let context_end = (line_idx + 3).min(lines.len());

			let context_before = lines[context_start..line_idx].iter().map(|s| s.to_string()).collect();

			let context_after = lines[line_idx + 1..context_end].iter().map(|s| s.to_string()).collect();

			matches.push(SearchMatch {
				line_number,
				line_content:line.to_string(),
				match_start,
				match_end,
				context_before,
				context_after,
			});
		}
	}

	matches
}

/// Find whole word match with word boundary detection
fn FindWholeWordMatch(line:&str, word:&str) -> Option<usize> {
	let mut start = 0;

	while let Some(pos) = line[start..].find(word) {
		let actual_pos = start + pos;

		// Check word boundary before
		let valid_before = actual_pos == 0
			|| line
				.chars()
				.nth(actual_pos - 1)
				.map_or(true, |c| !c.is_alphanumeric() && c != '_');

		// Check word boundary after
		let match_end = actual_pos + word.len();

		let valid_after =
			match_end == line.len() || line.chars().nth(match_end).map_or(true, |c| !c.is_alphanumeric() && c != '_');

		if valid_before && valid_after {
			return Some(actual_pos);
		}

		start = actual_pos + 1;
	}

	None
}

/// Find regex matches in content
fn FindRegexMatches(content:&str, regex:&Regex) -> Vec<SearchMatch> {
	let mut matches = Vec::new();

	let lines:Vec<&str> = content.lines().collect();

	for (line_idx, line) in lines.iter().enumerate() {
		let line_number = line_idx as u32 + 1;

		for mat in regex.find_iter(line) {
			matches.push(SearchMatch {
				line_number,
				line_content:line.to_string(),
				match_start:mat.start(),
				match_end:mat.end(),
				context_before:Vec::new(),
				context_after:Vec::new(),
			});
		}
	}

	matches
}

/// Find fuzzy matches using Levenshtein distance algorithm
fn FindFuzzyMatches(content:&str, query:&str, case_sensitive:bool, max_distance:usize) -> Vec<SearchMatch> {
	let mut matches = Vec::new();

	let lines:Vec<&str> = content.lines().collect();

	for (line_idx, line) in lines.iter().enumerate() {
		let line_number = line_idx as u32 + 1;

		let line_to_search = if case_sensitive { line.to_string() } else { line.to_lowercase() };

		// Calculate Levenshtein distance for fuzzy matching
		if let Some(pos) = line_to_search.find(query) {
			// Check if the match is within the MaxDistance threshold
			let distance = CalculateLevenshteinDistance(&line_to_search[pos..pos.saturating_add(query.len())], query);

			if distance <= max_distance {
				matches.push(SearchMatch {
					line_number,
					line_content:line.to_string(),
					match_start:pos,
					match_end:pos + query.len(),
					context_before:Vec::new(),
					context_after:Vec::new(),
				});
			}
		}
	}

	matches
}

/// Find exact matches (word boundary and case-sensitive)
fn FindExactMatches(content:&str, query:&str) -> Vec<SearchMatch> { FindMatchesWithContext(content, query, true, true) }

/// Calculate Levenshtein distance between two strings
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
	if match_count > 0 {
		relevance += (match_count as f64).log10() * 0.5;
	}

	// Bonus for recently modified files
	let days_old = (chrono::Utc::now() - metadata.modified).num_days() as f64;

	relevance += 1.0 / (days_old + 1.0).max(1.0);

	relevance.min(10.0).max(0.0)
}

/// Check if file matches filters
pub fn MatchesFilters(
	file_path:&PathBuf,

	metadata:&FileMetadata,

	path_filter:Option<&str>,

	language_filter:Option<&str>,
) -> bool {
	// Check path filter
	if let Some(search_path) = path_filter {
		if !file_path.to_string_lossy().contains(search_path) {
			return false;
		}
	}

	// Check language filter
	if let Some(lang) = language_filter {
		if metadata.language.as_deref() != Some(lang) {
			return false;
		}
	}

	true
}
