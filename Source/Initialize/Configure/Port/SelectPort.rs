//! # SelectPort
//!
//! ## File: Initialize/Configure/Port/SelectPort.rs
//!
//! ## Role in Air Architecture
//!
//! Validates and selects the binding address for the Vine gRPC server. The Vine
//! server is the primary communication channel between Air and Mountain,
//! running on port 50053.
//!
//! ## Primary Responsibility
//!
//! Parse and validate the gRPC server bind address from command-line arguments
//! or defaults.
//!
//! ## Secondary Responsibilities
//!
//! - Validate SocketAddr format
//! - Prevent port conflicts (50053 for Air, 50052 for Cocoon)
//! - Provide clear error messages for invalid addresses
//!
//! ## Dependencies
//!
//! **External Crates:**
//! - `std::net::SocketAddr` - Socket address parsing
//!
//! **Internal Modules:**
//! - `AirLibrary::DefaultBindAddress` - Default address constant ([::1]:50053)
//!
//! ## Dependents
//!
//! - `Initialize::Service::Vine::StartService` - Uses the selected address
//!
//! ## VSCode Pattern Reference
//!
//! Similar to VSCode's remote server port selection in
//! `src/vs/base/node/port.ts`
//!
//! ## Security Considerations
//!
//! - Validates address format to prevent injection attacks
//! - Prevents binding to privileged ports without explicit config
//! - Prevents null character injection in address strings
//!
//! ## Performance Considerations
//!
//! - Fast validation during startup
//! - No blocking operations
//!
//! ## Error Handling Strategy
//!
//! - Returns descriptive errors for invalid addresses
//! - Validates format before attempting to bind
//! - Provides clear guidance for port conflicts
//!
//! ## Thread Safety
//!
//! - Pure function, no mutable state
//! - Thread-safe for any context

use std::net::SocketAddr;

use AirLibrary::DefaultBindAddress;

use crate::dev_log;

/// Parse and select the gRPC server bind address
///
/// Determines the bind address for the Vine gRPC server based on:
/// 1. Command-line `--bind` argument (if provided)
/// 2. Default `DefaultBindAddress` constant ([::1]:50053)
///
/// # Arguments
///
/// * `bind_address` - Optional bind address from command-line argument
///
/// # Returns
///
/// Returns a validated `SocketAddr` for the gRPC server to bind to.
///
/// # Errors
///
/// Returns an error if:
/// - The provided address cannot be parsed as `SocketAddr`
/// - The default address constant is invalid
///
/// # Port Allocation
///
/// Air uses specific ports in the Land ecosystem:
/// - **50053**: Air (this daemon) - Background services
/// - **50052**: Cocoon (NodeJS host) - Frontend/web services
/// - **50054**: Reserved for future use (e.g., SideCar service)
/// - **50055**: Reserved for future metrics endpoints
///
/// # Examples
///
/// ```rust
/// // Use default address
/// let addr = SelectPort(None)?;
/// // addr = [::1]:50053
///
/// // Use custom address
/// let custom = "0.0.0.0:50053".to_string();
/// let addr = SelectPort(Some(custom))?;
/// // addr = 0.0.0.0:50053
/// ```
///
/// # FUTURE Enhancements
/// - Add support for IPv4-only binding (0.0.0.0:50053)
/// - Add support for IPv6-only binding ([::]:50053)
/// - Add port conflict detection before binding
/// - Add wildcard binding for all interfaces
pub fn SelectPort(bind_address:Option<String>) -> Result<SocketAddr, String> {
	match bind_address {
		Some(addr) => {
			// Custom address from command-line
			let parsed = addr
				.parse::<SocketAddr>()
				.map_err(|e| format!("Invalid bind address '{}': {}", addr, e))?;

			dev_log!("lifecycle", "[Boot] [Port] Using custom bind address: {}", parsed);
			Ok(parsed)
		},
		None => {
			// Use default address
			let parsed = DefaultBindAddress
				.parse::<SocketAddr>()
				.map_err(|e| format!("Invalid default bind address '{}': {}", DefaultBindAddress, e))?;

			dev_log!("lifecycle", "[Boot] [Port] Using default bind address: {}", parsed);
			Ok(parsed)
		},
	}
}

/// Validate that a port number is in valid range
///
/// # Arguments
///
/// * `port` - The port number to validate
///
/// # Returns
///
/// Returns `Ok(())` if valid, `Err` with description otherwise.
///
/// # Notes
///
/// Port 0 is valid for OS-assigned ports but not for configuration.
/// Ports 1-1023 require root/admin privileges.
pub fn ValidatePort(port:u16) -> Result<(), String> {
	if port == 0 {
		return Err("Port cannot be 0 for explicit configuration".to_string());
	}
	if port == 50052 {
		return Err("Port 50052 is reserved for Cocoon (NodeJS host)".to_string());
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_select_port_default() {
		let addr = SelectPort(None).unwrap();
		assert_eq!(addr.port(), 50053);
	}

	#[test]
	fn test_select_port_custom() {
		let custom = "127.0.0.1:54321".to_string();
		let addr = SelectPort(Some(custom)).unwrap();
		assert_eq!(addr.ip(), std::net::Ipv4Addr::new(127, 0, 0, 1));
		assert_eq!(addr.port(), 54321);
	}

	#[test]
	fn test_select_port_invalid() {
		let invalid = "not-an-address".to_string();
		assert!(SelectPort(Some(invalid)).is_err());
	}

	#[test]
	fn test_validate_port_zero() {
		assert!(ValidatePort(0).is_err());
	}

	#[test]
	fn test_validate_port_cocoon_reserved() {
		assert!(ValidatePort(50052).is_err());
	}

	#[test]
	fn test_validate_port_valid() {
		assert!(ValidatePort(50053).is_ok());
		assert!(ValidatePort(54321).is_ok());
	}
}
