//! CLI output format helpers.
//!
//! `OutputFormatter` renders typed response structs as human-readable or
//! JSON text. The `HELP_*` constants supply the help screens dispatched
//! by `format_help`.

use super::ResponseTypes::{MetricsResponse, ServiceHealth, StatusResponse};

// =============================================================================
// Help Messages
// =============================================================================

pub const HELP_MAIN:&str = r#"
Air 🪁 - Background Daemon for Land Code Editor
Version: {version}

USAGE:
    Air [COMMAND] [OPTIONS]

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
    Air status --verbose
    Air config get grpc.bind_address
    Air metrics --json
    Air logs --tail=100 --follow
    Air debug health-check

Use 'Air help <command>' for more information about a command.
"#;

pub const HELP_STATUS:&str = r#"
Show daemon and service status

USAGE:
    Air status [OPTIONS]

OPTIONS:
    -s, --service <NAME>    Show status of specific service
    -v, --verbose           Show detailed information
    --json                  Output in JSON format

EXAMPLES:
    Air status
    Air status --service authentication --verbose
    Air status --json
"#;

pub const HELP_RESTART:&str = r#"
Restart services

USAGE:
    Air restart [OPTIONS]

OPTIONS:
    -s, --service <NAME>    Restart specific service
    -f, --force             Force restart without graceful shutdown

EXAMPLES:
    Air restart
    Air restart --service updates
    Air restart --force
"#;

pub const HELP_CONFIG:&str = r#"
Manage configuration

USAGE:
    Air config <SUBCOMMAND> [OPTIONS]

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
    Air config get grpc.bind_address
    Air config set updates.auto_download true
    Air config reload --validate
    Air config show --json
"#;

pub const HELP_METRICS:&str = r#"
View performance metrics

USAGE:
    Air metrics [OPTIONS]

OPTIONS:
    -s, --service <NAME>    Show metrics for specific service
    --json                  Output in JSON format

EXAMPLES:
    Air metrics
    Air metrics --service downloader
    Air metrics --json
"#;

pub const HELP_LOGS:&str = r#"
View daemon logs

USAGE:
    Air logs [OPTIONS]

OPTIONS:
    -s, --service <NAME>    Show logs from specific service
    -n, --tail <N>          Show last N lines (default: 50)

    -f, --filter <PATTERN>  Filter logs by pattern
    --follow                Follow logs in real-time

EXAMPLES:
    Air logs
    Air logs --service updates --tail=100
    Air logs --filter "ERROR" --follow
"#;

pub const HELP_DEBUG:&str = r#"
Debug and diagnostics

USAGE:
    Air debug <SUBCOMMAND> [OPTIONS]

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
    Air debug dump-state
    Air debug dump-connections --json
    Air debug health-check --verbose
    Air debug diagnostics --full
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
					"Unknown help topic: {}\n\nUse 'Air help' for general help.",
					topic.unwrap_or("unknown")
				)
			},
		}
	}
}
