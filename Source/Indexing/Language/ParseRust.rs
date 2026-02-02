//! # ParseRust
//!
//! ## File: Indexing/Language/ParseRust.rs
//!
//! ## Role in Air Architecture
//!
//! Provides Rust-specific symbol extraction functionality for the File Indexer
//! service, identifying Rust language constructs like structs, impl blocks,
//! functions, modules, enums, and traits.
//!
//! ## Primary Responsibility
//!
//! Extract Rust code symbols from source files for VSCode Outline View and
//! Go to Symbol features.
//!
//! ## Secondary Responsibilities
//!
//! - Extract struct definitions
//! - Extract impl blocks
//! - Extract function definitions
//! - Extract module declarations
//! - Extract enum definitions
//! - Extract trait definitions
//! - Extract type aliases
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - None (uses std library)
//!
//! **Internal Modules:**
//! - `crate::Result` - Error handling type
//! - `super::super::SymbolInfo` - Symbol structure definitions
//!
//! ## Dependents
//!
//! - `Indexing::Process::ExtractSymbols` - Language routing
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's Rust symbol extraction in
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
//! - Efficient line-based parsing
//! - Minimal allocations per file
//! - Early termination for non-Rust files
//!
//! ## Error Handling Strategy
//!
//! Symbol extraction returns empty vectors on parse errors rather than
//! failures, allowing indexing to continue for other files.
//!
//! ## Thread Safety
//!
//! Symbol extraction functions are pure and safe to call from
//! parallel indexing tasks.

use std::path::PathBuf;

use super::super::SymbolInfo;
use super::super::SymbolKind;

/// Extract Rust symbols (struct, impl, fn, mod, enum, trait)
pub fn ExtractRustSymbols(content: &str, file_path: &PathBuf) -> Vec<SymbolInfo> {
	let mut symbols = Vec::new();
	let lines: Vec<&str> = content.lines().collect();

	for (line_idx, line) in lines.iter().enumerate() {
		let line_content = line.trim();
		let line_num = line_idx as u32 + 1;

		// Check for comments and skip them
		if line_content.starts_with("//") || line_content.starts_with("/*") || line_content.starts_with("*") {
			continue;
		}

		// Extract symbols from this line
		symbols.extend(ExtractRustSymbolsFromLine(line_content, line_num, line, file_path));
	}

	symbols
}

