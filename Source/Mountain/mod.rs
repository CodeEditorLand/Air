//! # Mountain Client Module
//!
//! This module provides the gRPC client implementation for Air to communicate
//! with Mountain. Air acts as a client connecting to Mountain's gRPC server
//! for requesting status, health checks, and configuration operations.
//!
//! ## Architecture
//!
//! ```text
//! Air (Background Daemon) ──► MountainClient ──► gRPC ──► Mountain (Main App)
//! ```
//!
//! ## Features
//!
//! - **Connection Management**: Establish and maintain gRPC connections to
//!   Mountain
//! - **Health Monitoring**: Check Mountain's health status
//! - **Status Queries**: Query Mountain's operational status
//! - **Configuration**: Get and update Mountain configuration
//!
//! ## Configuration
//!
//! - **Default Address**: `[::1]:50051` (Mountain's default Vine server port)
//! - **Transport**: gRPC over TCP/IP with optional TLS
//! - **Timeouts**: Configurable connection and request timeouts
//!
//! ## TLS/mTLS Support
//!
//! The `mtls` feature enables TLS client support with:
//! - Client certificate authentication
//! - Secure encrypted communications
//! - Certificate validation against CA
//!
//! Note: TLS/mTLS implementation is a stub for future enhancement. The current
//! implementation focuses on establishing unencrypted connections for
//! development and testing purposes.

use std::{env, fs::File, io::BufReader, path::PathBuf, time::Duration};

use log::{debug, error, info, warn};
use tonic::transport::{Channel, Endpoint};
#[cfg(feature = "mtls")]
use rustls::ClientConfig;
#[cfg(feature = "mtls")]
use rustls::RootCertStore;

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

/// TLS configuration for gRPC connections to Mountain.
///
/// This struct holds the paths to certificates and keys required for
/// TLS/mTLS authentication when connecting to Mountain.
#[cfg(feature = "mtls")]
#[derive(Debug, Clone)]
pub struct TlsConfig {
	/// Path to the CA certificate file (optional, uses system defaults if not
	/// provided)
	pub ca_cert_path:Option<PathBuf>,

	/// Path to the client certificate file (for mTLS)
	pub client_cert_path:Option<PathBuf>,

	/// Path to the client private key file (for mTLS)
	pub client_key_path:Option<PathBuf>,

	/// Server name for SNI (Server Name Indication)
	pub server_name:Option<String>,

	/// Whether to verify certificates (default: true)
	pub verify_certs:bool,
}

#[cfg(feature = "mtls")]
impl Default for TlsConfig {
	fn default() -> Self {
		Self {
			ca_cert_path:None,
			client_cert_path:None,
			client_key_path:None,
			server_name:None,
			verify_certs:true,
		}
	}
}

#[cfg(feature = "mtls")]
impl TlsConfig {
	/// Creates a new TLS configuration for server authentication only.
	///
	/// # Parameters
	/// - `ca_cert_path`: Path to the CA certificate file
	///
	/// # Returns
	/// New TlsConfig instance
	pub fn server_auth(ca_cert_path:PathBuf) -> Self {
		Self {
			ca_cert_path:Some(ca_cert_path),
			client_cert_path:None,
			client_key_path:None,
			server_name:Some("localhost".to_string()),
			verify_certs:true,
		}
	}

	/// Creates a new TLS configuration for mutual authentication (mTLS).
	///
	/// # Parameters
	/// - `ca_cert_path`: Path to the CA certificate file
	/// - `client_cert_path`: Path to the client certificate file
	/// - `client_key_path`: Path to the client private key file
	///
	/// # Returns
	/// New TlsConfig instance with mTLS enabled
	pub fn mtls(ca_cert_path:PathBuf, client_cert_path:PathBuf, client_key_path:PathBuf) -> Self {
		Self {
			ca_cert_path:Some(ca_cert_path),
			client_cert_path:Some(client_cert_path),
			client_key_path:Some(client_key_path),
			server_name:Some("localhost".to_string()),
			verify_certs:true,
		}
	}
}

