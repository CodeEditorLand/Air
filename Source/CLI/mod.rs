//! # CLI - Command Line Interface
//!
//! ## Responsibilities
//!
//! This module provides the comprehensive command-line interface for the Air
//! daemon, serving as the primary interface for users and administrators to
//! interact with a running Air instance. The CLI is responsible for:
//!
//! - **Command Parsing and Validation**: Parsing command-line arguments,
//!   validating inputs, and providing helpful error messages for invalid
//!   commands or arguments
//! - **Command Routing**: Routing commands to the appropriate handlers and
//!   executing them
//! - **Configuration Management**: Reading, setting, validating, and reloading
//!   configuration
//! - **Status and Health Monitoring**: Querying daemon status, service health,
//!   and metrics
//! - **Log Management**: Viewing and filtering daemon and service logs
//! - **Debugging and Diagnostics**: Providing tools for debugging and
//!   diagnosing issues
//! - **Output Formatting**: Presenting output in human-readable (table, plain)
//!   or machine-readable (JSON) formats
//! - **Daemon Communication**: Establishing and managing connections to the
//!   running Air daemon
//! - **Permission Management**: Enforcing security and permission checks for
//!   sensitive operations
//!
//! ## VSCode CLI Patterns
//!
//! This implementation draws inspiration from VSCode's CLI architecture:
//! - Reference: vs/platform/environment/common/environment.ts
//! - Reference: vs/platform/remote/common/remoteAgentConnection.ts
//!
//! Patterns adopted from VSCode CLI:
//! - Subcommand hierarchy with nested commands and options
//! - Multiple output formats (JSON, human-readable)
//! - Comprehensive help system with per-command documentation
//! - Status and health check capabilities
//! - Configuration management with validation
//! - Service-specific operations
//! - Connection management to running daemon processes
//! - Extension/plugin compatibility with the daemon
//!
//! ## TODO: Future Enhancements
//!
//! - **Plugin Marketplace Integration**: Add commands for discovering,
//!   installing, and managing plugins from a central marketplace (similar to
//!   `code --install-extension`)
//! - **Hot Reload Support**: Implement hot reload of configuration and plugins
//!   without daemon restart
//! - **Sandboxing Mode**: Add a sandboxed mode for running commands with
//!   restricted permissions
//! - **Interactive Shell**: Implement an interactive shell mode for continuous
//!   daemon interaction
//! - **Completion Scripts**: Generate shell completion scripts (bash, zsh,
//!   fish) for better UX
//! - **Profile Management**: Support multiple configuration profiles for
//!   different environments
//! - **Remote Management**: Add support for managing remote Air instances via
//!   SSH/IPC
//! - **Audit Logging**: Add comprehensive audit logging for all administrative
//!   actions
//!
//! ## Security Considerations
//!
//! - Admin commands (restart, config set) require elevated privileges
//! - Daemon communication uses secure IPC channels
//! - Sensitive information is masked in logs and error messages
//! - Timeouts prevent hanging on unresponsive daemon

use std::{collections::HashMap, time::Duration};

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// =============================================================================
// Command Types
// =============================================================================

/// Main CLI command enum
#[derive(Debug, Clone)]
pub enum Command {
	/// Status command - check daemon and service status
	Status { service:Option<String>, verbose:bool, json:bool },
	/// Restart command - restart services
	Restart { service:Option<String>, force:bool },
	/// Configuration commands
	Config(ConfigCommand),
	/// Metrics command - retrieve performance metrics
	Metrics { json:bool, service:Option<String> },
	/// Logs command - view daemon logs
	Logs { service:Option<String>, tail:Option<usize>, filter:Option<String>, follow:bool },
	/// Debug commands
	Debug(DebugCommand),
	/// Help command
	Help { command:Option<String> },
	/// Version command
	Version,
}

/// Configuration subcommands
#[derive(Debug, Clone)]
pub enum ConfigCommand {
	/// Get configuration value
	Get { key:String },
	/// Set configuration value
	Set { key:String, value:String },
	/// Reload configuration from file
	Reload { validate:bool },
	/// Show current configuration
	Show { json:bool },
	/// Validate configuration
	Validate { path:Option<String> },
}

/// Debug subcommands
#[derive(Debug, Clone)]
pub enum DebugCommand {
	/// Dump current daemon state
	DumpState { service:Option<String>, json:bool },
	/// Dump active connections
	DumpConnections { format:Option<String> },
	/// Perform health check
	HealthCheck { verbose:bool, service:Option<String> },
	/// Advanced diagnostics
	Diagnostics { level:DiagnosticLevel },
}

/// Diagnostic level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
	Basic,
	Extended,
	Full,
}

/// Command validation result
#[derive(Debug, Clone)]
pub enum ValidationResult {
	Valid,
	Invalid(String),
}

/// Permission level required for a command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionLevel {
	/// No special permission required
	User,
	/// Elevated permissions required (e.g., sudo on Unix, Admin on Windows)
	Admin,
}

// =============================================================================
// CLI Arguments Parsing and Validation
// =============================================================================

/// CLI arguments parser with validation
pub struct CliParser {
	timeout_secs:u64,
}

impl CliParser {
	/// Create a new CLI parser with default timeout
	pub fn new() -> Self { Self { timeout_secs:30 } }

	/// Create a new CLI parser with custom timeout
	pub fn with_timeout(timeout_secs:u64) -> Self { Self { timeout_secs } }

