//! HTTP Client Module with DNS Override
//!
//! This module provides a secured HTTP client that uses the local DNS server
//! for all DNS resolution. This ensures that all `*.editor.land` queries go
//! through the local Hickory DNS server, which resolves them to `127.x.x.x`
//! addresses as a defense-in-depth measure.

use std::{sync::Arc, time::Duration};

use anyhow::Result;
// Re-export types from Mist workspace dependency
pub use Mist::Resolver::LandDnsResolver;
#[allow(unused_imports)]
pub use Mist::Resolver::TokioResolver;
#[allow(unused_imports)]
pub use Mist::Resolver::LandResolver;

/// Creates a secured reqwest ClientBuilder with DNS override configured.
///
/// This returns a `ClientBuilder` with the DNS resolver already set, allowing
/// you to add additional configurations before calling `.build()`.
///
/// # Parameters
///
/// * `dns_port` - The port of the local DNS server (obtained from
///   `mist::dns_port()`)
///
/// # Returns
///
/// Returns a configured `reqwest::ClientBuilder` with the local DNS resolver.
///
/// # Example
///
/// ```rust,no_run
/// use std::time::Duration;
///
/// use AirLibrary::HTTP::secured_client_builder;
/// use Mist;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
/// 	let dns_port = mist::dns_port();
/// 	let client = secured_client_builder(dns_port)?.timeout(Duration::from_secs(30)).build()?;
///
/// 	// All HTTP requests will use the local DNS server
/// 	Ok(())
/// }
/// ```
pub fn secured_client_builder(dns_port:u16) -> Result<reqwest::ClientBuilder> {
	let resolver = Arc::new(LandDnsResolver::new(dns_port));

	Ok(reqwest::Client::builder().dns_resolver(resolver))
}

/// Creates a secured reqwest Client with DNS override.
///
/// This client uses the local DNS server (running on the specified port)
/// for all DNS resolution. This is a security measure to ensure that all
/// `*.editor.land` queries go through the local Hickory DNS server, which
/// validates that they only resolve to `127.x.x.x` addresses.
///
/// # Parameters
///
/// * `dns_port` - The port of the local DNS server (obtained from
///   `mist::dns_port()`)
///
/// # Returns
///
/// Returns a configured `reqwest::Client` that uses the local DNS resolver.
///
/// # Example
///
/// ```rust,no_run
/// use AirLibrary::HTTP::secured_client;
/// use Mist;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
/// 	let dns_port = mist::dns_port();
/// 	let client = secured_client(dns_port)?;
///
/// 	// All HTTP requests will use the local DNS server
/// 	let response = client.get("https://code.editor.land").send().await?;
/// 	Ok(())
/// }
/// ```
///
/// # Security
///
/// The DNS override ensures:
/// - All DNS queries go through the local DNS server
/// - `*.editor.land` domains resolve only to `127.x.x.x` addresses
/// - Protection against DNS spoofing and cache poisoning
/// - Defense-in-depth security for the local network
pub fn secured_client(dns_port:u16) -> Result<reqwest::Client> {
	secured_client_builder(dns_port)?
		.build()
		.map_err(|e| anyhow::anyhow!("Failed to build reqwest client: {}", e))
}

/// Creates a secured reqwest Client with timeout and DNS override.
///
/// This client uses the local DNS server for all DNS resolution and
/// has a default timeout configured.
///
/// # Parameters
///
/// * `dns_port` - The port of the local DNS server (obtained from
///   `mist::dns_port()`)
/// * `timeout` - The timeout duration for requests
///
/// # Returns
///
/// Returns a configured `reqwest::Client` with DNS override and timeout.
///
/// # Example
///
/// ```rust,no_run
/// use std::time::Duration;
///
/// use AirLibrary::HTTP::secured_client_with_timeout;
/// use Mist;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
/// 	let dns_port = mist::dns_port();
/// 	let client = secured_client_with_timeout(dns_port, Duration::from_secs(30))?;
///
/// 	// All HTTP requests will use the local DNS server with 30s timeout
/// 	Ok(())
/// }
/// ```
pub fn secured_client_with_timeout(dns_port:u16, timeout:Duration) -> Result<reqwest::Client> {
	secured_client_builder(dns_port)?
		.timeout(timeout)
		.build()
		.map_err(|e| anyhow::anyhow!("Failed to build reqwest client: {}", e))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_secured_client_creation() {
		let port = 15353;
		let result = secured_client(port);
		// Should succeed even if DNS server isn't running (client creation doesn't
		// require DNS)
		assert!(result.is_ok(), "Client creation should succeed");
	}

	#[test]
	fn test_secured_client_builder_creation() {
		let port = 15354;
		let result = secured_client_builder(port);
		assert!(result.is_ok(), "ClientBuilder creation should succeed");
	}
}