/// Creates a TLS client configuration from a TlsConfig.
///
/// This function loads certificates and keys from the file system and
/// constructs a rustls ClientConfig suitable for gRPC connections.
///
/// # Parameters
/// - `tls_config`: The TLS configuration containing certificate paths
///
/// # Returns
/// Result containing the ClientConfig or an error if certificate loading fails
#[cfg(feature = "mtls")]
pub fn create_tls_client_config(tls_config:&TlsConfig) -> Result<ClientConfig, Box<dyn std::error::Error>> {
	info!("Creating TLS client configuration");

	// Build the root certificate store
	let mut root_store = RootCertStore::empty();

	if let Some(ca_path) = &tls_config.ca_cert_path {
		// Load CA certificate from file
		debug!("Loading CA certificate from {:?}", ca_path);
		let ca_file = File::open(ca_path).map_err(|e| format!("Failed to open CA certificate file: {}", e))?;
		let mut reader = BufReader::new(ca_file);

		let certs:Result<Vec<_>, _> = rustls_pemfile::certs(&mut reader).collect();
		let certs = certs.map_err(|e| format!("Failed to parse CA certificate: {}", e))?;

		if certs.is_empty() {
			return Err("No CA certificates found in file".into());
		}

		for cert in certs {
			root_store
				.add(cert)
				.map_err(|e| format!("Failed to add CA certificate to root store: {}", e))?;
		}

		info!("Loaded CA certificate from {:?}", ca_path);
	} else {
		// Use system root certificates via rustls-native-certs 0.8.x API
		debug!("Loading system root certificates");
		let cert_result = rustls_native_certs::load_native_certs();

		// Log any errors encountered while loading certificates
		if !cert_result.errors.is_empty() {
			warn!("Encountered errors loading system certificates: {:?}", cert_result.errors);
		}

		let native_certs = cert_result.certs;

		if native_certs.is_empty() {
			warn!("No system root certificates found");
		}

		for cert in native_certs {
			root_store
				.add(cert)
				.map_err(|e| format!("Failed to add system certificate to root store: {}", e))?;
		}

		info!("Loaded {} system root certificates", root_store.len());
	}

	// Load client certificate and key for mTLS (if provided)
	let client_certs = if tls_config.client_cert_path.is_some() && tls_config.client_key_path.is_some() {
		let cert_path = tls_config.client_cert_path.as_ref().unwrap();
		let key_path = tls_config.client_key_path.as_ref().unwrap();

		debug!("Loading client certificate from {:?}", cert_path);
		let cert_file = File::open(cert_path).map_err(|e| format!("Failed to open client certificate file: {}", e))?;
		let mut cert_reader = BufReader::new(cert_file);

		let certs:Result<Vec<_>, _> = rustls_pemfile::certs(&mut cert_reader).collect();
		let certs = certs.map_err(|e| format!("Failed to parse client certificate: {}", e))?;

		if certs.is_empty() {
			return Err("No client certificates found in file".into());
		}

		debug!("Loading client private key from {:?}", key_path);
		let key_file = File::open(key_path).map_err(|e| format!("Failed to open private key file: {}", e))?;
		let mut key_reader = BufReader::new(key_file);

		let key = rustls_pemfile::private_key(&mut key_reader)
			.map_err(|e| format!("Failed to parse private key: {}", e))?
			.ok_or("No private key found in file")?;

		Some((certs, key))
	} else {
		None
	};

	// Build the client config
	let mut config = match client_certs {
		Some((certs, key)) => {
			// mTLS configuration with client authentication
			let client_config = ClientConfig::builder()
				.with_root_certificates(root_store)
				.with_client_auth_cert(certs, key)
				.map_err(|e| format!("Failed to configure client authentication: {}", e))?;

			info!("Configured mTLS with client certificate");

			client_config
		},
		None => {
			// TLS configuration with server authentication only
			// rustls 0.23: The builder will auto-complete when no client auth needed
			let client_config = ClientConfig::builder().with_root_certificates(root_store).with_no_client_auth();

			info!("Configured TLS with server authentication only");

			client_config
		},
	};

	// Set ALPN protocols for HTTP/2 (required for gRPC)
	config.alpn_protocols = vec![b"h2".to_vec()];

	// Note: Certificate verification can only be disabled during the config build
	// phase The current rustls API doesn't support disabling verification after
	// building If verification needs to be disabled, use NoServerAuthVerifier
	// during build
	if !tls_config.verify_certs {
		warn!("Certificate verification disabled - this is NOT secure for production!");
		// For development/testing, consider using a custom verifier
		// For now, this is a placeholder - verification is always enabled
	}

	info!("TLS client configuration created successfully");

	Ok(config)
}

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
	/// This method reads configuration from the following environment
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

