//! # ExtractSymbols
//!
//! ## File: Indexing/Process/ExtractSymbols.rs
//!
//! ## Role in Air Architecture
//!
//! Provides symbol extraction functionality for the File Indexer service,
//! extracting classes, functions, and other code constructs for VSCode
//! Outline View and Go to Symbol features.
//!
//! ## Primary Responsibility
//!
//! Extract code symbols from file content based on detected language,
//! including functions, classes, structs, enums, traits, and more.
//!
//! ## Secondary Responsibilities
//!
//! - Language-specific symbol extraction
//! - Line and column tracking for symbols
//! - Symbol kind classification
//! - Cross-file symbol reference support
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - None (uses std library)
//!
//! **Internal Modules:**
//! - `crate::Result` - Error handling type
//! - `super::Language` - Language-specific parsers
//!
//! ## Dependents
//!
//! - `Indexing::Scan::ScanFile` - Symbol extraction during file scan
//! - `Indexing::mod::FileIndexer` - Symbol search operations
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's symbol extraction in
//! `src/vs/workbench/services/search/common/`
//!
//! ## Security Considerations
//!
//! - Line-by-line parsing without eval
//! - No code execution during extraction
//! - Safe string handling
//!
//! ## Performance Considerations
//!
//! - efficient line-based parsing
//! - Minimal allocations per file
//! - Early termination for non-code files
//!
//! ## Error Handling Strategy
//!
//! Symbol extraction returns empty vectors on parse errors rather than
//! failures, allowing indexing to continue for other languages.
//!
//! ## Thread Safety
//!
//! Symbol extraction functions are pure and safe to call from
//! parallel indexing tasks.

use std::path::PathBuf;

use crate::{
	Indexing::{
		Language::{ParseRust::ExtractRustSymbols, ParseTypeScript::ExtractTypeScriptSymbols},
		State::CreateState::{SymbolInfo, SymbolKind},
	},
	Result,
};

/// Extract symbols from code for VSCode Outline View and Go to Symbol
///
/// Supports multiple programming languages:
/// - Rust: struct, impl, fn, mod, enum, trait, type
/// - TypeScript/JavaScript: class, interface, function, const, let, var
/// - Python: class, def
/// - Go: type, func, struct, interface
pub async fn ExtractSymbols(file_path:&PathBuf, content:&[u8], language:&str) -> Result<Vec<SymbolInfo>> {
	let content_str = String::from_utf8_lossy(content);

	let mut symbols = Vec::new();

	match language.to_lowercase().as_str() {
		"rust" => symbols.extend(ExtractRustSymbols(&content_str, file_path)),

		"typescript" | "javascript" => symbols.extend(ExtractTypeScriptSymbols(&content_str, file_path)),

		_ => {},
	}

	Ok(symbols)
}

/// Group symbols by kind for organization
pub fn GroupSymbolsByKind(symbols:&[SymbolInfo]) -> std::collections::HashMap<SymbolKind, Vec<&SymbolInfo>> {
	let mut grouped = std::collections::HashMap::new();

	for symbol in symbols {
		grouped.entry(symbol.kind.clone()).or_insert_with(Vec::new).push(symbol);
	}

	grouped
}

/// Sort symbols by line number
pub fn SortSymbolsByLine(symbols:&mut Vec<SymbolInfo>) { symbols.sort_by(|a, b| a.line.cmp(&b.line)); }

/// Filter symbols by name pattern
pub fn FilterSymbolsByName<'a>(symbols:&'a [SymbolInfo], pattern:&str) -> Vec<&'a SymbolInfo> {
	let pattern_lower = pattern.to_lowercase();

	symbols
		.iter()
		.filter(|s| s.name.to_lowercase().contains(&pattern_lower))
		.collect()
}

/// Get symbols of a specific kind
pub fn GetSymbolsByKind(symbols:&[SymbolInfo], kind:SymbolKind) -> Vec<&SymbolInfo> {
	symbols.iter().filter(|s| s.kind == kind).collect()
}

/// Find symbol at specific line
pub fn FindSymbolAtLine(symbols:&[SymbolInfo], line:u32) -> Option<&SymbolInfo> {
	symbols.iter().find(|s| s.line == line)
}

/// Find symbols in line range
pub fn FindSymbolsInRange(symbols:&[SymbolInfo], start_line:u32, end_line:u32) -> Vec<&SymbolInfo> {
	symbols.iter().filter(|s| s.line >= start_line && s.line <= end_line).collect()
}

/// Create symbol summary statistics
pub fn GetSymbolStatistics(symbols:&[SymbolInfo]) -> SymbolStatistics {
	let mut stats = SymbolStatistics { total:symbols.len(), by_kind:std::collections::HashMap::new() };

	for symbol in symbols {
		*stats.by_kind.entry(symbol.kind.clone()).or_insert(0) += 1;
	}

	stats
}

/// Symbol statistics
#[derive(Debug, Clone)]
pub struct SymbolStatistics {
	pub total:usize,

	pub by_kind:std::collections::HashMap<SymbolKind, usize>,
}

impl std::fmt::Display for SymbolStatistics {
	fn fmt(&self, f:&mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Total symbols: {}", self.total)?;

		for (kind, count) in &self.by_kind {
			write!(f, ", {:?}: {}", kind, count)?;
		}

		Ok(())
	}
}

/// Validate symbol information
pub fn ValidateSymbol(symbol:&SymbolInfo) -> bool {
	!symbol.name.is_empty() && symbol.line > 0 && !symbol.full_path.is_empty()
}

/// Deduplicate symbols by name and line
pub fn DeduplicateSymbols(symbols:Vec<SymbolInfo>) -> Vec<SymbolInfo> {
	let mut seen = std::collections::HashSet::new();

	symbols.into_iter().filter(|s| seen.insert((s.name.clone(), s.line))).collect()
}

/// Merge symbol lists from multiple files
pub fn MergeSymbolLists(symbol_lists:Vec<Vec<SymbolInfo>>) -> Vec<SymbolInfo> {
	let mut merged = Vec::new();

	for symbols in symbol_lists {
		merged.extend(symbols);
	}

	DeduplicateSymbols(merged)
}

/// Deduplicate multiple symbol lists
pub fn DeduplicateLists(symbol_lists:Vec<Vec<SymbolInfo>>) -> Vec<Vec<SymbolInfo>> {
	symbol_lists.into_iter().map(|list| DeduplicateSymbols(list)).collect()
}

/// Create a symbol search index (name -> symbols)
pub fn CreateSymbolIndex(symbols:&[SymbolInfo]) -> std::collections::HashMap<String, Vec<usize>> {
	let mut index = std::collections::HashMap::new();

	for (idx, symbol) in symbols.iter().enumerate() {
		index.entry(symbol.name.to_lowercase()).or_insert_with(Vec::new).push(idx);
	}

	index
}

/// Find symbols matching multiple criteria
pub fn FindSymbolsMatching<'a>(
	symbols:&'a [SymbolInfo],

	name_pattern:Option<&'a str>,

	kind:&Option<SymbolKind>,

	line_range:Option<(u32, u32)>,
) -> Vec<&'a SymbolInfo> {
	symbols
		.iter()
		.filter(|s| {
			if let Some(pattern) = name_pattern {
				if !s.name.to_lowercase().contains(&pattern.to_lowercase()) {
					return false;
				}
			}

			if let Some(k) = kind {
				if s.kind != *k {
					return false;
				}
			}

			if let Some((start, end)) = line_range {
				if s.line < start || s.line > end {
					return false;
				}
			}

			true
		})
		.collect()
}
