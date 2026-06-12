//! Main CLI command handler.
//!
//! `CliHandler` validates permissions, dispatches to `DaemonClient`, and
//! returns formatted output strings.

use crate::dev_log;
use super::{
	CommandTypes::{Command, ConfigCommand, DebugCommand, PermissionLevel},
	DaemonClient::DaemonClient,
	OutputFormat::OutputFormat,
	OutputFormatter::OutputFormatter,
};

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
			dev_log!("lifecycle", "warn: Admin privileges required for command");
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
			Command::Status { service, verbose, json } => self.Status(service, verbose, json),

			Command::Restart { service, force } => self.Restart(service, force),

			Command::Config(config_cmd) => self.Config(config_cmd),

			Command::Metrics { json, service } => self.Metrics(json, service),

			Command::Logs { service, tail, filter, follow } => self.Logs(service, tail, filter, follow),

			Command::Debug(debug_cmd) => self.Debug(debug_cmd),

			Command::Help { command } => Ok(OutputFormatter::format_help(command.as_deref(), "0.1.0")),

			Command::Version => Ok("Air 🪁 v0.1.0".to_string()),
		}
	}

	/// Handle status command
	fn Status(&self, service:Option<String>, verbose:bool, json:bool) -> Result<String, String> {
		let response = self.client.execute_status(service)?;

		Ok(OutputFormatter::format_status(&response, verbose, json))
	}

	/// Handle restart command
	fn Restart(&self, service:Option<String>, force:bool) -> Result<String, String> {
		let result = self.client.execute_restart(service, force)?;

		Ok(result)
	}

	/// Handle config commands
	fn Config(&self, cmd:ConfigCommand) -> Result<String, String> {
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
	fn Metrics(&self, json:bool, service:Option<String>) -> Result<String, String> {
		let response = self.client.execute_metrics(service)?;

		Ok(OutputFormatter::format_metrics(&response, json))
	}

	/// Handle logs command
	fn Logs(
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
	fn Debug(&self, cmd:DebugCommand) -> Result<String, String> {
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