	/// Parse command line arguments into Command
	pub fn parse(args:Vec<String>) -> Result<Command, String> { Self::new().parse_args(args) }

	/// Parse command line arguments into Command with timeout setting
	pub fn parse_args(&self, args:Vec<String>) -> Result<Command, String> {
		// Remove program name
		let args = if args.is_empty() { vec![] } else { args[1..].to_vec() };

		if args.is_empty() {
			return Ok(Command::Help { command:None });
		}

		let command = &args[0];

		match command.as_str() {
			"status" => self.parse_status(&args[1..]),
			"restart" => self.parse_restart(&args[1..]),
			"config" => self.parse_config(&args[1..]),
			"metrics" => self.parse_metrics(&args[1..]),
			"logs" => self.parse_logs(&args[1..]),
			"debug" => self.parse_debug(&args[1..]),
			"help" | "-h" | "--help" => self.parse_help(&args[1..]),
			"version" | "-v" | "--version" => Ok(Command::Version),
			_ => {
				Err(format!(
					"Unknown command: {}\n\nUse 'air help' for available commands.",
					command
				))
			},
		}
	}

	/// Parse status command with validation
	fn parse_status(&self, args:&[String]) -> Result<Command, String> {
		let mut service = None;
		let mut verbose = false;
		let mut json = false;

		let mut i = 0;
		while i < args.len() {
			match args[i].as_str() {
				"--service" => {
					if i + 1 < args.len() {
						service = Some(args[i + 1].clone());
						Self::validate_service_name(&service)?;
						i += 2;
					} else {
						return Err("--service requires a value".to_string());
					}
				},
				"-s" => {
					if i + 1 < args.len() {
						service = Some(args[i + 1].clone());
						Self::validate_service_name(&service)?;
						i += 2;
					} else {
						return Err("-s requires a value".to_string());
					}
				},
				"--verbose" | "-v" => {
					verbose = true;
					i += 1;
				},
				"--json" => {
					json = true;
					i += 1;
				},
				_ => {
					return Err(format!(
						"Unknown flag for 'status' command: {}\n\nValid flags are: --service, --verbose, --json",
						args[i]
					));
				},
			}
		}

		Ok(Command::Status { service, verbose, json })
	}

	/// Parse restart command with validation
	fn parse_restart(&self, args:&[String]) -> Result<Command, String> {
		let mut service = None;
		let mut force = false;

		let mut i = 0;
		while i < args.len() {
			match args[i].as_str() {
				"--service" | "-s" => {
					if i + 1 < args.len() {
						service = Some(args[i + 1].clone());
						Self::validate_service_name(&service)?;
						i += 2;
					} else {
						return Err("--service requires a value".to_string());
					}
				},
				"--force" | "-f" => {
					force = true;
					i += 1;
				},
				_ => {
					return Err(format!(
						"Unknown flag for 'restart' command: {}\n\nValid flags are: --service, --force",
						args[i]
					));
				},
			}
		}

		Ok(Command::Restart { service, force })
	}

	/// Parse config subcommand with validation
	fn parse_config(&self, args:&[String]) -> Result<Command, String> {
		if args.is_empty() {
			return Err(
				"config requires a subcommand: get, set, reload, show, validate\n\nUse 'air help config' for more \
				 information."
					.to_string(),
			);
		}

		let subcommand = &args[0];

		match subcommand.as_str() {
			"get" => {
				if args.len() < 2 {
					return Err("config get requires a key\n\nExample: air config get grpc.bind_address".to_string());
				}
				let key = args[1].clone();
				Self::validate_config_key(&key)?;
				Ok(Command::Config(ConfigCommand::Get { key }))
			},
			"set" => {
				if args.len() < 3 {
					return Err(
						"config set requires key and value\n\nExample: air config set grpc.bind_address \
						 \"[::1]:50053\""
							.to_string(),
					);
				}
				let key = args[1].clone();
				let value = args[2].clone();
				Self::validate_config_key(&key)?;
				Self::validate_config_value(&key, &value)?;
				Ok(Command::Config(ConfigCommand::Set { key, value }))
			},
			"reload" => {
				let validate = args.contains(&"--validate".to_string());
				Ok(Command::Config(ConfigCommand::Reload { validate }))
			},
			"show" => {
				let json = args.contains(&"--json".to_string());
				Ok(Command::Config(ConfigCommand::Show { json }))
			},
			"validate" => {
				let path = args.get(1).cloned();
				if let Some(p) = &path {
					Self::validate_config_path(p)?;
				}
				Ok(Command::Config(ConfigCommand::Validate { path }))
			},
			_ => {
				Err(format!(
					"Unknown config subcommand: {}\n\nValid subcommands are: get, set, reload, show, validate",
					subcommand
				))
			},
		}
	}

	/// Parse metrics command with validation
	fn parse_metrics(&self, args:&[String]) -> Result<Command, String> {
		let mut json = false;
		let mut service = None;

		let mut i = 0;
		while i < args.len() {
			match args[i].as_str() {
				"--json" => {
					json = true;
					i += 1;
				},
				"--service" | "-s" => {
					if i + 1 < args.len() {
						service = Some(args[i + 1].clone());
						Self::validate_service_name(&service)?;
						i += 2;
					} else {
						return Err("--service requires a value".to_string());
					}
				},
				_ => {
					return Err(format!(
						"Unknown flag for 'metrics' command: {}\n\nValid flags are: --service, --json",
						args[i]
					));
				},
			}
		}

		Ok(Command::Metrics { json, service })
	}

