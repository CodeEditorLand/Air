//! CLI argument parser with full validation.
//!
//! `CliParser` converts raw `Vec<String>` args into a typed `Command`,
//! validating each sub-command's flags and values inline.

use super::CommandTypes::{Command, ConfigCommand, DebugCommand, DiagnosticLevel};

/// CLI arguments parser with validation
pub struct CliParser {
	TimeoutSecs:u64,
}

impl CliParser {
	/// Create a new CLI parser with default timeout
	pub fn new() -> Self { Self { TimeoutSecs:30 } }

	/// Create a new CLI parser with custom timeout
	pub fn with_timeout(TimeoutSecs:u64) -> Self { Self { TimeoutSecs } }

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
					"Unknown command: {}\n\nUse 'Air help' for available commands.",
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
				"config requires a subcommand: get, set, reload, show, validate\n\nUse 'Air help config' for more \
				 information."
					.to_string(),
			);
		}

		let subcommand = &args[0];

		match subcommand.as_str() {
			"get" => {
				if args.len() < 2 {
					return Err("config get requires a key\n\nExample: Air config get grpc.BindAddress".to_string());
				}

				let key = args[1].clone();

				Self::validate_config_key(&key)?;

				Ok(Command::Config(ConfigCommand::Get { key }))
			},

			"set" => {
				if args.len() < 3 {
					return Err("config set requires key and value\n\nExample: Air config set grpc.BindAddress \
					            \"[::1]:50053\""
						.to_string());
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
				"debug requires a subcommand: dump-state, dump-connections, health-check, diagnostics\n\nUse 'Air \
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