/// Extract symbols from a single line of Rust code
fn ExtractRustSymbolsFromLine(line_content: &str, line_num: u32, line: &str, file_path: &PathBuf) -> Vec<SymbolInfo> {
	let mut symbols = Vec::new();

	// Struct
	if let Some(rest) = line_content.strip_prefix("struct ") {
		let name = rest.split_whitespace().next().unwrap_or("").trim_end_matches('{');
		if !name.is_empty() {
			if let Some(col) = line.find("struct") {
				symbols.push(SymbolInfo {
					name: name.to_string(),
					kind: SymbolKind::Struct,
					line: line_num,
					column: col as u32,
					full_path: format!("{}::{}", file_path.display(), name),
				});
			}
		}
	}

	// impl
	if let Some(rest) = line_content.strip_prefix("impl ") {
		let name = rest.split_whitespace().next().unwrap_or("").trim_end_matches('{');
		if !name.is_empty() {
			if let Some(col) = line.find("impl") {
				symbols.push(SymbolInfo {
					name: name.to_string(),
					kind: SymbolKind::Method,
					line: line_num,
					column: col as u32,
					full_path: format!("{}::{}::", file_path.display(), name),
				});
			}
		}
	}

	// Function
	if let Some(rest) = line_content.strip_prefix("fn ") {
		let name = rest.split(|c| c == '(' || c == '<' || c == ':').next().unwrap_or("").trim();
		if !name.is_empty() {
			if let Some(col) = line.find("fn") {
				symbols.push(SymbolInfo {
					name: name.to_string(),
					kind: SymbolKind::Function,
					line: line_num,
					column: col as u32,
					full_path: format!("{}::{}", file_path.display(), name),
				});
			}
		}
	}

	// Module
	if let Some(rest) = line_content.strip_prefix("mod ") {
		let name = rest.split_whitespace().next().unwrap_or("").trim_end_matches('{');
		if !name.is_empty() {
			if let Some(col) = line.find("mod") {
				symbols.push(SymbolInfo {
					name: name.to_string(),
					kind: SymbolKind::Module,
					line: line_num,
					column: col as u32,
					full_path: format!("{}::{}::", file_path.display(), name),
				});
			}
		}
	}

	// Enum
	if let Some(rest) = line_content.strip_prefix("enum ") {
		let name = rest.split_whitespace().next().unwrap_or("").trim_end_matches('{');
		if !name.is_empty() {
			if let Some(col) = line.find("enum") {
				symbols.push(SymbolInfo {
					name: name.to_string(),
					kind: SymbolKind::Enum,
					line: line_num,
					column: col as u32,
					full_path: format!("{}::{}", file_path.display(), name),
				});
			}
		}
	}

	// Trait
	if let Some(rest) = line_content.strip_prefix("trait ") {
		let name = rest.split_whitespace().next().unwrap_or("").trim_end_matches('{');
		if !name.is_empty() {
			if let Some(col) = line.find("trait") {
				symbols.push(SymbolInfo {
					name: name.to_string(),
					kind: SymbolKind::Interface,
					line: line_num,
					column: col as u32,
					full_path: format!("{}::{}", file_path.display(), name),
				});
			}
		}
	}

	// Type alias
	if let Some(rest) = line_content.strip_prefix("type ") {
		let name = rest.split('=').next().unwrap_or("").trim().trim_end_matches(';');
		if !name.is_empty() {
			if let Some(col) = line.find("type") {
				symbols.push(SymbolInfo {
					name: name.to_string(),
					kind: SymbolKind::TypeParameter,
					line: line_num,
					column: col as u32,
					full_path: format!("{}::{}", file_path.display(), name),
				});
			}
		}
	}

	// Const
	if line_content.starts_with("const ") && !line_content.contains('=') {
		if let Some(rest) = line_content.strip_prefix("const ") {
			let name = rest.split(|c| c == ':' || c == '=').next().unwrap_or("").trim();
			if !name.is_empty() {
				if let Some(col) = line.find("const") {
					symbols.push(SymbolInfo {
						name: name.to_string(),
						kind: SymbolKind::Constant,
						line: line_num,
						column: col as u32,
						full_path: format!("{}::{}", file_path.display(), name),
					});
				}
			}
		}
	}

	// Static
	if line_content.starts_with("static ") {
		if let Some(rest) = line_content.strip_prefix("static ") {
			let name = rest.split(|c| c == ':' || c == '=').next().unwrap_or("").trim();
			if !name.is_empty() {
				if let Some(col) = line.find("static") {
					symbols.push(SymbolInfo {
						name: name.to_string(),
						kind: SymbolKind::Variable,
						line: line_num,
						column: col as u32,
						full_path: format!("{}::{}", file_path.display(), name),
					});
				}
			}
		}
	}

	symbols
}

/// Check if a line contains a Rust struct definition
pub fn IsRustStruct(line: &str) -> bool {
	let trimmed = line.trim();
	let after_keywords = trimmed
		.strip_prefix("pub ")
		.or_else(|| trimmed.strip_prefix("unsafe "))
		.or_else(|| trimmed.strip_prefix("pub(crate) "))
		.unwrap_or(trimmed);
	after_keywords.starts_with("struct ")
}

/// Check if a line contains a Rust function definition
pub fn IsRustFunction(line: &str) -> bool {
	let trimmed = line.trim();
	let after_keywords = trimmed
		.strip_prefix("pub ")
		.or_else(|| trimmed.strip_prefix("pub(crate) "))
		.or_else(|| trimmed.strip_prefix("unsafe "))
		.or_else(|| trimmed.strip_prefix("async "))
		.unwrap_or(trimmed);
	after_keywords.starts_with("fn ")
}

/// Check if a line contains a Rust impl block
pub fn IsRustImpl(line: &str) -> bool {
	// Handle variations: impl, pub impl, unsafe impl
	let trimmed = line.trim();
	let after_keywords = trimmed
		.strip_prefix("pub ")
		.or_else(|| trimmed.strip_prefix("unsafe "))
		.unwrap_or(trimmed);
	after_keywords.starts_with("impl ")
}

/// Extract Rust visibility modifier if present
pub fn ExtractVisibilityModifier(line: &str) -> Option<&str> {
	let trimmed = line.trim();
	if trimmed.starts_with("pub ") {
		Some("pub")
	} else if trimmed.starts_with("pub(crate) ") {
		Some("pub(crate)")
	} else if trimmed.starts_with("pub(super) ") {
		Some("pub(super)")
	} else if trimmed.starts_with("pub(in ") {
		// Extract the path part
		let rest = trimmed.strip_prefix("pub(in ").unwrap_or("");
		let path = rest.split(')').next().unwrap_or("");
		if !path.is_empty() {
			Some(&trimmed[0..trimmed.find(')').unwrap_or(trimmed.len()) + 1])
		} else {
			None
		}
	} else {
		None
	}
}