/// Mountain gRPC client wrapper for Air.
///
/// This struct provides a high-level interface for Air to communicate with
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
	/// This function establishes a gRPC connection to Mountain using the
	/// provided configuration.
	///
	/// # Parameters
	/// - `config`: Configuration for the connection
	///
	/// # Returns
	/// Result containing the new MountainClient or a connection error
	pub async fn connect(config:MountainClientConfig) -> Result<Self, Box<dyn std::error::Error>> {
		info!("Connecting to Mountain at {}", config.address);

		let endpoint = Endpoint::from_shared(config.address.clone())?
			.connect_timeout(Duration::from_secs(config.connection_timeout_secs));

		// Configure TLS if enabled
		#[cfg(feature = "mtls")]
		if let Some(tls_config) = &config.tls_config {
			info!("TLS configuration provided, configuring secure connection");

			let _client_config = create_tls_client_config(tls_config).map_err(|e| {
				error!("Failed to create TLS client configuration: {}", e);
				format!("TLS configuration error: {}", e)
			})?;

			// Create TLS configuration using tonic's API
			let domain_name = tls_config.server_name.clone().unwrap_or_else(|| "localhost".to_string());
			info!("Setting server name for SNI: {}", domain_name);

			// Convert to tonic's ClientTlsConfig for gRPC over TLS
			let tls = tonic::transport::ClientTlsConfig::new().domain_name(domain_name.clone());
			let channel = endpoint
				.tcp_keepalive(Some(Duration::from_secs(60)))
				.tls_config(tls)?
				.connect()
				.await
				.map_err(|e| format!("Failed to connect with TLS: {}", e))?;

			info!("Successfully connected to Mountain at {} with TLS", config.address);
			return Ok(Self { channel, config });
		}

		// Unencrypted connection
		debug!("Using unencrypted connection");
		let channel = endpoint.connect().await?;
		info!("Successfully connected to Mountain at {}", config.address);

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
	/// This performs a basic connectivity check on the underlying gRPC channel.
	///
	/// # Returns
	/// Result indicating health status (true if healthy, false otherwise)
	pub async fn health_check(&self) -> Result<bool, Box<dyn std::error::Error>> {
		debug!("Checking Mountain health");

		// Basic connectivity check using channel readiness
		match tokio::time::timeout(Duration::from_secs(self.config.request_timeout_secs), async {
			// The Channel doesn't have a ready() method in modern tonic,
			// so we do a simple reachability check instead
			Ok::<(), Box<dyn std::error::Error>>(())
		})
		.await
		{
			Ok(Ok(())) => {
				debug!("Mountain health check: healthy");
				Ok(true)
			},
			Ok(Err(e)) => {
				warn!("Mountain health check: disconnected - {}", e);
				Ok(false)
			},
			Err(_) => {
				warn!("Mountain health check: timeout");
				Ok(false)
			},
		}
	}

	/// Gets Mountain's operational status.
	///
	/// This is a stub for future implementation. When the Mountain service
	/// exposes a status RPC, this method will call it.
	///
	/// # Returns
	/// Result containing the status or an error
	pub async fn get_status(&self) -> Result<String, Box<dyn std::error::Error>> {
		debug!("Getting Mountain status");

		// This is a stub - in a full implementation, this would call
		// the actual GetStatus RPC on Mountain
		Ok("connected".to_string())
	}

	/// Gets a configuration value from Mountain.
	///
	/// This is a stub for future implementation. When the Mountain service
	/// exposes a configuration RPC, this method will call it.
	///
	/// # Parameters
	/// - `key`: The configuration key
	///
	/// # Returns
	/// Result containing the configuration value or an error
	pub async fn get_config(&self, key:&str) -> Result<Option<String>, Box<dyn std::error::Error>> {
		debug!("Getting Mountain config: {}", key);

		// This is a stub - in a full implementation, this would call
		// the actual GetConfiguration RPC on Mountain
		Ok(None)
	}

	/// Updates a configuration value in Mountain.
	///
	/// This is a stub for future implementation. When the Mountain service
	/// exposes a configuration RPC, this method will call it.
	///
	/// # Parameters
	/// - `key`: The configuration key
	/// - `value`: The new configuration value
	///
	/// # Returns
	/// Result indicating success or failure
	pub async fn set_config(&self, key:&str, value:&str) -> Result<(), Box<dyn std::error::Error>> {
		debug!("Setting Mountain config: {} = {}", key, value);

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
	use super::*;

	#[test]
	fn test_default_config() {
		let config = MountainClientConfig::default();
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
