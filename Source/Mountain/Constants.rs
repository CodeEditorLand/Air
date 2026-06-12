//! Mountain connection constants.
//!
//! Default addresses, timeouts, and protocol configuration for gRPC
//! communication between Air and Mountain.

/// Default Vine server address for Mountain component.
///
/// Port Allocation:
/// - 50051: Mountain Vine server (this target)
/// - 50052: Cocoon Vine server
/// - 50053: Air Vine server
pub const DEFAULT_MOUNTAIN_ADDRESS:&str = "[::1]:50051";

/// Default connection timeout in seconds
pub const DEFAULT_CONNECTION_TIMEOUT_SECS:u64 = 5;

/// Default request timeout in seconds
pub const DEFAULT_REQUEST_TIMEOUT_SECS:u64 = 30;