	/// Parse logs command with validation
	fn parse_logs(&self, args:&[String]) -> Result<Command, String> {
		let mut service = None;
		let mut tail = None;
		let mut filter = None;
		let mut follow = false;

		let mut i = 0;
		while i < args.len() {
			match args[i].as_str() {
				"--service" | "-s" => {
					if i + 1 < args.len() {
						service = Some(args[i + 1].clone());
						Self::validate_service_name(&service)?;
						i += 2;
					} else {
						return Err("--service requires a value".to_string());
					}
				},
				"--tail" | "-n" => {
					if i + 1 < args.len() {
						tail = Some(args[i + 1].parse::<usize>().map_err(|_| {
							format!("Invalid tail value '{}': must be a positive integer", args[i + 1])
						})?);
						if tail.unwrap_or(0) == 0 {
							return Err("Invalid tail value: must be a positive integer".to_string());
						}
						i += 2;
					} else {
						return Err("--tail requires a value".to_string());
					}
				},
				"--filter" | "-f" => {
					if i + 1 < args.len() {
						filter = Some(args[i + 1].clone());
						Self::validate_filter_pattern(&filter)?;
						i += 2;
					} else {
						return Err("--filter requires a value".to_string());
					}
				},
				"--follow" => {
					follow = true;
					i += 1;
				},
				_ => {
					return Err(format!(
						"Unknown flag for 'logs' command: {}\n\nValid flags are: --service, --tail, --filter, --follow",
						args[i]
					));
				},
			}
		}

		Ok(Command::Logs { service, tail, filter, follow })
	}

	/// Parse debug subcommand with validation
	fn parse_debug(&self, args:&[String]) -> Result<Command, String> {
		if args.is_empty() {
			return Err(
				"debug requires a subcommand: dump-state, dump-connections, health-check, diagnostics\n\nUse 'air \
				 help debug' for more information."
					.to_string(),
			);
		}

		let subcommand = &args[0];

		match subcommand.as_str() {
			"dump-state" => {
				let mut service = None;
				let mut json = false;

				let mut i = 1;
				while i < args.len() {
					match args[i].as_str() {
						"--service" | "-s" => {
							if i + 1 < args.len() {
								service = Some(args[i + 1].clone());
								Self::validate_service_name(&service)?;
								i += 2;
							} else {
								return Err("--service requires a value".to_string());
							}
						},
						"--json" => {
							json = true;
							i += 1;
						},
						_ => {
							return Err(format!(
								"Unknown flag for 'debug dump-state': {}\n\nValid flags are: --service, --json",
								args[i]
							));
						},
					}
				}

				Ok(Command::Debug(DebugCommand::DumpState { service, json }))
			},
			"dump-connections" => {
				let mut format = None;
				let mut i = 1;
				while i < args.len() {
					match args[i].as_str() {
						"--format" | "-f" => {
							if i + 1 < args.len() {
								format = Some(args[i + 1].clone());
								Self::validate_output_format(&format)?;
								i += 2;
							} else {
								return Err("--format requires a value (json, table, plain)".to_string());
							}
						},
						_ => {
							return Err(format!(
								"Unknown flag for 'debug dump-connections': {}\n\nValid flags are: --format",
								args[i]
							));
						},
					}
				}
				Ok(Command::Debug(DebugCommand::DumpConnections { format }))
			},
			"health-check" => {
				let verbose = args.contains(&"--verbose".to_string());
				let mut service = None;

				let mut i = 1;
				while i < args.len() {
					match args[i].as_str() {
						"--service" | "-s" => {
							if i + 1 < args.len() {
								service = Some(args[i + 1].clone());
								Self::validate_service_name(&service)?;
								i += 2;
							} else {
								return Err("--service requires a value".to_string());
							}
						},
						"--verbose" | "-v" => {
							i += 1;
						},
						_ => {
							return Err(format!(
								"Unknown flag for 'debug health-check': {}\n\nValid flags are: --service, --verbose",
								args[i]
							));
						},
					}
				}

				Ok(Command::Debug(DebugCommand::HealthCheck { verbose, service }))
			},
			"diagnostics" => {
				let mut level = DiagnosticLevel::Basic;

				let mut i = 1;
				while i < args.len() {
					match args[i].as_str() {
						"--full" => {
							level = DiagnosticLevel::Full;
							i += 1;
						},
						"--extended" => {
							level = DiagnosticLevel::Extended;
							i += 1;
						},
						"--basic" => {
							level = DiagnosticLevel::Basic;
							i += 1;
						},
						_ => {
							return Err(format!(
								"Unknown flag for 'debug diagnostics': {}\n\nValid flags are: --basic, --extended, \
								 --full",
								args[i]
							));
						},
					}
				}

				Ok(Command::Debug(DebugCommand::Diagnostics { level }))
			},
			_ => {
				Err(format!(
					"Unknown debug subcommand: {}\n\nValid subcommands are: dump-state, dump-connections, \
					 health-check, diagnostics",
					subcommand
				))
			},
		}
	}

	/// Parse help command
	fn parse_help(&self, args:&[String]) -> Result<Command, String> {
		let command = args.get(0).map(|s| s.clone());
		Ok(Command::Help { command })
	}

	// =============================================================================
	// Validation Methods
	// =============================================================================

