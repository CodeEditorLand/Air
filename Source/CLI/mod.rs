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
//! ## FUTURE Enhancements
//!
//! - **Plugin Marketplace Integration**: Add commands for discovering,
//! installing, and managing plugins from a central marketplace (similar to
//! `code --install-extension`)
//! - **Hot Reload Support**: Implement hot reload of configuration and plugins
//! without daemon restart
//! - **Sandboxing Mode**: Add a sandboxed mode for running commands with
//! restricted permissions
//! - **Interactive Shell**: Implement an interactive shell mode for continuous
//! daemon interaction
//! - **Completion Scripts**: Generate shell completion scripts (bash, zsh,
//! fish) for better UX
//! - **Profile Management**: Support multiple configuration profiles for
//! different environments
//! - **Remote Management**: Add support for managing remote Air instances via
//! SSH/IPC
//! - **Audit Logging**: Add comprehensive audit logging for all administrative
//!   actions
//!
//! ## Security Considerations
//!
//! - Admin commands (restart, config set) require elevated privileges
//! - Daemon communication uses secure IPC channels
//! - Sensitive information is masked in logs and error messages
//! - Timeouts prevent hanging on unresponsive daemon

pub mod CliHandler;

pub mod CliParser;

pub mod CommandTypes;

pub mod DaemonClient;

pub mod OutputFormat;

pub mod OutputFormatter;

pub mod ResponseTypes;

#[cfg(test)]
mod Tests;
