//! # Configuration Management
//!
//! This module provides comprehensive configuration management for the Air
//! daemon, serving as the central configuration layer for the entire Land
//! ecosystem.
//!
//! ## Responsibilities
//!
//! - **Configuration Loading**: Load and parse configuration from TOML files
//!   with fallback to defaults
//! - **Schema Validation**: Validate all configuration values against defined
//!   schemas with detailed error messages
//! - **Type Safety**: Strong typing with compile-time guarantees and runtime
//!   validation
//! - **Value Constraints**: Range validation, path validation, and security
//!   checks
//! - **Environment Integration**: Support environment variable overrides and
//!   profile-based configuration
//! - **Hot Reload**: Live configuration updates without service restart (via
//!   HotReload module)
//! - **Change Tracking**: Audit trail for all configuration changes with
//!   rollback support
//! - **Migration Support**: Automated configuration schema versioning and
//!   migration
//!
//! ## VSCode Configuration System References
//!
//! This configuration system is designed to be compatible with VSCode's
//! configuration architecture:
//! - VSCode config reference:
//!   `Dependency/Microsoft/Editor/src/vs/platform/configuration/`
//! - Format compatibility with `settings.json` schema structure
//! - Support for workspace-specific overrides similar to VSCode's multi-layer
//!   config
//! - Configuration inheritance and overriding patterns aligned with VSCode
//!
//! ## Connection to Mountain's Configuration Needs
//!
//! Mountain (the VSCode application layer) consumes Air's configuration:
//! - User settings in Mountain flow through to Air's daemon configuration
//! - Wind services read centralized configuration for consistency
//! - Configuration changes propagate through the hot-reload system to all
//!   services
//! - Profile switching (dev/staging/prod) affects entire Land ecosystem
//!
//! ## Configuration Flow
//!
//! ```text
//! Mountain (User Settings) → Air config file → Wind services
//! ↓ ↓ ↓
//! settings.json ~/.Air/config.toml Service-specific overrides
//! ↓ ↓ ↓
//! Workspace settings Environment variables Hot-reload notifications
//! ```
//!
//! ## FUTURE: Schema Validation
//! - Implement JSON Schema generation for validation
//! - Add schema versioning and migration support
//! - Provide schema validation errors with detailed field-level information
//! - Support schema evolution with backward compatibility
//!
//! ## FUTURE: Configuration Migration
//! - Add version field to configuration structure
//! - Implement automatic migration between schema versions
//! - Provide migration tools for manual upgrades
//! - Document migration paths and breaking changes
//!
//! ## FUTURE: Configuration Inheritance
//! - Implement base profile templates
//! - Support profile inheritance and overrides
//! - Add configuration layer merging logic
//! - Document precedence rules (defaults → file → env → runtime)
//!
//! ## Profiles and Environments
//!
//! Configuration supports multiple profiles for different deployment scenarios:
//! - **dev**: Development environment with debug logging
//! - **staging**: Pre-production with production-like settings
//! - **prod**: Production optimized settings
//! - **custom**: User-defined profiles
//!
//! ## Security Considerations
//!
//! - Path validation prevents directory traversal attacks
//! - Sensitive values support environment variable injection
//! - Configuration files enforce proper permissions
//! - Atomic updates prevent partial/corrupted state

pub mod AirConfiguration;
pub mod ConfigurationManager;
pub mod HotReload;
pub mod Schema;

// Re-export commonly-used sub-types so downstream code doesn't break.
// Struct names match their module names (Rust convention).
pub use AirConfiguration::{AirConfiguration, gRPCConfig, AuthConfig, UpdateConfig, DownloadConfig, IndexingConfig, LoggingConfig, PerformanceConfig};

#[cfg(test)]
mod Tests;
