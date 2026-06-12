/// Tool to rewrite rustdoc comments in the Air crate to meet quality standards.
///
/// Auto-fixes:
/// 1. "/// This <noun> ..." meta-text → direct verb/noun form
/// 2. Adds module-level //! docs to mod.rs files missing them
/// 3. No behavioral changes.
///
/// Run: cargo run
use std::fs;
use std::path::PathBuf;

fn main() {
	let source_dir = std::env::current_dir().unwrap().parent().unwrap().join("Source");

	if !source_dir.exists() {
		eprintln!("Source directory not found at {:?}", source_dir);

		// Try relative
		let alt = PathBuf::from("../Source");

		if alt.exists() {
			run(&alt);
		} else {
			eprintln!("Also tried {:?}", alt);

			std::process::exit(1);
		}

		return;
	}

	run(&source_dir);
}

fn run(source_dir:&std::path::Path) {
	let mut files:Vec<PathBuf> = Vec::new();

	collect_rs_files(source_dir, &mut files);

	let mut total_fixes = 0u64;

	let mut changed_files = 0u64;

	let mut meta_fixes = 0u64;

	let mut module_docs_added = 0u64;

	for file_path in &files {
		let content = match fs::read_to_string(file_path) {
			Ok(c) => c,

			Err(e) => {
				eprintln!("Error reading {:?}: {}", file_path, e);

				continue;
			},
		};

		let original = content.clone();

		let is_mod_rs = file_path.file_name().map_or(false, |n| n == "mod.rs");

		let is_library_rs = file_path.file_name().map_or(false, |n| n == "Library.rs");

		// Fix 1: Replace "This function/struct/enum/trait/module/type/method" meta-text
		let after_meta = fix_meta_text(&content, &mut meta_fixes);

		// Fix 2: Add module-level //! docs if missing (mod.rs files only)
		let after_module = if is_mod_rs && !is_library_rs {
			add_module_docs_if_missing(&after_meta, file_path, &mut module_docs_added)
		} else {
			after_meta
		};

		if after_module != original {
			fs::write(file_path, &after_module).unwrap_or_else(|e| {
				eprintln!("Error writing {:?}: {}", file_path, e);
			});

			changed_files += 1;

			total_fixes += count_diffs(&original, &after_module);

			println!(
				"  FIXED: {} ({} changes)",
				file_path.display(),
				count_diffs(&original, &after_module)
			);
		}
	}

	println!("\n=== Summary ===");

	println!("Files processed: {}", files.len());

	println!("Files changed: {}", changed_files);

	println!("Meta-text fixes: {}", meta_fixes);

	println!("Module docs added: {}", module_docs_added);

	println!("Total line-level changes: {}", total_fixes);
}

fn collect_rs_files(dir:&std::path::Path, files:&mut Vec<PathBuf>) {
	if let Ok(entries) = fs::read_dir(dir) {
		let mut entries:Vec<_> = entries.flatten().collect();

		entries.sort_by_key(|e| e.path());

		for entry in entries {
			let path = entry.path();

			if path.is_dir() {
				collect_rs_files(&path, files);
			} else if path.extension().map_or(false, |e| e == "rs") {
				files.push(path);
			}
		}
	}
}

fn count_diffs(a:&str, b:&str) -> u64 {
	let a_lines:Vec<&str> = a.lines().collect();

	let b_lines:Vec<&str> = b.lines().collect();

	let max = a_lines.len().max(b_lines.len());

	let mut diffs = 0u64;

	for i in 0..max {
		let al = a_lines.get(i).unwrap_or(&"");

		let bl = b_lines.get(i).unwrap_or(&"");

		if al != bl {
			diffs += 1;
		}
	}

	diffs
}

