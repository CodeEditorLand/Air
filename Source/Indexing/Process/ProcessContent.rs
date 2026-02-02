//! # ProcessContent
//!
//! ## File: Indexing/Process/ProcessContent.rs
//!
//! ## Role in Air Architecture
//!
//! Provides content processing functionality for the File Indexer service,
//! handling encoding detection, MIME type detection, and content tokenization.
//!
//! ## Primary Responsibility
//!
//! Process file content for indexing by detecting encoding, mime types, and
//! tokenizing text for search operations.
//!
//! ## Secondary Responsibilities
//!
//! - File encoding detection (UTF-8, UTF-16, ASCII)
//! - MIME type detection from extensions and content
//! - Content tokenization for search indexing
//! - Language detection for code analysis
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - None (uses std library)
//!
//! **Internal Modules:**
//! - `crate::Result` - Error handling type
//!
//! ## Dependents
//!
//! - `Indexing::Scan::ScanFile` - Content processing during file scan
//! - `Indexing::Store::StoreEntry` - Index storage operations
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's content processing in
//! `src/vs/base/node/encoding/`
//!
//! ## Security Considerations
//!
//! - Safe BOM marker detection
//! - Null byte filtering
//! - Length limits on processed content
//!
//! ## Performance Considerations
//!
//! - Efficient tokenization with minimal allocations
//! - Early termination for binary files
//! - Lazy content evaluation
//!
//! ## Error Handling Strategy
//!
//! Content processing functions return Option or safe defaults when
//! detection fails, rather than errors, to allow indexing to continue.
//!
//! ## Thread Safety
//!
//! Content processing functions are pure and safe to call from
//! parallel indexing tasks.

use std::path::PathBuf;

/// Detect file encoding (simplified detection)
pub fn DetectEncoding(content:&[u8]) -> Option<String> {
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

/// Detect MIME type with comprehensive file type detection
pub fn DetectMimeType(file_path:&PathBuf, content:&[u8]) -> String {
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
				DetectMimeTypeFromContent(content)
			},
		}
	} else {
		// No extension, try content-based detection
		DetectMimeTypeFromContent(content)
	}
}

/// Detect MIME type from content (magic numbers)
fn DetectMimeTypeFromContent(content:&[u8]) -> String {
	if content.is_empty() {
		return "application/octet-stream".to_string();
	}

	if content.starts_with(b"{") || content.starts_with(b"[") {
		"application/json".to_string()
	} else if content.starts_with(b"#!") {
		"text/x-shellscript".to_string()
	} else if content.starts_with(b"<?xml") {
		"application/xml".to_string()
	} else if content.starts_with(b"<!DOCTYPE") || content.starts_with(b"<html") {
		"text/html".to_string()
	} else if content.starts_with(b"---") {
		"text/x-yaml".to_string()
	} else if content.is_ascii() && !content.windows(4).any(|w| w.starts_with(&[0u8])) {
		"text/plain".to_string()
	} else {
		"application/octet-stream".to_string()
	}
}

/// Detect programming language from file extension and shebang
pub fn DetectLanguage(file_path:&PathBuf) -> Option<String> {
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
			"mojo" => "mojo",
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

/// Tokenize content for indexing with improved word boundary handling
pub fn TokenizeContent(content:&str) -> Vec<String> {
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

/// Remove null bytes and control characters from content
pub fn SanitizeContent(content:&str) -> String { content.chars().filter(|c| *c != '\0' && !c.is_control()).collect() }

/// Convert content to UTF-8 string with error handling
pub fn ContentToString(content:&[u8]) -> Result<String> {
	String::from_utf8(content.to_vec())
		.map_err(|e| crate::AirError::FileSystem(format!("Invalid UTF-8 content: {}", e)))
}

/// Check if content is likely binary (contains null bytes or high ratio of
/// non-text)
pub fn IsBinaryContent(content:&[u8]) -> bool {
	const MAX_NULL_BYTES:usize = 10;
	const BINARY_SCAN_LIMIT:usize = 8000;

	let scan_length = content.len().min(BINARY_SCAN_LIMIT);
	let null_count = content[..scan_length].iter().filter(|&&b| b == 0).count();

	if null_count > MAX_NULL_BYTES {
		return true;
	}

	// Check for high ratio of non-text bytes in first chunk
	let scan_bytes = &content[..scan_length];
	let text_ratio = scan_bytes
		.iter()
		.filter(|&&b| b.is_ascii_graphic() || b.is_ascii_whitespace() || b >= 0x80)
		.count() as f64
		/ scan_length as f64;

	text_ratio < 0.7
}

/// Get line count from content
pub fn GetLineCount(content:&str) -> u32 {
	if content.is_empty() {
		return 0;
	}
	content.lines().count() as u32
}

/// Get char count from content
pub fn GetCharCount(content:&str) -> usize { content.chars().count() }

/// Truncate content to specified maximum size in characters
pub fn TruncateContent(content:&str, max_chars:usize) -> String {
	let chars:Vec<char> = content.chars().take(max_chars).collect();
	chars.into_iter().collect()
}