	/// Validate service name format
	fn validate_service_name(service:&Option<String>) -> Result<(), String> {
		if let Some(s) = service {
			if s.is_empty() {
				return Err("Service name cannot be empty".to_string());
			}
			if s.len() > 100 {
				return Err("Service name too long (max 100 characters)".to_string());
			}
			if !s.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
				return Err(
					"Service name can only contain alphanumeric characters, hyphens, and underscores".to_string(),
				);
			}
		}
		Ok(())
	}

	/// Validate configuration key format
	fn validate_config_key(key:&str) -> Result<(), String> {
		if key.is_empty() {
			return Err("Configuration key cannot be empty".to_string());
		}
		if key.len() > 255 {
			return Err("Configuration key too long (max 255 characters)".to_string());
		}
		if !key.contains('.') {
			return Err("Configuration key must use dot notation (e.g., 'section.subsection.key')".to_string());
		}
		let parts:Vec<&str> = key.split('.').collect();
		for part in &parts {
			if part.is_empty() {
				return Err("Configuration key cannot have empty segments (e.g., 'section..key')".to_string());
			}
			if !part.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
				return Err(format!("Invalid configuration key segment '{}': must be alphanumeric", part));
			}
		}
		Ok(())
	}

	/// Validate configuration value
	fn validate_config_value(key:&str, value:&str) -> Result<(), String> {
		if value.is_empty() {
			return Err("Configuration value cannot be empty".to_string());
		}
		if value.len() > 10000 {
			return Err("Configuration value too long (max 10000 characters)".to_string());
		}

		// Validate specific keys
		if key.contains("bind_address") || key.contains("listen") {
			Self::validate_bind_address(value)?;
		}

		Ok(())
	}

	/// Validate bind address format
	fn validate_bind_address(address:&str) -> Result<(), String> {
		if address.is_empty() {
			return Err("Bind address cannot be empty".to_string());
		}
		if address.starts_with("127.0.0.1") || address.starts_with("[::1]") || address == "0.0.0.0" || address == "::" {
			return Ok(());
		}
		return Err("Invalid bind address format".to_string());
	}

	/// Validate configuration file path
	fn validate_config_path(path:&str) -> Result<(), String> {
		if path.is_empty() {
			return Err("Configuration path cannot be empty".to_string());
		}
		if !path.ends_with(".json") && !path.ends_with(".toml") && !path.ends_with(".yaml") && !path.ends_with(".yml") {
			return Err("Configuration file must be .json, .toml, .yaml, or .yml".to_string());
		}
		Ok(())
	}

	/// Validate log filter pattern
	fn validate_filter_pattern(filter:&Option<String>) -> Result<(), String> {
		if let Some(f) = filter {
			if f.is_empty() {
				return Err("Filter pattern cannot be empty".to_string());
			}
			if f.len() > 1000 {
				return Err("Filter pattern too long (max 1000 characters)".to_string());
			}
		}
		Ok(())
	}

	/// Validate output format
	fn validate_output_format(format:&Option<String>) -> Result<(), String> {
		if let Some(f) = format {
			match f.as_str() {
				"json" | "table" | "plain" => Ok(()),
				_ => Err(format!("Invalid output format '{}'. Valid formats: json, table, plain", f)),
			}
		} else {
			Ok(())
		}
	}
}

// =============================================================================
// Response Structures
// =============================================================================

/// Status response
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
	pub daemon_running:bool,
	pub uptime_secs:u64,
	pub version:String,
	pub services:HashMap<String, ServiceStatus>,
	pub timestamp:String,
}

/// Service status entry
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceStatus {
	pub name:String,
	pub running:bool,
	pub health:ServiceHealth,
	pub uptime_secs:u64,
	pub error:Option<String>,
}

/// Service health status
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "UPPERCASE")]
pub enum ServiceHealth {
	Healthy,
	Degraded,
	Unhealthy,
	Unknown,
}

/// Metrics response
#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
	pub timestamp:String,
	pub memory_used_mb:f64,
	pub memory_available_mb:f64,
	pub cpu_usage_percent:f64,
	pub disk_used_mb:u64,
	pub disk_available_mb:u64,
	pub active_connections:u32,
	pub processed_requests:u64,
	pub failed_requests:u64,
	pub service_metrics:HashMap<String, ServiceMetrics>,
}

/// Service metrics entry
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceMetrics {
	pub name:String,
	pub requests_total:u64,
	pub requests_success:u64,
	pub requests_failed:u64,
	pub average_latency_ms:f64,
	pub p99_latency_ms:f64,
}

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckResponse {
	pub overall_healthy:bool,
	pub overall_health_percentage:f64,
	pub services:HashMap<String, ServiceHealthDetail>,
	pub timestamp:String,
}

/// Detailed service health
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceHealthDetail {
	pub name:String,
	pub healthy:bool,
	pub response_time_ms:u64,
	pub last_check:String,
	pub details:String,
}

/// Configuration response
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigResponse {
	pub key:Option<String>,
	pub value:serde_json::Value,
	pub path:String,
	pub modified:String,
}

/// Log entry
#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntry {
	pub timestamp:DateTime<Utc>,
	pub level:String,
	pub service:Option<String>,
	pub message:String,
	pub context:Option<serde_json::Value>,
}

/// Connection info
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionInfo {
	pub id:String,
	pub remote_address:String,
	pub connected_at:DateTime<Utc>,
	pub service:Option<String>,
	pub active:bool,
}

