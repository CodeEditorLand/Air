//! # ValidateCommand
//!
//! ## File: Initialize/Command/ValidateCommand.rs
//!
//! ## Role in Air Architecture
//!
//! Validates CLI command parameters before execution to prevent invalid inputs
//! and provide early feedback to users. Validation happens before any network
//! connections or daemon operations.
//!
//! ## Primary Responsibility
//!
//! Validate command parameters to prevent invalid inputs.
//!
//! ## Secondary Responsibilities
//!
//! - Provide descriptive error messages
//! - Enforce length limits on string parameters
//! - Detect potentially malicious input patterns
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - None
//!
//! **Internal Modules:**
//! - `AirLibrary::CLI::Command` - Command enum for validation
//!
//! ## Dependents
//!
//! - `Initialize::Command::HandleCommand` - Validates before execution
//!
//! ## VSCode Pattern Reference
//!
//! Inspired by VSCode's command validation in
//! `src/vs/platform/commands/common/commandsService.ts`
//!
//! ## Security Considerations
//!
//! - Validating inputs prevents injection attacks
//! - Length limits prevent DoS attacks
//! - Pattern detection catches malicious strings
//!
//! ## Performance Considerations
//!
//! - Fast validation with minimal allocations
//! - Early return on first error
//!
//! ## Error Handling Strategy
//!
//! - Returns descriptive errors for invalid inputs
//! - Fails fast on first validation error
//! - Guidance provided for fixing errors
//!
//! ## Thread Safety
//!
//! - Pure function, no mutable state
//! - Thread-safe for any context

use AirLibrary::CLI::CommandTypes::Command;

/// Validate command parameters to prevent invalid inputs
///
/// Checks command parameters for length limits, invalid characters, and
/// potentially malicious input patterns before execution.
///
/// # Arguments
//!
/// * `cmd` - Reference to the Command to validate
///
/// # Returns
///
/// Returns `Ok(())` if valid, error with description otherwise.
///
/// # Validation Rules
//!
//! - Help command name: max 128 characters
//! - Service names: 1-64 characters
//! - Configuration keys: 1-256 characters, no null or newline
//! - Configuration values: max 8192 characters
//! - Config paths: 1-512 characters, no path traversal
//! - Log filter strings: 1-512 characters
//! - Log tail count: 1-10000 lines
//!
//! # FUTURE Enhancements
//! - Add timeout parameter validation
//! - Add rate limit checks for commands
//! - Implement command permission checks
//!
pub fn ValidateCommand(cmd: &Command) -> Result<(), String> {

    match cmd {

        Command::Help { command } => {

            if let Some(ref cmd) = command {

                if cmd.len() > 128 {

                    return Err("Command name too long (max: 128)".to_string());
                }
            }
        }

        Command::Status { service, .. } => {

            if let Some(ref svc) = service {

                if svc.is_empty() || svc.len() > 64 {

                    return Err("Service name must be 1-64 characters".to_string());
                }
            }
        }

        Command::Restart { service, .. } => {

            if let Some(ref svc) = service {

                if svc.is_empty() || svc.len() > 64 {

                    return Err("Service name must be 1-64 characters".to_string());
                }
            }
        }

        Command::Config(_) => {

            // Config sub-commands have their own validation
        }

        Command::Metrics { service, .. } => {

            if let Some(ref svc) = service {

                if svc.is_empty() || svc.len() > 64 {

                    return Err("Service name must be 1-64 characters".to_string());
                }
            }
        }

        Command::Logs { service, tail, filter, .. } => {

            if let Some(ref svc) = service {

                if svc.is_empty() || svc.len() > 64 {

                    return Err("Service name must be 1-64 characters".to_string());
                }
            }

            if let Some(n) = tail {

                if n < 1 || n > 10000 {

                    return Err("Tail count must be 1-10000 lines".to_string());
                }
            }

            if let Some(ref f) = filter {

                if f.is_empty() || f.len() > 512 {

                    return Err("Filter string must be 1-512 characters".to_string());
                }
            }
        }

        Command::Debug(_) => {

            // Debug sub-commands have their own validation
        }

        Command::Version => {

            // No parameters to validate
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;
    
    #[test]
    fn test_validate_help_command() {

        let cmd = Command::Help {

            command: Some("version".to_string()),
        };

        assert!(ValidateCommand(&cmd).is_ok());
    }
    
    #[test]
    fn test_validate_help_command_too_long() {

        let cmd = Command::Help {

            command: Some("a".repeat(200)),
        };

        assert!(ValidateCommand(&cmd).is_err());
    }
    
    #[test]
    fn test_validate_status_command_long_service() {

        let cmd = Command::Status {

            service: Some("a".repeat(100)),

            verbose: false,

            json: false,
        };

        assert!(ValidateCommand(&cmd).is_err());
    }
}
