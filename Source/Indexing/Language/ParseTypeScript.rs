//! # ParseTypeScript
//!
//! ## File: Indexing/Language/ParseTypeScript.rs
//!
//! ## Role in Air Architecture
//!
//! Provides TypeScript/JavaScript-specific symbol extraction functionality for
//! the File Indexer service, identifying TS/JS language constructs like
//! classes, interfaces, functions, constants, and types.
//!
//! ## Primary Responsibility
//!
//! Extract TypeScript/JavaScript code symbols from source files for VSCode
//! Outline View and Go to Symbol features.
//!
//! ## Secondary Responsibilities
//!
//! - Extract class definitions
//! - Extract interface definitions
//! - Extract function declarations
//! - Extract arrow functions
//! - Extract variable declarations (const, let, var)
//! - Extract type definitions
//! - Extract enum definitions
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
//! Inspired by VSCode's TypeScript symbol extraction in
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
//! - Early termination for non-TS/JS files
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

use crate::Indexing::State::CreateState::{SymbolInfo, SymbolKind};

/// Extract TypeScript/JavaScript symbols (class, interface, function, etc.)
pub fn ExtractTypeScriptSymbols(content:&str, file_path:&PathBuf) -> Vec<SymbolInfo> {
	let mut symbols = Vec::new();

	let lines:Vec<&str> = content.lines().collect();

	for (line_idx, line) in lines.iter().enumerate() {
		let line_content = line.trim();

		let line_num = line_idx as u32 + 1;

		// Skip comments
		if line_content.starts_with("//") || line_content.starts_with("/*") || line_content.starts_with("*") {
			continue;
		}

		// Extract symbols from this line
		symbols.extend(ExtractTypeScriptSymbolsFromLine(line_content, line_num, line, file_path));
	}

	symbols
}