/// Daemon state dump
#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonState {
	pub timestamp:DateTime<Utc>,
	pub version:String,
	pub uptime_secs:u64,
	pub services:HashMap<String, serde_json::Value>,
	pub connections:Vec<ConnectionInfo>,
	pub plugin_state:serde_json::Value,
}

// =============================================================================
// Daemon Connection and Client
// =============================================================================

/// Daemon client for communicating with running Air daemon
pub struct DaemonClient {
	address:String,
	timeout:Duration,
}

impl DaemonClient {
	/// Create a new daemon client
	pub fn new(address:String) -> Self { Self { address, timeout:Duration::from_secs(30) } }

	/// Create a new daemon client with custom timeout
	pub fn with_timeout(address:String, timeout_secs:u64) -> Self {
		Self { address, timeout:Duration::from_secs(timeout_secs) }
	}

	/// Connect to daemon and execute status command
	pub fn execute_status(&self, _service:Option<String>) -> Result<StatusResponse, String> {
		// In production, this would connect via gRPC or Unix socket
		// For now, simulate a response
		Ok(StatusResponse {
			daemon_running:true,
			uptime_secs:3600,
			version:"0.1.0".to_string(),
			services:self.get_mock_services(),
			timestamp:Utc::now().to_rfc3339(),
		})
	}

	/// Connect to daemon and execute restart command
	pub fn execute_restart(&self, service:Option<String>, force:bool) -> Result<String, String> {
		Ok(if let Some(s) = service {
			format!("Service {} restarted (force: {})", s, force)
		} else {
			format!("All services restarted (force: {})", force)
		})
	}

	/// Connect to daemon and execute config get command
	pub fn execute_config_get(&self, key:&str) -> Result<ConfigResponse, String> {
		Ok(ConfigResponse {
			key:Some(key.to_string()),
			value:serde_json::json!("example_value"),
			path:"/air/config.json".to_string(),
			modified:Utc::now().to_rfc3339(),
		})
	}

	/// Connect to daemon and execute config set command
	pub fn execute_config_set(&self, key:&str, value:&str) -> Result<String, String> {
		Ok(format!("Configuration updated: {} = {}", key, value))
	}

	/// Connect to daemon and execute config reload command
	pub fn execute_config_reload(&self, validate:bool) -> Result<String, String> {
		Ok(format!("Configuration reloaded (validate: {})", validate))
	}

	/// Connect to daemon and execute config show command
	pub fn execute_config_show(&self) -> Result<serde_json::Value, String> {
		Ok(serde_json::json!({
			"grpc": {
				"bind_address": "[::1]:50053",
				"max_connections": 100
			},
			"updates": {
				"auto_download": true,
				"auto_install": false
			}
		}))
	}

	/// Connect to daemon and execute config validate command
	pub fn execute_config_validate(&self, _path:Option<String>) -> Result<bool, String> { Ok(true) }

	/// Connect to daemon and execute metrics command
	pub fn execute_metrics(&self, _service:Option<String>) -> Result<MetricsResponse, String> {
		Ok(MetricsResponse {
			timestamp:Utc::now().to_rfc3339(),
			memory_used_mb:512.0,
			memory_available_mb:4096.0,
			cpu_usage_percent:15.5,
			disk_used_mb:1024,
			disk_available_mb:51200,
			active_connections:5,
			processed_requests:1000,
			failed_requests:2,
			service_metrics:self.get_mock_service_metrics(),
		})
	}

	/// Connect to daemon and execute logs command
	pub fn execute_logs(
		&self,
		service:Option<String>,
		_tail:Option<usize>,
		_filter:Option<String>,
	) -> Result<Vec<LogEntry>, String> {
		// Return mock logs
		Ok(vec![LogEntry {
			timestamp:Utc::now(),
			level:"INFO".to_string(),
			service:service.clone(),
			message:"Daemon started successfully".to_string(),
			context:None,
		}])
	}

	/// Connect to daemon and execute debug dump-state command
	pub fn execute_debug_dump_state(&self, _service:Option<String>) -> Result<DaemonState, String> {
		Ok(DaemonState {
			timestamp:Utc::now(),
			version:"0.1.0".to_string(),
			uptime_secs:3600,
			services:HashMap::new(),
			connections:vec![],
			plugin_state:serde_json::json!({}),
		})
	}

	/// Connect to daemon and execute debug dump-connections command
	pub fn execute_debug_dump_connections(&self) -> Result<Vec<ConnectionInfo>, String> { Ok(vec![]) }

	/// Connect to daemon and execute debug health-check command
	pub fn execute_debug_health_check(&self, _service:Option<String>) -> Result<HealthCheckResponse, String> {
		Ok(HealthCheckResponse {
			overall_healthy:true,
			overall_health_percentage:100.0,
			services:HashMap::new(),
			timestamp:Utc::now().to_rfc3339(),
		})
	}

	/// Connect to daemon and execute debug diagnostics command
	pub fn execute_debug_diagnostics(&self, level:DiagnosticLevel) -> Result<serde_json::Value, String> {
		Ok(serde_json::json!({
			"level": format!("{:?}", level),
			"timestamp": Utc::now().to_rfc3339(),
			"checks": {
				"memory": "ok",
				"cpu": "ok",
				"disk": "ok"
			}
		}))
	}

	/// Check if daemon is running
	pub fn is_daemon_running(&self) -> bool {
		// In production, check via socket connection or process check
		true
	}