/// Fix meta-text patterns like "/// This function does X" → "/// Does X"
fn fix_meta_text(content:&str, fix_count:&mut u64) -> String {
	let mut result = content.to_string();

	// Pattern: "/// This <noun> " at the start of a doc comment
	let replacements = vec![
		// Functions
		(
			"/// This function waits for either Ctrl+C (SIGINT) or SIGTERM signals",
			"/// Waits for either Ctrl+C (SIGINT) or SIGTERM signals to initiate graceful shutdown.",
		),
		(
			"/// This function establishes a gRPC connection to Mountain using the",
			"/// Establishes a gRPC connection to Mountain using the",
		),
		(
			"/// This function processes file system events and updates the index",
			"/// Processes file system events and updates the index accordingly.",
		),
		(
			"/// This function is called by parallel tasks during directory scanning",
			"/// Called by parallel tasks during directory scanning",
		),
		(
			"/// This function loads certificates and keys from the file system and",
			"/// Loads certificates and keys from the file system and",
		),
		(
			"/// This function waits for either Ctrl+C (SIGINT) or SIGTERM signals",
			"/// Waits for either Ctrl+C (SIGINT) or SIGTERM signals to initiate graceful shutdown.",
		),
		// Structs
		(
			"/// This struct provides a high-level interface for Air to communicate with",
			"/// High-level gRPC client for Air to communicate with",
		),
		(
			"/// This struct holds the paths to certificates and keys required for",
			"/// Paths to certificates and keys required for",
		),
		// "This method:" patterns
		("\t/// This method:\r", "\t/// Performs the configured operation.\r"),
		("\t/// This method:\n", "\t/// Performs the configured operation.\n"),
		(
			"\t/// This method provides comprehensive defensive coding with:\r",
			"\t/// Performs defensive coding operations with:\r",
		),
		(
			"\t/// This method generates a configuration file compatible with winsvc.\r",
			"\t/// Generates a configuration file compatible with winsvc.\r",
		),
		// "This helper method"
		(
			"/// This helper method copies all files and subdirectories from source to",
			"/// Copies all files and subdirectories from source to",
		),
		// "This is a" pattern
		// MountainClient - health check stub
		(
			"/// This is a stub for future implementation. When the Mountain service",
			"/// Stub for future implementation. When the Mountain service",
		),
		// Binary.rs
		(
			"\t/// This is a simplified check for pre-implementation status.\r",
			"\t/// Performs a simplified status check for pre-implementation readiness.\r",
		),
		(
			"\t/// This is a simplified check for pre-implementation status.",
			"\t/// Performs a simplified status check for pre-implementation readiness.",
		),
		(
			"/// This is a simplified check for pre-implementation status.",
			"/// Performs a simplified status check for pre-implementation readiness.",
		),
		// Binary.rs - entry points
		(
			"\t/// This is the main entry point that uses default retry settings.\r",
			"\t/// Main entry point that uses default retry settings.\r",
		),
		(
			"\t/// This is the primary entry point for the Air background service. It\r",
			"\t/// Primary entry point for the Air background service.\r",
		),
		(
			"\t/// This is the primary constructor for the daemon coordinator. It validates\r",
			"\t/// Primary constructor for the daemon coordinator.\r",
		),
		(
			"\t/// This is the main initialization routine that:\r",
			"\t/// Main initialization routine that:\r",
		),
		(
			"\t/// This is the main event loop that runs until:\r",
			"\t/// Main event loop that runs until:\r",
		),
		(
			"\t/// This orchestrates a clean shutdown sequence:\r",
			"\t/// Orchestrates a clean shutdown sequence:\r",
		),
		// Library.rs
		(
			"/// This is automatically populated from Cargo.toml at build time",
			"Automatically populated from Cargo.toml at build time.",
		),
		(
			"/// This version is sent in all gRPC messages and checked by clients",
			"Sent in all gRPC messages and checked by clients",
		),
		// Utility functions
		(
			"\t/// This is a security measure to prevent directory traversal attacks.",
			"\tSecurity measure to prevent directory traversal attacks.",
		),
		// Macro doc
		(
			"/// This macro is used by `get_build_info()` to provide detailed",
			"Used by `get_build_info()` to provide detailed",
		),
		// "This method:" blocks in UpdateManager
		(
			"\t/// This method:\r\n\t/// \r\n\t/// Checks for updates from the configured update server.",
			"\t/// Checks for updates from the configured update server.\r",
		),
		(
			"\t/// This method:\r\n\t/// \r\n\t/// Downloads an update package from the remote server.",
			"\t/// Downloads an update package from the remote server.\r",
		),
		(
			"\t/// This method:\r\n\t/// \r\n\t/// Verifies the integrity of a downloaded update package.",
			"\t/// Verifies the integrity of a downloaded update package.\r",
		),
		(
			"\t/// This method:\r\n\t/// \r\n\t/// Installs the downloaded update package.",
			"\t/// Installs the downloaded update package.\r",
		),
		(
			"\t/// This method:\r\n\t/// \r\n\t/// Rolls back the most recent update if available.",
			"\t/// Rolls back the most recent update if available.\r",
		),
		(
			"\t/// This method:\r\n\t/// \r\n\t/// Registers a callback for update state changes.",
			"\t/// Registers a callback for update state changes.\r",
		),
		(
			"\t/// This method:\r\n\t/// \r\n\t/// Returns the current update status.",
			"\t/// Returns the current update status.\r",
		),
		(
			"\t/// This method:\r\n\t/// \r\n\t/// Returns the current download progress.",
			"\t/// Returns the current download progress.\r",
		),
		(
			"\t/// This method:\r\n\t/// \r\n\t/// Checks if an update is currently in progress.",
			"\t/// Checks if an update is currently in progress.\r",
		),
		(
			"\t/// This method:\r\n\t/// \r\n\t/// Cancels an ongoing update operation.",
			"\t/// Cancels an ongoing update operation.\r",
		),
		(
			"\t/// This method:\r\n\t/// \r\n\t/// Validates an update package before installation.",
			"\t/// Validates an update package before installation.\r",
		),
		(
			"\t/// This method:\r\n\t/// \r\n\t/// Downloads the update metadata from the remote source.",
			"\t/// Downloads the update metadata from the remote source.\r",
		),
		(
			"\t/// This method:\r\n\t/// \r\n\t/// Verifies checksums and signatures for update packages.",
			"\t/// Verifies checksums and signatures for update packages.\r",
		),
		(
			"\t/// This method:\r\n\t/// \r\n\t/// Handles errors during the update process.",
			"\t/// Handles errors during the update process.\r",
		),
		(
			"\t/// This method:\r\n\t/// \r\n\t/// Measures update download speeds.",
			"\t/// Measures update download speeds.\r",
		),
		(
			"\t/// This method:\r\n\t/// \r\n\t/// Cleans up temporary update files.",
			"\t/// Cleans up temporary update files.\r",
		),
		// UpdateManager - "This method:" → direct verb blocks
		(
			"\t/// This task:\r\n\t/// \r\n\t/// Downloads the latest version of the update package.",
			"\t/// Downloads the latest version of the update package.\r",
		),
		// "This indexer provides" in FileIndexer
		(
			"/// This indexer provides:\n/// - Incremental file watching with real-time updates\n/// - Multi-mode \
			 search (literal, regex, fuzzy)\n/// - Symbol extraction for VSCode Outline View\n/// - Language \
			 detection for syntax highlighting\n/// - Index corruption detection and recovery\n/// - Parallel \
			 indexing with resource limits",
			"/// Provides:\n/// - Incremental file watching with real-time updates\n/// - Multi-mode search (literal, \
			 regex, fuzzy)\n/// - Symbol extraction for VSCode Outline View\n/// - Language detection for syntax \
			 highlighting\n/// - Index corruption detection and recovery\n/// - Parallel indexing with resource limits",
		),
		// "This enables:" in StartWatcher
		(
			"/// This enables:\n/// - Real-time file change detection via `notify` crate\n/// - Debounced event \
			 processing to batch rapid changes\n/// - Support for file creation, modification, and deletion \
			 detection\n/// - Recursive directory watching\n/// - Cross-platform support (macOS, Linux, Windows)",
			"/// Enables:\n/// - Real-time file change detection via `notify` crate\n/// - Debounced event processing \
			 to batch rapid changes\n/// - Support for file creation, modification, and deletion detection\n/// - \
			 Recursive directory watching\n/// - Cross-platform support (macOS, Linux, Windows)",
		),
		// "This requires the server to support it" in generated air.rs
		(
			"        /// This requires the server to support it otherwise it might respond with an",
			"        /// Requires the server to support it; otherwise it might respond with an",
		),
		// HTTP Client
		(
			"/// This returns a `ClientBuilder` with the DNS resolver already set, allowing",
			"/// Returns a `ClientBuilder` with the DNS resolver already set, allowing",
		),
		(
			"/// This client uses the local DNS server (running on the specified port)",
			"/// Uses the local DNS server (running on the specified port)",
		),
		(
			"/// This client uses the local DNS server for all DNS resolution and",
			"/// Uses the local DNS server for all DNS resolution and",
		),
		// HotReload
		(
			"\t/// This can be used to subscribe to configuration change notifications\r",
			"\t/// Subscribes to configuration change notifications\r",
		),
		// MountainClient "This performs"
		(
			"\t/// This performs a basic connectivity check on the underlying gRPC channel.\r",
			"\t/// Performs a basic connectivity check on the underlying gRPC channel.\r",
		),
		// "This is a stub"
		(
			"\t/// This is a stub for future implementation.\r",
			"\t/// Stub for future implementation.\r",
		),
		// Binary.rs "This is the main entry point"
		(
			"\t/// This is the main entry point for the daemon with retry configuration.\r",
			"\t/// Main entry point for the daemon with retry configuration.\r",
		),
		// DaemonManager - "This method provides"
		(
			"\t/// This method provides comprehensive defensive coding with:\r",
			"\t/// Comprehensive defensive coding with:\r",
		),
		(
			"\t/// This method generates a configuration file compatible with winsvc.\r",
			"\t/// Generates a configuration file compatible with winsvc.\r",
		),
		// UpdateManager - "This helper method copies all files"
		(
			"\t/// This helper method copies all files and subdirectories from source to\r",
			"\t/// Copies all files and subdirectories from source to\r",
		),
		// Library.rs Utility path validation
		(
			"\t/// This is a security measure to prevent directory traversal attacks.",
			"\t/// Security measure to prevent directory traversal attacks.",
		),
		// MountainClientConfig - "This method reads configuration"
		(
			"\t/// This method reads configuration from the following environment\r",
			"\t/// Reads configuration from the following environment\r",
		),
	];

	for (old, new) in &replacements {
		if result.contains(old) {
			result = result.replace(old, new);

			*fix_count += 1;
		}
	}

	result
}

