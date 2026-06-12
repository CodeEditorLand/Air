//! TLS configuration for gRPC connections to Mountain.
//!
//! Provides types and functions for establishing secure gRPC connections
//! with optional mutual TLS (mTLS) authentication.

use std::{fs::File, io::BufReader, path::PathBuf};

#[cfg(feature = "mtls")]
use rustls::ClientConfig;
#[cfg(feature = "mtls")]
use rustls::RootCertStore;

use crate::dev_log;

/// TLS configuration for gRPC connections to Mountain.
///
/// Paths to certificates and keys required for
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
/// Loads certificates and keys from the file system and
/// constructs a rustls ClientConfig suitable for gRPC connections.
///
/// # Parameters
/// - `tls_config`: The TLS configuration containing certificate paths
///
/// # Returns
/// Result containing the ClientConfig or an error if certificate loading fails
#[cfg(feature = "mtls")]
pub fn create_tls_client_config(tls_config:&TlsConfig) -> Result<ClientConfig, Box<dyn std::error::Error>> {
	dev_log!("grpc", "Creating TLS client configuration");

	// Build the root certificate store
	let mut root_store = RootCertStore::empty();

	if let Some(ca_path) = &tls_config.ca_cert_path {
		// Load CA certificate from file
		dev_log!("grpc", "Loading CA certificate from {:?}", ca_path);

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

		dev_log!("grpc", "Loaded CA certificate from {:?}", ca_path);
	} else {
		// Use system root certificates via rustls-native-certs 0.8.x API
		dev_log!("grpc", "Loading system root certificates");

		let cert_result = rustls_native_certs::load_native_certs();

		// Log any errors encountered while loading certificates
		if !cert_result.errors.is_empty() {
			dev_log!(
				"grpc",
				"warn: Encountered errors loading system certificates: {:?}",
				cert_result.errors
			);
		}

		let native_certs = cert_result.certs;

		if native_certs.is_empty() {
			dev_log!("grpc", "warn: No system root certificates found");
		}

		for cert in native_certs {
			root_store
				.add(cert)
				.map_err(|e| format!("Failed to add system certificate to root store: {}", e))?;
		}

		dev_log!("grpc", "Loaded {} system root certificates", root_store.len());
	}

	// Load client certificate and key for mTLS (if provided)
	let client_certs = if tls_config.client_cert_path.is_some() && tls_config.client_key_path.is_some() {
		let cert_path = tls_config.client_cert_path.as_ref().unwrap();

		let key_path = tls_config.client_key_path.as_ref().unwrap();

		dev_log!("grpc", "Loading client certificate from {:?}", cert_path);

		let cert_file = File::open(cert_path).map_err(|e| format!("Failed to open client certificate file: {}", e))?;

		let mut cert_reader = BufReader::new(cert_file);

		let certs:Result<Vec<_>, _> = rustls_pemfile::certs(&mut cert_reader).collect();

		let certs = certs.map_err(|e| format!("Failed to parse client certificate: {}", e))?;

		if certs.is_empty() {
			return Err("No client certificates found in file".into());
		}

		dev_log!("grpc", "Loading client private key from {:?}", key_path);

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

			dev_log!("grpc", "Configured mTLS with client certificate");

			client_config
		},

		None => {
			// TLS configuration with server authentication only
			// rustls 0.23: The builder will auto-complete when no client auth needed
			let client_config = ClientConfig::builder().with_root_certificates(root_store).with_no_client_auth();

			dev_log!("grpc", "Configured TLS with server authentication only");

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
		dev_log!(
			"grpc",
			"warn: Certificate verification disabled - this is NOT secure for production!"
		); // For development/testing, consider using a custom verifier

		// For now, this is a placeholder - verification is always enabled
	}

	dev_log!("grpc", "TLS client configuration created successfully");

	Ok(config)
}