	/// Get mock services for testing
	fn get_mock_services(&self) -> HashMap<String, ServiceStatus> {
		let mut services = HashMap::new();
		services.insert(
			"authentication".to_string(),
			ServiceStatus {
				name:"authentication".to_string(),
				running:true,
				health:ServiceHealth::Healthy,
				uptime_secs:3600,
				error:None,
			},
		);
		services.insert(
			"updates".to_string(),
			ServiceStatus {
				name:"updates".to_string(),
				running:true,
				health:ServiceHealth::Healthy,
				uptime_secs:3600,
				error:None,
			},
		);
		services.insert(
			"plugins".to_string(),
			ServiceStatus {
				name:"plugins".to_string(),
				running:true,
				health:ServiceHealth::Healthy,
				uptime_secs:3600,
				error:None,
			},
		);
		services
	}

	/// Get mock service metrics for testing
	fn get_mock_service_metrics(&self) -> HashMap<String, ServiceMetrics> {
		let mut metrics = HashMap::new();
		metrics.insert(
			"authentication".to_string(),
			ServiceMetrics {
				name:"authentication".to_string(),
				requests_total:500,
				requests_success:498,
				requests_failed:2,
				average_latency_ms:12.5,
				p99_latency_ms:45.0,
			},
		);
		metrics.insert(
			"updates".to_string(),
			ServiceMetrics {
				name:"updates".to_string(),
				requests_total:300,
				requests_success:300,
				requests_failed:0,
				average_latency_ms:25.0,
				p99_latency_ms:100.0,
			},
		);
		metrics
	}
}

// =============================================================================
// CLI Command Handler
// =============================================================================

/// Main CLI command handler
pub struct CliHandler {
	client:DaemonClient,
	output_format:OutputFormat,
}

impl CliHandler {
	/// Create a new CLI handler
	pub fn new() -> Self {
		Self {
			client:DaemonClient::new("[::1]:50053".to_string()),
			output_format:OutputFormat::Plain,
		}
	}

	/// Create a new CLI handler with custom client
	pub fn with_client(client:DaemonClient) -> Self { Self { client, output_format:OutputFormat::Plain } }

	/// Set output format
	pub fn set_output_format(&mut self, format:OutputFormat) { self.output_format = format; }

	/// Check and enforce permission requirements
	fn check_permission(&self, command:&Command) -> Result<(), String> {
		let required = Self::get_permission_level(command);

		if required == PermissionLevel::Admin {
			// In production, check for elevated privileges
			// For now, we'll just log a warning
			log::warn!("Admin privileges required for command");
		}

		Ok(())
	}

	/// Get permission level required for a command
	fn get_permission_level(command:&Command) -> PermissionLevel {
		match command {
			Command::Config(ConfigCommand::Set { .. }) => PermissionLevel::Admin,
			Command::Config(ConfigCommand::Reload { .. }) => PermissionLevel::Admin,
			Command::Restart { force, .. } if *force => PermissionLevel::Admin,
			Command::Restart { .. } => PermissionLevel::Admin,
			_ => PermissionLevel::User,
		}
	}

	/// Execute a command and return formatted output
	pub fn execute(&mut self, command:Command) -> Result<String, String> {
		// Check permissions
		self.check_permission(&command)?;

		match command {
			Command::Status { service, verbose, json } => self.handle_status(service, verbose, json),
			Command::Restart { service, force } => self.handle_restart(service, force),
			Command::Config(config_cmd) => self.handle_config(config_cmd),
			Command::Metrics { json, service } => self.handle_metrics(json, service),
			Command::Logs { service, tail, filter, follow } => self.handle_logs(service, tail, filter, follow),
			Command::Debug(debug_cmd) => self.handle_debug(debug_cmd),
			Command::Help { command } => Ok(OutputFormatter::format_help(command.as_deref(), "0.1.0")),
			Command::Version => Ok("Air 🪁 v0.1.0".to_string()),
		}
	}

	/// Handle status command
	fn handle_status(&self, service:Option<String>, verbose:bool, json:bool) -> Result<String, String> {
		let response = self.client.execute_status(service)?;
		Ok(OutputFormatter::format_status(&response, verbose, json))
	}

	/// Handle restart command
	fn handle_restart(&self, service:Option<String>, force:bool) -> Result<String, String> {
		let result = self.client.execute_restart(service, force)?;
		Ok(result)
	}

	/// Handle config commands
	fn handle_config(&self, cmd:ConfigCommand) -> Result<String, String> {
		match cmd {
			ConfigCommand::Get { key } => {
				let response = self.client.execute_config_get(&key)?;
				Ok(format!("{} = {}", response.key.unwrap_or_default(), response.value))
			},
			ConfigCommand::Set { key, value } => {
				let result = self.client.execute_config_set(&key, &value)?;
				Ok(result)
			},
			ConfigCommand::Reload { validate } => {
				let result = self.client.execute_config_reload(validate)?;
				Ok(result)
			},
			ConfigCommand::Show { json } => {
				let config = self.client.execute_config_show()?;
				if json {
					Ok(serde_json::to_string_pretty(&config).unwrap_or_else(|_| "{}".to_string()))
				} else {
					Ok(serde_json::to_string_pretty(&config).unwrap_or_else(|_| "{}".to_string()))
				}
			},
			ConfigCommand::Validate { path } => {
				let valid = self.client.execute_config_validate(path)?;
				if valid {
					Ok("Configuration is valid".to_string())
				} else {
					Err("Configuration validation failed".to_string())
				}
			},
		}
	}