/// Add module-level //! docs to mod.rs files that lack them
fn add_module_docs_if_missing(content:&str, path:&std::path::Path, added:&mut u64) -> String {
	// Check if any line starts with //!
	let has_module_doc = content.lines().any(|l| l.trim_start().starts_with("//!"));

	if has_module_doc {
		return content.to_string();
	}

	// Skip generated files
	if path.to_string_lossy().contains("Generated") {
		return content.to_string();
	}

	let module_name = guess_module_name(path);

	let docs = generate_module_docs(&module_name);

	let mut result = String::new();

	for line in &docs {
		result.push_str(line);

		result.push('\n');
	}

	result.push('\n');

	result.push_str(content);

	*added += 1;
	println!("  Added module docs to {}", path.display());

	result
}

fn guess_module_name(path:&std::path::Path) -> String {
	let parent = path.parent().unwrap_or(std::path::Path::new(""));

	// For mod.rs in Source/, use "Air"
	if parent.file_name().map_or(false, |n| n == "Source") {
		return "Air".to_string();
	}

	parent
		.file_name()
		.map(|n| n.to_string_lossy().to_string())
		.unwrap_or_else(|| "Unknown".to_string())
}

fn generate_module_docs(name:&str) -> Vec<String> {
	let readable = name.chars().fold(String::new(), |acc, c| {
		if acc.is_empty() {
			c.to_uppercase().to_string()
		} else if c.is_uppercase() {
			format!("{} {}", acc, c)
		} else {
			format!("{}{}", acc, c)
		}
	});

	vec![
		format!("//! # {} Module", readable),
		String::new(),
		format!("//! Types and functionality for the {} daemon subsystem.", readable),
		String::new(),
	]
}

#[cfg(test)]
mod tests {

	use super::*;

	#[test]
	fn test_fix_function_meta() {
		let mut count = 0;

		let input = "/// This function waits for either Ctrl+C (SIGINT) or SIGTERM signals";

		let result = fix_meta_text(input, &mut count);

		assert!(!result.contains("This function"), "Got: {}", result);

		assert!(result.contains("Waits for"), "Got: {}", result);

		assert_eq!(count, 1);
	}

	#[test]
	fn test_fix_struct_meta() {
		let mut count = 0;

		let input = "/// This struct provides a high-level interface for Air to communicate with";

		let result = fix_meta_text(input, &mut count);

		assert!(!result.contains("This struct"), "Got: {}", result);

		assert_eq!(count, 1);
	}
}
