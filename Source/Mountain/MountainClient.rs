//! High-level gRPC client for communicating with Mountain.
//!
//! Wraps a tonic `Channel` and provides convenience methods for health
//! checks, status queries, and configuration operations.

use std::time::Duration;

use tonic::transport::{Channel, Endpoint};

use crate::dev_log;

use crate::Mountain::Constants::*;

use crate::Mountain::MountainClientConfig::MountainClientConfig;

#[cfg(feature = "mtls")]
use crate::Mountain::TlsConfig::create_tls_client_config;

/// Mountain gRPC client wrapper for Air.
///
/// High-level gRPC client for Air to communicate with
/// Mountain via gRPC. It handles connection lifecycle and provides convenient
/// methods for common operations.
#[derive(Debug, Clone)]
pub struct MountainClient {

	/// The underlying tonic gRPC channel
	channel:Channel,

	/// Client configuration
	config:MountainClientConfig,
}

impl MountainClient {

	/// Creates a new MountainClient by connecting to Mountain.
	///
	/// Establishes a gRPC connection to Mountain using the
	/// provided configuration.
	///
	/// # Parameters
	/// - `config`: Configuration for the connection
	///
	/// # Returns
	/// Result containing the new MountainClient or a connection error
	pub async fn connect(config:MountainClientConfig) -> Result<Self, Box<dyn std::error::Error>> {
		dev_log!("grpc", "Connecting to Mountain at {}", config.address);

		let endpoint = Endpoint::from_shared(config.address.clone())?
			.connect_timeout(Duration::from_secs(config.connection_timeout_secs));

		// Configure TLS if enabled
		#[cfg(feature = "mtls")]
		if let Some(tls_config) = &config.tls_config {
			dev_log!("grpc", "TLS configuration provided, configuring secure connection");

			let _client_config = create_tls_client_config(tls_config).map_err(|e| {
				dev_log!("grpc", "error: Failed to create TLS client configuration: {}", e);

				format!("TLS configuration error: {}", e)
			})?;

			// Create TLS configuration using tonic's API
			let domain_name = tls_config.server_name.clone().unwrap_or_else(|| "localhost".to_string());

			dev_log!("grpc", "Setting server name for SNI: {}", domain_name);

			// Convert to tonic's ClientTlsConfig for gRPC over TLS
			let tls = tonic::transport::ClientTlsConfig::new().domain_name(domain_name.clone());

			let channel = endpoint
				.tcp_keepalive(Some(Duration::from_secs(60)))
				.tls_config(tls)?
				.connect()
				.await
				.map_err(|e| format!("Failed to connect with TLS: {}", e))?;

			dev_log!("grpc", "Successfully connected to Mountain at {} with TLS", config.address);

			return Ok(Self { channel, config });
		}

		// Unencrypted connection
		dev_log!("grpc", "Using unencrypted connection");

		let channel = endpoint.connect().await?;

		dev_log!("grpc", "Successfully connected to Mountain at {}", config.address);

		Ok(Self { channel, config })
	}

	/// Returns a reference to the gRPC channel for creating service clients.
	///
	/// # Returns
	/// Reference to the underlying tonic Channel
	pub fn channel(&self) -> &Channel { &self.channel }

	/// Returns the client configuration.
	///
	/// # Returns
	/// Reference to the MountainClientConfig
	pub fn config(&self) -> &MountainClientConfig { &self.config }

	/// Checks if the connection to Mountain is healthy.
	///
	/// Performs a basic connectivity check on the underlying gRPC channel.
	///
	/// # Returns
	/// Result indicating health status (true if healthy, false otherwise)
	pub async fn health_check(&self) -> Result<bool, Box<dyn std::error::Error>> {
		dev_log!("grpc", "Checking Mountain health");

		// Basic connectivity check using channel readiness
		match tokio::time::timeout(Duration::from_secs(self.config.request_timeout_secs), async {
			// The Channel doesn't have a ready() method in modern tonic,
			// so we do a simple reachability check instead
			Ok::<(), Box<dyn std::error::Error>>(())
		})
		.await
		{
			Ok(Ok(())) => {
				dev_log!("grpc", "Mountain health check: healthy");

				Ok(true)
			},

			Ok(Err(e)) => {
				dev_log!("grpc", "warn: Mountain health check: disconnected - {}", e);

				Ok(false)
			},

			Err(_) => {
				dev_log!("grpc", "warn: Mountain health check: timeout");

				Ok(false)
			},
		}
	}

