//! # CLI - Command Line Interface
//!
//! Provides comprehensive command-line interface for Air daemon operations.
//! Handles argument parsing, command routing, and output formatting.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// =============================================================================
// Command Types
// =============================================================================

/// Main CLI command enum
#[derive(Debug, Clone)]
pub enum Command {
    /// Status command - check daemon and service status
    Status {
        service: Option<String>,
        verbose: bool,
        json: bool,
    },
    /// Restart command - restart services
    Restart {
        service: Option<String>,
        force: bool,
    },
    /// Configuration commands
    Config(ConfigCommand),
    /// Metrics command - retrieve performance metrics
    Metrics {
        json: bool,
        service: Option<String>,
    },
    /// Logs command - view daemon logs
    Logs {
        service: Option<String>,
        tail: Option<usize>,
        filter: Option<String>,
        follow: bool,
    },
    /// Debug commands
    Debug(DebugCommand),
    /// Help command
    Help {
        command: Option<String>,
    },
    /// Version command
    Version,
}

/// Configuration subcommands
#[derive(Debug, Clone)]
pub enum ConfigCommand {
    /// Get configuration value
    Get {
        key: String,
    },
    /// Set configuration value
    Set {
        key: String,
        value: String,
    },
    /// Reload configuration from file
    Reload {
        validate: bool,
    },
    /// Show current configuration
    Show {
        json: bool,
    },
    /// Validate configuration
    Validate {
        path: Option<String>,
    },
}

/// Debug subcommands
#[derive(Debug, Clone)]
pub enum DebugCommand {
    /// Dump current daemon state
    DumpState {
        service: Option<String>,
        json: bool,
    },
    /// Dump active connections
    DumpConnections {
        format: Option<String>,
    },
    /// Perform health check
    HealthCheck {
        verbose: bool,
        service: Option<String>,
    },
    /// Advanced diagnostics
    Diagnostics {
        level: DiagnosticLevel,
    },
}

/// Diagnostic level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Basic,
    Extended,
    Full,
}

// =============================================================================
// CLI Arguments Parsing
// =============================================================================

/// CLI arguments parser
pub struct CliParser;

impl CliParser {
    /// Parse command line arguments into Command
    pub fn parse(args: Vec<String>) -> Result<Command, String> {
        // Remove program name
        let args = if args.is_empty() { vec![] } else { args[1..].to_vec() };
        
        if args.is_empty() {
            return Ok(Command::Help { command: None });
        }
        
        let command = &args[0];
        
        match command.as_str() {
            "status" => Self::parse_status(&args[1..]),
            "restart" => Self::parse_restart(&args[1..]),
            "config" => Self::parse_config(&args[1..]),
            "metrics" => Self::parse_metrics(&args[1..]),
            "logs" => Self::parse_logs(&args[1..]),
            "debug" => Self::parse_debug(&args[1..]),
            "help" | "-h" | "--help" => Self::parse_help(&args[1..]),
            "version" | "-v" | "--version" => Ok(Command::Version),
            _ => Err(format!("Unknown command: {}", command)),
        }
    }
    