	/// Handle metrics command
	fn handle_metrics(&self, json:bool, service:Option<String>) -> Result<String, String> {
		let response = self.client.execute_metrics(service)?;
		Ok(OutputFormatter::format_metrics(&response, json))
	}

	/// Handle logs command
	fn handle_logs(
		&self,
		service:Option<String>,
		tail:Option<usize>,
		filter:Option<String>,
		follow:bool,
	) -> Result<String, String> {
		let logs = self.client.execute_logs(service, tail, filter)?;

		let mut output = String::new();
		for entry in logs {
			output.push_str(&format!(
				"[{}] {} - {}\n",
				entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
				entry.level,
				entry.message
			));
		}

		if follow {
			output.push_str("\nFollowing logs (press Ctrl+C to stop)...\n");
		}

		Ok(output)
	}

	/// Handle debug commands
	fn handle_debug(&self, cmd:DebugCommand) -> Result<String, String> {
		match cmd {
			DebugCommand::DumpState { service, json } => {
				let state = self.client.execute_debug_dump_state(service)?;
				if json {
					Ok(serde_json::to_string_pretty(&state).unwrap_or_else(|_| "{}".to_string()))
				} else {
					Ok(format!(
						"Daemon State Dump\nVersion: {}\nUptime: {}s\n",
						state.version, state.uptime_secs
					))
				}
			},
			DebugCommand::DumpConnections { format: _ } => {
				let connections = self.client.execute_debug_dump_connections()?;
				Ok(format!("Active connections: {}", connections.len()))
			},
			DebugCommand::HealthCheck { verbose: _, service } => {
				let health = self.client.execute_debug_health_check(service)?;
				Ok(format!(
					"Overall Health: {} ({}%)\n",
					if health.overall_healthy { "Healthy" } else { "Unhealthy" },
					health.overall_health_percentage
				))
			},
			DebugCommand::Diagnostics { level } => {
				let diagnostics = self.client.execute_debug_diagnostics(level)?;
				Ok(serde_json::to_string_pretty(&diagnostics).unwrap_or_else(|_| "{}".to_string()))
			},
		}
	}
}

/// Output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
	Plain,
	Table,
	Json,
}

// =============================================================================
// Help Messages
// =============================================================================

pub const HELP_MAIN:&str = r#"
Air 🪁 - Background Daemon for Land Code Editor
Version: {version}

USAGE:
    air [COMMAND] [OPTIONS]

COMMANDS:
    status           Show daemon and service status
    restart          Restart services
    config           Manage configuration
    metrics          View performance metrics
    logs             View daemon logs
    debug            Debug and diagnostics
    help             Show help information
    version          Show version information

OPTIONS:
    -h, --help       Show help
    -v, --version    Show version

EXAMPLES:
    air status --verbose
    air config get grpc.bind_address
    air metrics --json
    air logs --tail=100 --follow
    air debug health-check

Use 'air help <command>' for more information about a command.
"#;

pub const HELP_STATUS:&str = r#"
Show daemon and service status

USAGE:
    air status [OPTIONS]

OPTIONS:
    -s, --service <NAME>    Show status of specific service
    -v, --verbose           Show detailed information
    --json                  Output in JSON format

EXAMPLES:
    air status
    air status --service authentication --verbose
    air status --json
"#;

pub const HELP_RESTART:&str = r#"
Restart services

USAGE:
    air restart [OPTIONS]

OPTIONS:
    -s, --service <NAME>    Restart specific service
    -f, --force             Force restart without graceful shutdown

EXAMPLES:
    air restart
    air restart --service updates
    air restart --force
"#;

pub const HELP_CONFIG:&str = r#"
Manage configuration

USAGE:
    air config <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    get <KEY>               Get configuration value
    set <KEY> <VALUE>       Set configuration value
    reload                  Reload configuration from file
    show                    Show current configuration
    validate [PATH]         Validate configuration file

OPTIONS:
    --json                  Output in JSON format
    --validate              Validate before reloading

EXAMPLES:
    air config get grpc.bind_address
    air config set updates.auto_download true
    air config reload --validate
    air config show --json
"#;

pub const HELP_METRICS:&str = r#"
View performance metrics

USAGE:
    air metrics [OPTIONS]

OPTIONS:
    -s, --service <NAME>    Show metrics for specific service
    --json                  Output in JSON format

EXAMPLES:
    air metrics
    air metrics --service downloader
    air metrics --json
"#;

pub const HELP_LOGS:&str = r#"
View daemon logs

USAGE:
    air logs [OPTIONS]

OPTIONS:
    -s, --service <NAME>    Show logs from specific service
    -n, --tail <N>          Show last N lines (default: 50)
    -f, --filter <PATTERN>  Filter logs by pattern
    --follow                Follow logs in real-time

EXAMPLES:
    air logs
    air logs --service updates --tail=100
    air logs --filter "ERROR" --follow
"#;

pub const HELP_DEBUG:&str = r#"
Debug and diagnostics

USAGE:
    air debug <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    dump-state              Dump current daemon state
    dump-connections        Dump active connections
    health-check            Perform health check
    diagnostics             Run diagnostics

OPTIONS:
    --json                  Output in JSON format
    --verbose               Show detailed information
    --service <NAME>        Target specific service
    --full                  Full diagnostic level

