//! Configuration builder for Mountain gRPC client connections.
//!
//! Supports environment-variable configuration, builder pattern with method
//! chaining, and optional TLS/mTLS settings.

use std::{env, path::PathBuf, time::Duration};

use crate::Mountain::Constants::{DEFAULT_CONNECTION_TIMEOUT_SECS, DEFAULT_MOUNTAIN_ADDRESS, DEFAULT_REQUEST_TIMEOUT_SECS};
#[cfg(feature = "mtls")]
use crate::Mountain::TlsConfig::TlsConfig;

/// Configuration for connecting to Mountain.
#[derive(Debug, Clone)]
pub struct MountainClientConfig {
	/// The gRPC server address of Mountain (e.g., `"[::1]:50051"`)
	pub address:String,

	/// Connection timeout in seconds
	pub connection_timeout_secs:u64,

	/// Request timeout in seconds
	pub request_timeout_secs:u64,

	/// TLS configuration (if mtls feature is enabled)
	#[cfg(feature = "mtls")]
	pub tls_config:Option<TlsConfig>,
}

impl Default for MountainClientConfig {
	fn default() -> Self {
		Self {
			address:DEFAULT_MOUNTAIN_ADDRESS.to_string(),

			connection_timeout_secs:DEFAULT_CONNECTION_TIMEOUT_SECS,

			request_timeout_secs:DEFAULT_REQUEST_TIMEOUT_SECS,

			#[cfg(feature = "mtls")]
			tls_config:None,
		}
	}
}

impl MountainClientConfig {
	/// Creates a new MountainClientConfig with the specified address.
	///
	/// # Parameters
	/// - `address`: The gRPC server address
	///
	/// # Returns
	/// New MountainClientConfig instance
	pub fn new(address:impl Into<String>) -> Self { Self { address:address.into(), ..Default::default() } }

	/// Creates a MountainClientConfig from environment variables.
	///
	/// Reads configuration from the following environment
	/// variables:
	/// - `MOUNTAIN_ADDRESS`: gRPC server address (default: `"[::1]:50051"`)
	/// - `MOUNTAIN_CONNECTION_TIMEOUT_SECS`: Connection timeout in seconds
	///   (default: 5)
	/// - `MOUNTAIN_REQUEST_TIMEOUT_SECS`: Request timeout in seconds (default:
	///   30)
	/// - `MOUNTAIN_TLS_ENABLED`: Enable TLS if set to "1" or "true"
	/// - `MOUNTAIN_CA_CERT`: Path to CA certificate file
	/// - `MOUNTAIN_CLIENT_CERT`: Path to client certificate file
	/// - `MOUNTAIN_CLIENT_KEY`: Path to client private key file
	/// - `MOUNTAIN_SERVER_NAME`: Server name for SNI
	/// - `MOUNTAIN_VERIFY_CERTS`: Verify certificates (default: true, set to
	///   "0" or "false" to disable)
	///
	/// # Returns
	/// New MountainClientConfig instance loaded from environment
	pub fn from_env() -> Self {
		let address = env::var("MOUNTAIN_ADDRESS").unwrap_or_else(|_| DEFAULT_MOUNTAIN_ADDRESS.to_string());

		let connection_timeout_secs = env::var("MOUNTAIN_CONNECTION_TIMEOUT_SECS")
			.ok()
			.and_then(|s| s.parse().ok())
			.unwrap_or(DEFAULT_CONNECTION_TIMEOUT_SECS);

		let request_timeout_secs = env::var("MOUNTAIN_REQUEST_TIMEOUT_SECS")
			.ok()
			.and_then(|s| s.parse().ok())
			.unwrap_or(DEFAULT_REQUEST_TIMEOUT_SECS);

		#[cfg(feature = "mtls")]
		let tls_config = if env::var("MOUNTAIN_TLS_ENABLED")
			.map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
			.unwrap_or(false)
		{
			Some(TlsConfig {
				ca_cert_path:env::var("MOUNTAIN_CA_CERT").ok().map(PathBuf::from),
				client_cert_path:env::var("MOUNTAIN_CLIENT_CERT").ok().map(PathBuf::from),
				client_key_path:env::var("MOUNTAIN_CLIENT_KEY").ok().map(PathBuf::from),
				server_name:env::var("MOUNTAIN_SERVER_NAME").ok(),
				verify_certs:env::var("MOUNTAIN_VERIFY_CERTS")
					.map(|v| v != "0" && !v.eq_ignore_ascii_case("false"))
					.unwrap_or(true),
			})
		} else {
			None
		};

		#[cfg(not(feature = "mtls"))]
		let tls_config = None;

		Self {
			address,

			connection_timeout_secs,

			request_timeout_secs,

			#[cfg(feature = "mtls")]
			tls_config,
		}
	}

	/// Sets the connection timeout.
	///
	/// # Parameters
	/// - `timeout_secs`: Timeout in seconds
	///
	/// # Returns
	/// Self for method chaining
	pub fn with_connection_timeout(mut self, timeout_secs:u64) -> Self {
		self.connection_timeout_secs = timeout_secs;

		self
	}

	/// Sets the request timeout.
	///
	/// # Parameters
	/// - `timeout_secs`: Timeout in seconds
	///
	/// # Returns
	/// Self for method chaining
	pub fn with_request_timeout(mut self, timeout_secs:u64) -> Self {
		self.request_timeout_secs = timeout_secs;

		self
	}

	/// Sets the TLS configuration (requires mtls feature).
	///
	/// # Parameters
	/// - `tls_config`: The TLS configuration
	///
	/// # Returns
	/// Self for method chaining
	#[cfg(feature = "mtls")]
	pub fn with_tls(mut self, tls_config:TlsConfig) -> Self {
		self.tls_config = Some(tls_config);

		self
	}
}