    /// Parse status command
    fn parse_status(args: &[String]) -> Result<Command, String> {
        let mut service = None;
        let mut verbose = false;
        let mut json = false;
        
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--service" => {
                    if i + 1 < args.len() {
                        service = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        return Err("--service requires a value".to_string());
                    }
                }
                "-s" => {
                    if i + 1 < args.len() {
                        service = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        return Err("-s requires a value".to_string());
                    }
                }
                "--verbose" | "-v" => {
                    verbose = true;
                    i += 1;
                }
                "--json" => {
                    json = true;
                    i += 1;
                }
                _ => {
                    return Err(format!("Unknown flag: {}", args[i]));
                }
            }
        }
        
        Ok(Command::Status {
            service,
            verbose,
            json,
        })
    }
    
    /// Parse restart command
    fn parse_restart(args: &[String]) -> Result<Command, String> {
        let mut service = None;
        let mut force = false;
        
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--service" | "-s" => {
                    if i + 1 < args.len() {
                        service = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        return Err("--service requires a value".to_string());
                    }
                }
                "--force" | "-f" => {
                    force = true;
                    i += 1;
                }
                _ => {
                    return Err(format!("Unknown flag: {}", args[i]));
                }
            }
        }
        
        Ok(Command::Restart { service, force })
    }
    
    /// Parse config subcommand
    fn parse_config(args: &[String]) -> Result<Command, String> {
        if args.is_empty() {
            return Err("config requires a subcommand: get, set, reload, show, validate".to_string());
        }
        
        let subcommand = &args[0];
        
        match subcommand.as_str() {
            "get" => {
                if args.len() < 2 {
                    return Err("config get requires a key".to_string());
                }
                Ok(Command::Config(ConfigCommand::Get {
                    key: args[1].clone(),
                }))
            }
            "set" => {
                if args.len() < 3 {
                    return Err("config set requires key and value".to_string());
                }
                Ok(Command::Config(ConfigCommand::Set {
                    key: args[1].clone(),
                    value: args[2].clone(),
                }))
            }
            "reload" => {
                let validate = args.contains(&"--validate".to_string());
                Ok(Command::Config(ConfigCommand::Reload { validate }))
            }
            "show" => {
                let json = args.contains(&"--json".to_string());
                Ok(Command::Config(ConfigCommand::Show { json }))
            }
            "validate" => {
                let path = args.get(1).cloned();
                Ok(Command::Config(ConfigCommand::Validate { path }))
            }
            _ => Err(format!("Unknown config subcommand: {}", subcommand)),
        }
    }
    
    /// Parse metrics command
    fn parse_metrics(args: &[String]) -> Result<Command, String> {
        let mut json = false;
        let mut service = None;
        
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--json" => {
                    json = true;
                    i += 1;
                }
                "--service" | "-s" => {
                    if i + 1 < args.len() {
                        service = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        return Err("--service requires a value".to_string());
                    }
                }
                _ => {
                    return Err(format!("Unknown flag: {}", args[i]));
                }
            }
        }
        
        Ok(Command::Metrics { json, service })
    }
    
    /// Parse logs command
    fn parse_logs(args: &[String]) -> Result<Command, String> {
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
                        i += 2;
                    } else {
                        return Err("--service requires a value".to_string());
                    }
                }
                "--tail" | "-n" => {
                    if i + 1 < args.len() {
                        tail = Some(args[i + 1].parse::<usize>()
                            .map_err(|_| format!("Invalid tail value: {}", args[i + 1]))?);
                        i += 2;
                    } else {
                        return Err("--tail requires a value".to_string());
                    }
                }
                "--filter" | "-f" => {
                    if i + 1 < args.len() {
                        filter = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        return Err("--filter requires a value".to_string());
                    }
                }
                "--follow" => {
                    follow = true;
                    i += 1;
                }
                _ => {
                    return Err(format!("Unknown flag: {}", args[i]));
                }
            }
        }
        
        Ok(Command::Logs {
            service,
            tail,
            filter,
            follow,
        })
    }
    
    /// Parse debug subcommand
    fn parse_debug(args: &[String]) -> Result<Command, String> {
        if args.is_empty() {
            return Err("debug requires a subcommand: dump-state, dump-connections, health-check, diagnostics".to_string());
        }
        
        let subcommand = &args[0];
        
        match subcommand.as_str() {
            "dump-state" => {
                let mut service = None;
                let mut json = false;
                
                for arg in &args[1..] {
                    match arg.as_str() {
                        "--service" | "-s" => {
                            // Would need to parse next argument
                        }
                        "--json" => {
                            json = true;
                        }
                        _ => {}
                    }
                }
                
                Ok(Command::Debug(DebugCommand::DumpState { service, json }))
            }
            "dump-connections" => {
                let format = args.get(1).map(|s| s.clone());
                Ok(Command::Debug(DebugCommand::DumpConnections { format }))
            }
            "health-check" => {
                let verbose = args.contains(&"--verbose".to_string());
                let mut service = None;
                
                for arg in args {
                    if arg.starts_with("--service=") {
                        service = Some(arg.split('=').nth(1).unwrap_or("").to_string());
                    }
                }
                
                Ok(Command::Debug(DebugCommand::HealthCheck { verbose, service }))
            }
            "diagnostics" => {
                let level = if args.contains(&"--full".to_string()) {
                    DiagnosticLevel::Full
                } else if args.contains(&"--extended".to_string()) {
                    DiagnosticLevel::Extended
                } else {
                    DiagnosticLevel::Basic
                };
                
                Ok(Command::Debug(DebugCommand::Diagnostics { level }))
            }
            _ => Err(format!("Unknown debug subcommand: {}", subcommand)),
        }
    }
    
    /// Parse help command
    fn parse_help(args: &[String]) -> Result<Command, String> {
        let command = args.get(0).map(|s| s.clone());
        Ok(Command::Help { command })
    }
}

// =============================================================================
// Response Structures
// =============================================================================

/// Status response
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub daemon_running: bool,
    pub uptime_secs: u64,
    pub version: String,
    pub services: HashMap<String, ServiceStatus>,
    pub timestamp: String,
}