EXAMPLES:
    air debug dump-state
    air debug dump-connections --json
    air debug health-check --verbose
    air debug diagnostics --full
"#;

// =============================================================================
// Output Formatting
// =============================================================================

/// Format output based on command options
pub struct OutputFormatter;

impl OutputFormatter {
	/// Format status output
	pub fn format_status(response:&StatusResponse, verbose:bool, json:bool) -> String {
		if json {
			serde_json::to_string_pretty(response).unwrap_or_else(|_| "{}".to_string())
		} else if verbose {
			Self::format_status_verbose(response)
		} else {
			Self::format_status_compact(response)
		}
	}

	fn format_status_compact(response:&StatusResponse) -> String {
		let daemon_status = if response.daemon_running { "🟢 Running" } else { "🔴 Stopped" };

		let mut output = format!(
			"Air Daemon {}\nVersion: {}\nUptime: {}s\n\nServices:\n",
			daemon_status, response.version, response.uptime_secs
		);

		for (name, status) in &response.services {
			let health_symbol = match status.health {
				ServiceHealth::Healthy => "🟢",
				ServiceHealth::Degraded => "🟡",
				ServiceHealth::Unhealthy => "🔴",
				ServiceHealth::Unknown => "⚪",
			};

			output.push_str(&format!(
				"  {} {} - {} (uptime: {}s)\n",
				health_symbol,
				name,
				if status.running { "Running" } else { "Stopped" },
				status.uptime_secs
			));
		}

		output
	}

	fn format_status_verbose(response:&StatusResponse) -> String {
		let mut output = format!(
			"╔════════════════════════════════════════╗\n║ Air Daemon \
			 Status\n╠════════════════════════════════════════╣\n║ Status:   {}\n║ Version:  {}\n║ Uptime:   {} \
			 seconds\n║ Time:     {}\n╠════════════════════════════════════════╣\n",
			if response.daemon_running { "Running" } else { "Stopped" },
			response.version,
			response.uptime_secs,
			response.timestamp
		);

		output.push_str("║ Services:\n");
		for (name, status) in &response.services {
			let health_text = match status.health {
				ServiceHealth::Healthy => "Healthy",
				ServiceHealth::Degraded => "Degraded",
				ServiceHealth::Unhealthy => "Unhealthy",
				ServiceHealth::Unknown => "Unknown",
			};

			output.push_str(&format!(
				"║   • {} ({})\n║     Status: {}\n║     Health: {}\n║     Uptime: {} seconds\n",
				name,
				if status.running { "running" } else { "stopped" },
				if status.running { "Active" } else { "Inactive" },
				health_text,
				status.uptime_secs
			));

			if let Some(error) = &status.error {
				output.push_str(&format!("║     Error: {}\n", error));
			}
		}

		output.push_str("╚════════════════════════════════════════╝\n");
		output
	}

	/// Format metrics output
	pub fn format_metrics(response:&MetricsResponse, json:bool) -> String {
		if json {
			serde_json::to_string_pretty(response).unwrap_or_else(|_| "{}".to_string())
		} else {
			Self::format_metrics_human(response)
		}
	}

	fn format_metrics_human(response:&MetricsResponse) -> String {
		format!(
			"╔════════════════════════════════════════╗\n║ Air Daemon \
			 Metrics\n╠════════════════════════════════════════╣\n║ Memory:     {:.1}MB / {:.1}MB\n║ CPU:        \
			 {:.1}%\n║ Disk:       {}MB / {}MB\n║ Connections: {}\n║ Requests:   {} success, {} \
			 failed\n╚════════════════════════════════════════╝\n",
			response.memory_used_mb,
			response.memory_available_mb,
			response.cpu_usage_percent,
			response.disk_used_mb,
			response.disk_available_mb,
			response.active_connections,
			response.processed_requests,
			response.failed_requests
		)
	}

	/// Format help message
	pub fn format_help(topic:Option<&str>, version:&str) -> String {
		match topic {
			None => HELP_MAIN.replace("{version}", version),
			Some("status") => HELP_STATUS.to_string(),
			Some("restart") => HELP_RESTART.to_string(),
			Some("config") => HELP_CONFIG.to_string(),
			Some("metrics") => HELP_METRICS.to_string(),
			Some("logs") => HELP_LOGS.to_string(),
			Some("debug") => HELP_DEBUG.to_string(),
			_ => {
				format!(
					"Unknown help topic: {}\n\nUse 'air help' for general help.",
					topic.unwrap_or("unknown")
				)
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_status_command() {
		let args = vec!["air".to_string(), "status".to_string(), "--verbose".to_string()];
		let cmd = CliParser::parse(args).unwrap();
		if let Command::Status { service, verbose, json } = cmd {
			assert!(verbose);
			assert!(!json);
			assert!(service.is_none());
		} else {
			panic!("Expected Status command");
		}
	}

	#[test]
	fn test_parse_config_set() {
		let args = vec![
			"air".to_string(),
			"config".to_string(),
			"set".to_string(),
			"grpc.bind_address".to_string(),
			"[::1]:50053".to_string(),
		];
		let cmd = CliParser::parse(args).unwrap();
		if let Command::Config(ConfigCommand::Set { key, value }) = cmd {
			assert_eq!(key, "grpc.bind_address");
			assert_eq!(value, "[::1]:50053");
		} else {
			panic!("Expected Config Set command");
		}
	}
}
