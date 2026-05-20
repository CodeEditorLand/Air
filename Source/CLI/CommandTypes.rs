#![allow(non_snake_case, unused_variables, dead_code, unused_imports)]

//! CLI command enum hierarchy for the Air daemon CLI.
//!
//! `Command` is the top-level dispatch value produced by `CliParser::parse`.
//! Sub-enums (`ConfigCommand`, `DebugCommand`) scope arguments to logical
//! domains. Auxiliary enums (`DiagnosticLevel`, `ValidationResult`,
//! `PermissionLevel`) are referenced by parser and handler logic.

/// Top-level CLI command.
#[derive(Debug, Clone)]
pub enum Command {
	Status { service:Option<String>, verbose:bool, json:bool },
	Restart { service:Option<String>, force:bool },
	Config(ConfigCommand),
	Metrics { json:bool, service:Option<String> },
	Logs { service:Option<String>, tail:Option<usize>, filter:Option<String>, follow:bool },
	Debug(DebugCommand),
	Help { command:Option<String> },
	Version,
}

/// Configuration management sub-commands.
#[derive(Debug, Clone)]
pub enum ConfigCommand {
	Get { key:String },
	Set { key:String, value:String },
	Reload { validate:bool },
	Show { json:bool },
	Validate { path:Option<String> },
}

/// Debug and diagnostic sub-commands.
#[derive(Debug, Clone)]
pub enum DebugCommand {
	DumpState { service:Option<String>, json:bool },
	DumpConnections { format:Option<String> },
	HealthCheck { verbose:bool, service:Option<String> },
	Diagnostics { level:DiagnosticLevel },
}

/// Verbosity level for diagnostic operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
	Basic,
	Extended,
	Full,
}

/// Result of argument validation for a parsed command.
#[derive(Debug, Clone)]
pub enum ValidationResult {
	Valid,
	Invalid(String),
}

/// Minimum privilege required to execute a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionLevel {
	User,
	Admin,
}