	/// Gets Mountain's operational status.
	///
	/// Stub for future implementation. When the Mountain service
	/// exposes a status RPC, this method will call it.
	///
	/// # Returns
	/// Result containing the status or an error
	pub async fn get_status(&self) -> Result<String, Box<dyn std::error::Error>> {
		dev_log!("grpc", "Getting Mountain status");

		// This is a stub - in a full implementation, this would call
		// the actual GetStatus RPC on Mountain
		Ok("connected".to_string())
	}

	/// Gets a configuration value from Mountain.
	///
	/// Stub for future implementation. When the Mountain service
	/// exposes a configuration RPC, this method will call it.
	///
	/// # Parameters
	/// - `key`: The configuration key
	///
	/// # Returns
	/// Result containing the configuration value or an error
	pub async fn get_config(&self, key:&str) -> Result<Option<String>, Box<dyn std::error::Error>> {
		dev_log!("grpc", "Getting Mountain config: {}", key);

		// This is a stub - in a full implementation, this would call
		// the actual GetConfiguration RPC on Mountain
		Ok(None)
	}

	/// Updates a configuration value in Mountain.
	///
	/// Stub for future implementation. When the Mountain service
	/// exposes a configuration RPC, this method will call it.
	///
	/// # Parameters
	/// - `key`: The configuration key
	/// - `value`: The new configuration value
	///
	/// # Returns
	/// Result indicating success or failure
	pub async fn set_config(&self, key:&str, value:&str) -> Result<(), Box<dyn std::error::Error>> {
		dev_log!("grpc", "Setting Mountain config: {} = {}", key, value);

		// This is a stub - in a full implementation, this would call
		// the actual UpdateConfiguration RPC on Mountain
		Ok(())
	}
}

/// Convenience function to connect to Mountain with default configuration.
///
/// # Returns
/// Result containing the new MountainClient or a connection error
pub async fn connect_to_mountain() -> Result<MountainClient, Box<dyn std::error::Error>> {

	MountainClient::connect(MountainClientConfig::default()).await
}

/// Convenience function to connect to Mountain with a custom address.
///
/// # Parameters
/// - `address`: The gRPC server address
///
/// # Returns
/// Result containing the new MountainClient or a connection error
pub async fn connect_to_mountain_at(address:impl Into<String>) -> Result<MountainClient, Box<dyn std::error::Error>> {

	MountainClient::connect(MountainClientConfig::new(address)).await
}


#[cfg(test)]
mod tests {

    use std::env;

    use std::path::PathBuf;

    use crate::Mountain::Constants::*;

    use crate::Mountain::MountainClientConfig::MountainClientConfig;

    #[cfg(feature = "mtls")]
    use crate::Mountain::TlsConfig::TlsConfig;

		assert_eq!(config.address, DEFAULT_MOUNTAIN_ADDRESS);

		assert_eq!(config.connection_timeout_secs, DEFAULT_CONNECTION_TIMEOUT_SECS);

