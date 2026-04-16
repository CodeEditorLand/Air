//! # ParseArguments
//!
//! ## File: Initialize/Command/ParseArguments.rs
//!
//! ## Role in Air Architecture
//!
//! Parses command-line arguments into daemon configuration or CLI commands. The parser
//! handles two modes: (1) CLI mode for commands like status/restart, and (2) daemon
//! mode for starting the background service with optional config/bind arguments.
//!
//! ## Primary Responsibility
//!
//! Parse command-line arguments and determine execution mode.
//!
//! ## Secondary Responsibilities
//!
//! - Validate argument count and length
//! - Detect CLI commands vs daemon arguments
//! - Provide clear error messages for invalid input
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `std` - Command-line argument access
//!
//! **Internal Modules:**
//! - `AirLibrary::CLI::CliParser` - CLI command parser
//! - `AirLibrary::CLI::Command` - Command enum
//!
//! ## Dependents
//!
//! - `Initialize::Binary::Binary` - Determines boot mode based on parsed args
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's CLI parsing in
//! `src/vs/code/node/cli.ts`
//!
//! ## Security Considerations
//!
//! - Argument length limits prevent DoS
//! - Path validation prevents directory traversal
//! - Null character detection prevents injection
//!
//! ## Performance Considerations
//!
//! - Fast parsing during daemon startup
//! - Minimal memory allocation
//!
//! ## Error Handling Strategy
//!
//! - Invalid arguments exit with helpful error
//! - Unknown flags are logged but ignored
//! - Partial inputs are rejected completely

use AirLibrary::CLI::{CliParser, Command};

use crate::dev_log;

/// Parsed command-line arguments
///
/// Contains the results of parsing command-line arguments.
pub struct ParsedArguments {
    /// Optional path to configuration file
    pub config_path: Option<String>,
    /// Optional bind address for gRPC server
    pub bind_address: Option<String>,
    /// Optional CLI command to execute (skips daemon mode)
    pub command: Option<Command>,
}

/// Parse command-line arguments into daemon config or CLI command
///
/// Handles two modes of operation:
/// 1. CLI mode: Execute commands like `status`, `restart`, `config`, etc.
/// 2. Daemon mode: Start the background service with optional config/bind args
///
/// # Returns
///
/// Returns a `ParsedArguments` struct containing parsed configuration and
/// optional command. If `command` is Some, daemon startup should be skipped.
///
/// # CLI Commands
///
/// Recognized commands:
/// - `status` - Show daemon status
//! - `restart` - Restart daemon or service
//! - `config` - Configuration management
//! - `metrics` - Show performance metrics
//! - `logs` - View log files
//! - `debug` - Debug utilities
//! - `help` - Show help information
//! - `version` - Show version info
//!
//! # Daemon Arguments
//!
//! - `--config` / `-c` - Path to configuration file
//! - `--bind` / `-b` - Bind address for gRPC server
//!
//! # Examples
//!
//! ```bash
//! # CLI mode
//! Air status              # Show daemon status
//! Air version             # Show version
//! Air config get log.level # Get config value
//!
//! # Daemon mode
//! Air --daemon            # Start with defaults
//! Air --config /path/to/Air.toml
//! Air --bind 0.0.0.0:50053
//! ```
//!
//! # FUTURE Enhancements
//! - Add `--validate-config` flag
//! - Add `--daemon` flag for explicit daemon mode
//! - Make flags case-insensitive

pub fn ParseArguments() -> ParsedArguments {
    // Defensive: Ensure args collection is not extremely large
    let args: Vec<String> = std::env::args().collect();
    
    // Safety: Limit argument length to prevent potential DoS
    if args.len() > 1024 {
        eprintln!("[ERROR] Too many command line arguments (max: 1024)");
        std::process::exit(1);
    }
    
    // Safety: Validate each argument length
    for (i, arg) in args.iter().enumerate() {
        if arg.len() > 4096 {
            eprintln!("[ERROR] Argument at position {} is too long (max: 4096 characters)", i);
            std::process::exit(1);
        }
    }
    
    dev_log!("lifecycle", "parsing command-line arguments ({} args)", args.len());
    
    // Check if we're running with CLI command (first arg is a known command)
    if args.len() > 1 {
        match args[1].as_str() {
            "status" | "restart" | "config" | "metrics" | "logs" | "debug" |
            "help" | "version" | "-h" | "--help" | "-v" | "--version" => {
                // Parse CLI command with error handling
                match CliParser::parse(args.clone()) {
                    Ok(cmd) => {
                        dev_log!("lifecycle", "CLI command parsed: {:?}", cmd);
                        return ParsedArguments {
                            config_path: None,
                            bind_address: None,
                            command: Some(cmd),
                        };
                    }
                    Err(e) => {
                        eprintln!("[ERROR] Error parsing CLI command: {}", e);
                        eprintln!("[ERROR] Run 'Air help' for usage information");
                        std::process::exit(1);
                    }
                }
            }
            _ => {}
        }
    }
    
    // Parse as daemon arguments with validation
    let mut config_path: Option<String> = None;
    let mut bind_address: Option<String> = None;
    
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--config" | "-c" => {
                if i + 1 < args.len() {
                    let path = &args[i + 1];
                    // Validate path doesn't contain suspicious characters
                    if path.contains("..") || path.contains('\0') {
                        eprintln!("[ERROR] Invalid config path: contains '..' or null character");
                        std::process::exit(1);
                    }
                    config_path = Some(path.clone());
                    dev_log!("lifecycle", "config path: {}", path);
                    i += 1;
                } else {
                    eprintln!("[ERROR] --config flag requires a path argument");
                    std::process::exit(1);
                }
            },
            "--bind" | "-b" => {
                if i + 1 < args.len() {
                    let addr = &args[i + 1];
                    // Basic validation of address format
                    if addr.is_empty() || addr.len() > 256 {
                        eprintln!("[ERROR] Invalid bind address: must be 1-256 characters");
                        std::process::exit(1);
                    }
                    // Full validation happens during bind, but check for null characters
                    if addr.contains('\0') {
                        eprintln!("[ERROR] Invalid bind address: contains null character");
                        std::process::exit(1);
                    }
                    bind_address = Some(addr.clone());
                    dev_log!("lifecycle", "bind address: {}", addr);
                    i += 1;
                } else {
                    eprintln!("[ERROR] --bind flag requires an address argument");
                    std::process::exit(1);
                }
            },
            _ => {
                // Ignore unknown flags or positional arguments
                // Could add warning for unknown flags if desired
            }
        }
        i += 1;
    }
    
    dev_log!("lifecycle", "daemon mode - config: {:?}, bind: {:?}", config_path, bind_address);
    
    ParsedArguments {
        config_path,
        bind_address,
        command: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_arguments_default() {
        let args = vec!["Air".to_string()];
        let parsed = ParsedArguments();
        assert!(parsed.command.is_none());
    }
}