/// Service status entry
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub name: String,
    pub running: bool,
    pub health: ServiceHealth,
    pub uptime_secs: u64,
    pub error: Option<String>,
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
    pub timestamp: String,
    pub memory_used_mb: f64,
    pub memory_available_mb: f64,
    pub cpu_usage_percent: f64,
    pub disk_used_mb: u64,
    pub disk_available_mb: u64,
    pub active_connections: u32,
    pub processed_requests: u64,
    pub failed_requests: u64,
    pub service_metrics: HashMap<String, ServiceMetrics>,
}

/// Service metrics entry
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceMetrics {
    pub name: String,
    pub requests_total: u64,
    pub requests_success: u64,
    pub requests_failed: u64,
    pub average_latency_ms: f64,
    pub p99_latency_ms: f64,
}

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub overall_healthy: bool,
    pub overall_health_percentage: f64,
    pub services: HashMap<String, ServiceHealthDetail>,
    pub timestamp: String,
}

/// Detailed service health
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceHealthDetail {
    pub name: String,
    pub healthy: bool,
    pub response_time_ms: u64,
    pub last_check: String,
    pub details: String,
}

/// Configuration response
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub key: Option<String>,
    pub value: serde_json::Value,
    pub path: String,
    pub modified: String,
}

// =============================================================================
// Help Messages
// =============================================================================

pub const HELP_MAIN: &str = r#"
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

pub const HELP_STATUS: &str = r#"
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

pub const HELP_RESTART: &str = r#"
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

pub const HELP_CONFIG: &str = r#"
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

pub const HELP_METRICS: &str = r#"
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

pub const HELP_LOGS: &str = r#"
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

pub const HELP_DEBUG: &str = r#"
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
    pub fn format_status(response: &StatusResponse, verbose: bool, json: bool) -> String {
        if json {
            serde_json::to_string_pretty(response).unwrap_or_else(|_| "{}".to_string())
        } else if verbose {
            Self::format_status_verbose(response)
        } else {
            Self::format_status_compact(response)
        }
    }
    
    fn format_status_compact(response: &StatusResponse) -> String {
        let daemon_status = if response.daemon_running { "🟢 Running" } else { "🔴 Stopped" };
        
        let mut output = format!(
            "Air Daemon {}\nVersion: {}\nUptime: {}s\n\nServices:\n",
            daemon_status,
            response.version,
            response.uptime_secs
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
    
    fn format_status_verbose(response: &StatusResponse) -> String {
        let mut output = format!(
            "╔════════════════════════════════════════╗\n\
             ║ Air Daemon Status\n\
             ╠════════════════════════════════════════╣\n\
             ║ Status:   {}\n\
             ║ Version:  {}\n\
             ║ Uptime:   {} seconds\n\
             ║ Time:     {}\n\
             ╠════════════════════════════════════════╣\n",
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
                "║   • {} ({})\n\
                 ║     Status: {}\n\
                 ║     Health: {}\n\
                 ║     Uptime: {} seconds\n",
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
    pub fn format_metrics(response: &MetricsResponse, json: bool) -> String {
        if json {
            serde_json::to_string_pretty(response).unwrap_or_else(|_| "{}".to_string())
        } else {
            Self::format_metrics_human(response)
        }
    }
    
    fn format_metrics_human(response: &MetricsResponse) -> String {
        format!(
            "╔════════════════════════════════════════╗\n\
             ║ Air Daemon Metrics\n\
             ╠════════════════════════════════════════╣\n\
             ║ Memory:     {:.1}MB / {:.1}MB\n\
             ║ CPU:        {:.1}%\n\
             ║ Disk:       {}MB / {}MB\n\
             ║ Connections: {}\n\
             ║ Requests:   {} success, {} failed\n\
             ╚════════════════════════════════════════╝\n",
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
    pub fn format_help(topic: Option<&str>, version: &str) -> String {
        match topic {
            None => HELP_MAIN.replace("{version}", version),
            Some("status") => HELP_STATUS.to_string(),
            Some("restart") => HELP_RESTART.to_string(),
            Some("config") => HELP_CONFIG.to_string(),
            Some("metrics") => HELP_METRICS.to_string(),
            Some("logs") => HELP_LOGS.to_string(),
            Some("debug") => HELP_DEBUG.to_string(),
            _ => format!("Unknown help topic: {}\n\nUse 'air help' for general help.", topic.unwrap_or("unknown")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_status_command() {
        let args = vec![
            "air".to_string(),
            "status".to_string(),
            "--verbose".to_string(),
        ];
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