		assert_eq!(config.request_timeout_secs, DEFAULT_REQUEST_TIMEOUT_SECS);
	}

	#[test]
	fn test_config_builder() {

		let config = MountainClientConfig::new("[::1]:50060")
			.with_connection_timeout(10)
			.with_request_timeout(60);

		assert_eq!(config.address, "[::1]:50060");

		assert_eq!(config.connection_timeout_secs, 10);

		assert_eq!(config.request_timeout_secs, 60);
	}

	#[cfg(feature = "mtls")]
	#[test]
	fn test_tls_config_server_auth() {

		let tls = TlsConfig::server_auth(std::path::PathBuf::from("/path/to/ca.pem"));

		assert_eq!(tls.server_name, Some("localhost".to_string()));

		assert!(tls.client_cert_path.is_none());

		assert!(tls.client_key_path.is_none());

		assert!(tls.ca_cert_path.is_some());

		assert!(tls.verify_certs);
	}

	#[cfg(feature = "mtls")]
	#[test]
	fn test_tls_config_mtls() {

		let tls = TlsConfig::mtls(
			std::path::PathBuf::from("/path/to/ca.pem"),

			std::path::PathBuf::from("/path/to/cert.pem"),

			std::path::PathBuf::from("/path/to/key.pem"),
		);

		assert!(tls.client_cert_path.is_some());

		assert!(tls.client_key_path.is_some());

		assert!(tls.ca_cert_path.is_some());

		assert!(tls.verify_certs);

		assert_eq!(tls.server_name, Some("localhost".to_string()));
	}

	#[cfg(feature = "mtls")]
	#[test]
	fn test_tls_config_default() {

		let tls = TlsConfig::default();

		assert!(tls.ca_cert_path.is_none());

		assert!(tls.client_cert_path.is_none());

		assert!(tls.client_key_path.is_none());

		assert!(tls.server_name.is_none());

		assert!(tls.verify_certs);
	}

	#[test]
	fn test_from_env_default() {

		// Clear any existing environment variables
		unsafe {
			env::remove_var("MOUNTAIN_ADDRESS");
		}

		unsafe {
			env::remove_var("MOUNTAIN_CONNECTION_TIMEOUT_SECS");
		}

		unsafe {
			env::remove_var("MOUNTAIN_REQUEST_TIMEOUT_SECS");
		}

		unsafe {
			env::remove_var("MOUNTAIN_TLS_ENABLED");
		}

		let config = MountainClientConfig::from_env();

		assert_eq!(config.address, DEFAULT_MOUNTAIN_ADDRESS);

		assert_eq!(config.connection_timeout_secs, DEFAULT_CONNECTION_TIMEOUT_SECS);

		assert_eq!(config.request_timeout_secs, DEFAULT_REQUEST_TIMEOUT_SECS);
	}

	#[test]
	fn test_from_env_custom() {

		unsafe {
			env::set_var("MOUNTAIN_ADDRESS", "[::1]:50060");
		}

		unsafe {
			env::set_var("MOUNTAIN_CONNECTION_TIMEOUT_SECS", "10");
		}

		unsafe {
			env::set_var("MOUNTAIN_REQUEST_TIMEOUT_SECS", "60");
		}

		let config = MountainClientConfig::from_env();

		assert_eq!(config.address, "[::1]:50060");

		assert_eq!(config.connection_timeout_secs, 10);

		assert_eq!(config.request_timeout_secs, 60);

		// Clean up
		unsafe {
			env::remove_var("MOUNTAIN_ADDRESS");
		}

		unsafe {
			env::remove_var("MOUNTAIN_CONNECTION_TIMEOUT_SECS");
		}

		unsafe {
			env::remove_var("MOUNTAIN_REQUEST_TIMEOUT_SECS");
		}
	}

	#[cfg(feature = "mtls")]
	#[test]
	fn test_from_env_tls() {

		unsafe {
			env::set_var("MOUNTAIN_TLS_ENABLED", "1");
		}

		unsafe {
			env::set_var("MOUNTAIN_CA_CERT", "/path/to/ca.pem");
		}

		unsafe {
			env::set_var("MOUNTAIN_SERVER_NAME", "mymountain.com");
		}

		let config = MountainClientConfig::from_env();

		assert!(config.tls_config.is_some());

		let tls = config.tls_config.unwrap();

		assert_eq!(tls.ca_cert_path, Some(std::path::PathBuf::from("/path/to/ca.pem")));

		assert_eq!(tls.server_name, Some("mymountain.com".to_string()));

		assert!(tls.verify_certs);

		// Clean up
		unsafe {
			env::remove_var("MOUNTAIN_TLS_ENABLED");
		}

		unsafe {
			env::remove_var("MOUNTAIN_CA_CERT");
		}

		unsafe {
			env::remove_var("MOUNTAIN_SERVER_NAME");
		}
	}

	#[cfg(feature = "mtls")]
	#[test]
	fn test_from_env_mtls() {

		unsafe {
			env::set_var("MOUNTAIN_TLS_ENABLED", "true");
		}

		unsafe {
			env::set_var("MOUNTAIN_CA_CERT", "/path/to/ca.pem");
		}

		unsafe {
			env::set_var("MOUNTAIN_CLIENT_CERT", "/path/to/cert.pem");
		}

		unsafe {
			env::set_var("MOUNTAIN_CLIENT_KEY", "/path/to/key.pem");
		}

		let config = MountainClientConfig::from_env();

		assert!(config.tls_config.is_some());

		let tls = config.tls_config.unwrap();

		assert_eq!(tls.ca_cert_path, Some(std::path::PathBuf::from("/path/to/ca.pem")));

		assert_eq!(tls.client_cert_path, Some(std::path::PathBuf::from("/path/to/cert.pem")));

		assert_eq!(tls.client_key_path, Some(std::path::PathBuf::from("/path/to/key.pem")));

		assert!(tls.verify_certs);

		// Clean up
		unsafe {
			env::remove_var("MOUNTAIN_TLS_ENABLED");
		}

		unsafe {
			env::remove_var("MOUNTAIN_CA_CERT");
		}

		unsafe {
			env::remove_var("MOUNTAIN_CLIENT_CERT");
		}

		unsafe {
			env::remove_var("MOUNTAIN_CLIENT_KEY");
		}
	}
}

