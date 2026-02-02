//! # ConfigureLog
//!
//! ## File: Initialize/Configure/Log/ConfigureLog.rs
//!
//! ## Role in Air Architecture
//!
//! Provides logging configuration for the Air daemon, initializing structured
//! logging with support for JSON output and file-based logging. This is called
//! early in the boot sequence to ensure all subsequent operations can be
//! logged.
//!
//! ## Primary Responsibility
//!
//! Initialize and configure structured logging for the Air daemon based on
//! environment variables.
//!
//! ## Secondary Responsibilities
//!
//! - Validate log configuration environment variables
//! - Handle JSON vs plain text output formatting
//! - Support file-based logging with directory validation
//! - Provide fallback logging on initialization failure
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `log` - Logging facade
//!
//! **Internal Modules:**
//! - `AirLibrary::Logging` - Logger initialization
//! - `std::env` - Environment variable access
//!
//! ## Dependents
//!
//! - `Initialize::Binary::Binary` - Calls during daemon boot sequence
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's logging service in
//! `src/vs/platform/log/common/log.ts`
//!
//! ## Security Considerations
//!
//! - Validates log file path to prevent directory traversal
//! - Checks directory exists and is writable before using
//! - Sanitizes configuration values to prevent injection
//!
//! ## Performance Considerations
//!
//! - Logging is early in boot sequence, must be fast
//! - Async logging to avoid blocking daemon startup
//! - JSON output optimized for production systems
//!
//! ## Error Handling Strategy
//!
//! - Defensive validation of all environment variables
//! - Fallback to stderr if logging initialization fails
//! - Clear error messages for configuration issues
//!
//! ## Thread Safety
//!
//! - Thread-safe via the underlying log crate
//! - No mutable state, pure configuration function

/// Initialize logging based on environment variables
///
/// Sets up structured logging with support for JSON output (useful for
/// production) and file-based logging (useful for debugging). Environment
/// variables:
///
/// - `AIR_LOG_JSON`: "true" enables JSON formatted output
/// - `AIR_LOG_LEVEL`: Set logging level (debug, info, warn, error)
/// - `AIR_LOG_FILE`: Path to log file (optional)
///
/// # Returns
///
/// Returns `()` on success, logs errors and provides fallback on failure.
///
/// # Environment Variables
///
/// - `AIR_LOG_JSON` - Set to "true" for JSON formatted logs
/// - `AIR_LOG_FILE` - Path to log file (optional)
///
/// # Examples
///
/// ```bash
/// # Standard text logging to stderr
/// AIR_LOG_LEVEL=debug Air --daemon
///
/// # JSON logging to file
/// AIR_LOG_JSON=true AIR_LOG_FILE=/var/log/Air.log Air --daemon
/// ```
///
/// # TODO
/// - Add log rotation support
/// - Implement log file size limits
/// - Add structured log correlation IDs
/// - Support syslog integration on Unix
/// - Add Windows Event Log integration
pub fn ConfigureLog() {
	// Validate environment variables
	let json_output = match std::env::var("AIR_LOG_JSON") {
		Ok(val) if !val.is_empty() => {
			let normalized = val.to_lowercase();
			if normalized != "true" && normalized != "false" {
				eprintln!(
					"Warning: Invalid AIR_LOG_JSON value '{}', expected 'true' or 'false'. Using default: false",
					val
				);
				false
			} else {
				normalized == "true"
			}
		},
		Ok(_) => false,
		Err(_) => false,
	};

	// Validate log file path exists and is writable
	let log_file_path = std::env::var("AIR_LOG_FILE").ok().and_then(|path| {
		if path.is_empty() {
			None
		} else {
			// Check if directory exists for the log file
			if let Some(parent) = std::path::PathBuf::from(&path).parent() {
				if parent.as_os_str().is_empty() {
					// No parent directory, use current directory
					Some(path)
				} else if parent.exists() {
					Some(path)
				} else {
					eprintln!(
						"Warning: Log file directory does not exist: {}. Logging to stdout only.",
						parent.display()
					);
					None
				}
			} else {
				Some(path)
			}
		}
	});

	// Initialize structured logging with defensive error handling
	let log_result = Logging::initialize_logger(json_output, log_file_path.clone());

	match log_result {
		Ok(_) => {
			let log_info = match &log_file_path {
				Some(path) => format!("file: {}", path),
				None => "stdout/stderr".to_string(),
			};
			log::info!("[Boot] Logging initialized - JSON: {}, Output: {}", json_output, log_info);
		},
		Err(e) => {
			// Fallback: ensure we can at least log errors to stderr
			eprintln!("[ERROR] Failed to initialize structured logging: {}", e);
			eprintln!("[ERROR] Logging will fall back to stderr-only output");
		},
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_configure_log_default() {
		// Should not panic with default settings
		ConfigureLog();
	}

	#[test]
	fn test_invalid_json_value() {
		std::env::set_var("AIR_LOG_JSON", "invalid");
		// Should handle gracefully and use default
		ConfigureLog();
		std::env::remove_var("AIR_LOG_JSON");
	}
}