/// Extract symbols from a single line of TypeScript/JavaScript code
fn ExtractTypeScriptSymbolsFromLine(line_content:&str, line_num:u32, line:&str, file_path:&PathBuf) -> Vec<SymbolInfo> {
	let mut symbols = Vec::new();

	// Class
	if let Some(rest) = line_content.strip_prefix("class ") {
		let name = rest.split(|c| c == '{' || c == '<' || c == ' ').next().unwrap_or("").trim();
		if !name.is_empty() {
			if let Some(col) = line.find("class") {
				symbols.push(SymbolInfo {
					name:name.to_string(),
					kind:SymbolKind::Class,
					line:line_num,
					column:col as u32,
					full_path:format!("{}::{}", file_path.display(), name),
				});
			}
		}
	}

	// Interface
	if let Some(rest) = line_content.strip_prefix("interface ") {
		let name = rest.split(|c| c == '{' || c == '<' || c == ' ').next().unwrap_or("").trim();
		if !name.is_empty() {
			if let Some(col) = line.find("interface") {
				symbols.push(SymbolInfo {
					name:name.to_string(),
					kind:SymbolKind::Interface,
					line:line_num,
					column:col as u32,
					full_path:format!("{}::{}", file_path.display(), name),
				});
			}
		}
	}

	// Type
	if let Some(rest) = line_content.strip_prefix("type ") {
		// Handle type aliases which may end with = or {
		let name = rest.split(|c| c == '=' || c == '{' || c == ';').next().unwrap_or("").trim();
		if !name.is_empty() {
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

	// Enum
	if let Some(rest) = line_content.strip_prefix("enum ") {
		let name = rest.split(|c| c == '{' || c == ';').next().unwrap_or("").trim();
		if !name.is_empty() {
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
	}

	// Function declaration
	if let Some(rest) = line_content.strip_prefix("function ") {
		let name = rest.split('(').next().unwrap_or("").trim();
		if !name.is_empty() {
			// Check for arrow functions: const name = () => {}
			if !name.contains("=") {
				if let Some(col) = line.find("function") {
					symbols.push(SymbolInfo {
						name:name.to_string(),
						kind:SymbolKind::Function,
						line:line_num,
						column:col as u32,
						full_path:format!("{}::{}", file_path.display(), name),
					});
				}
			}
		}
	}

	// Arrow function
	if line_content.contains("=>") {
		if let Some(col) = line.find("=>") {
			let before_arrow = &line[..col];
			// Try to extract function name
			let name_part = before_arrow.split('=').next().unwrap_or("").trim();

			let func_name = if name_part.contains("(") || name_part.contains("<") {
				let mut parts = name_part.split(|c| c == '(' || c == '<' || c == ':');
				let name = parts.next().unwrap_or("").trim();
				name
			} else {
				name_part
			};

			// Filter out keywords and non-names
			if !func_name.is_empty() && func_name != "const" && func_name != "let" && func_name != "var" {
				symbols.push(SymbolInfo {
					name:func_name.to_string(),
					kind:SymbolKind::Function,
					line:line_num,
					column:col as u32,
					full_path:format!("{}::{}", file_path.display(), func_name),
				});
			}
		}
	}

	// Const/let/var declarations
	for kw in &["const ", "let ", "var "] {
		if let Some(rest) = line_content.strip_prefix(kw) {
			let name = rest.split(|c| c == '=' || c == ':' || c == ';').next().unwrap_or("").trim();
			// Check if it's a function assignment: const myFunc = () => {}
			let _is_function_assignment = !line_content.contains("=>")
				&& !line_content.contains("function")
				&& (line_content.contains("=>") || rest.to_lowercase().contains("function"));

			if !name.is_empty() {
				// Determine if it's a constant or variable
				let kind = if line_content.starts_with("const ") {
					SymbolKind::Constant
				} else {
					SymbolKind::Variable
				};

				if let Some(col) = line.find(kw) {
					symbols.push(SymbolInfo {
						name:name.to_string(),
						kind,
						line:line_num,
						column:col as u32,
						full_path:format!("{}::{}", file_path.display(), name),
					});
				}
			}
		}
	}

	// Namespace
	if let Some(rest) = line_content.strip_prefix("namespace ") {
		let name = rest.split(|c| c == '{' || c == ';').next().unwrap_or("").trim();
		if !name.is_empty() {
			if let Some(col) = line.find("namespace") {
				symbols.push(SymbolInfo {
					name:name.to_string(),
					kind:SymbolKind::Namespace,
					line:line_num,
					column:col as u32,
					full_path:format!("{}::{}", file_path.display(), name),
				});
			}
		}
	}

	symbols
}

/// Check if a line contains a TypeScript/JavaScript class definition
pub fn IsTypeScriptClass(line:&str) -> bool {
	let trimmed = line.trim();
	let after_keywords = trimmed
		.strip_prefix("export ")
		.or_else(|| trimmed.strip_prefix("default "))
		.or_else(|| trimmed.strip_prefix("declare "))
		.unwrap_or(trimmed);
	after_keywords.starts_with("class ") && !after_keywords.contains(" extends ")
}

/// Check if a line contains a TypeScript/JavaScript interface definition
pub fn IsTypeScriptInterface(line:&str) -> bool {
	let trimmed = line.trim();
	let after_keywords = trimmed
		.strip_prefix("export ")
		.or_else(|| trimmed.strip_prefix("default "))
		.or_else(|| trimmed.strip_prefix("declare "))
		.unwrap_or(trimmed);
	after_keywords.starts_with("interface ")
}

/// Check if a line contains a TypeScript/JavaScript function definition
pub fn IsTypeScriptFunction(line:&str) -> bool {
	let trimmed = line.trim();
	let after_keywords = trimmed
		.strip_prefix("export ")
		.or_else(|| trimmed.strip_prefix("default "))
		.or_else(|| trimmed.strip_prefix("declare "))
		.or_else(|| trimmed.strip_prefix("async "))
		.unwrap_or(trimmed);
	after_keywords.starts_with("function ")
}

/// Extract TypeScript/JavaScript export modifier if present
pub fn ExtractExportModifier(line:&str) -> Option<&str> {
	let trimmed = line.trim();
	if trimmed.starts_with("export ") {
		Some("export")
	} else if trimmed.starts_with("export default ") {
		Some("export default")
	} else if trimmed.starts_with("export type ") {
		Some("export type")
	} else if trimmed.starts_with("export const ") {
		Some("export const")
	} else if trimmed.starts_with("export function ") {
		Some("export function")
	} else if trimmed.starts_with("export interface ") {
		Some("export interface")
	} else if trimmed.starts_with("export class ") {
		Some("export class")
	} else {
		None
	}
}

/// Extract TypeScript/JavaScript type annotation from a declaration
pub fn ExtractTypeAnnotation(line:&str) -> Option<String> {
	if let Some(colon_idx) = line.find(':') {
		let rest = &line[colon_idx + 1..];
		// Find the end of the type annotation (before =, {, ;, or <)
		let end_idx = rest
			.find(|c| c == '=' || c == '{' || c == ';' || c == ',')
			.unwrap_or(rest.len());
		let type_str = rest[..end_idx].trim();
		if !type_str.is_empty() { Some(type_str.to_string()) } else { None }
	} else {
		None
	}
}

/// Parse TypeScript/JavaScript generic parameters
pub fn ExtractGenericParameters(line:&str) -> Vec<String> {
	let mut generics = Vec::new();
	if let Some(start) = line.find('<') {
		if let Some(end) = line.rfind('>') {
			let content = &line[start + 1..end];
			// Split by comma and trim
			for part in content.split(',') {
				let trimmed = part.trim();
				if !trimmed.is_empty() {
					generics.push(trimmed.to_string());
				}
			}
		}
	}
	generics
}
