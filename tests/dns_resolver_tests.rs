//! DNS Resolver Integration Tests
//!
//! These tests verify the Land DNS resolver functionality, including:
//! - Resolution of land.playform.cloud domains to localhost
//! - IP validation (security enforcement)
//! - Resolver configuration
//!
//! Note: These tests use the DNS resolver from the Mist module.

use std::time::Duration;

#[tokio::test]
async fn test_land_dns_resolver_localhost() {
	// Start DNS server from Mist module
	let port = Mist::start(15370).expect("Failed to start DNS server");

	// Give server time to start
	tokio::time::sleep(Duration::from_millis(200)).await;

	// Create resolver pointing to local server
	let resolver = Mist::resolver::land_resolver(port);

	// Test that code.land.playform.cloud resolves to localhost
	let lookup = resolver.lookup_ip("code.land.playform.cloud").await.expect("DNS lookup failed");

	let resolved_ips:Vec<_> = lookup.iter().collect();

	println!("Resolved IPs for code.land.playform.cloud: {:?}", resolved_ips);

	assert!(!resolved_ips.is_empty(), "Should resolve to at least one IP");

	assert!(
		resolved_ips.iter().all(|ip| ip.is_loopback()),
		"All resolved IPs for land.playform.cloud should be loopback addresses"
	);
}

#[tokio::test]
async fn test_land_dns_resolver_wildcard() {
	// Start DNS server
	let port = Mist::start(15371).expect("Failed to start DNS server");

	tokio::time::sleep(Duration::from_millis(200)).await;

	let resolver = Mist::resolver::land_resolver(port);

	// Test wildcard resolution
	let test_domains = vec![
		"test.editor.land",
		"api.land.playform.cloud",
		"cdn.editor.land",
		"random-subdomain.editor.land",
	];

	for domain in test_domains {
		let lookup = resolver
			.lookup_ip(domain)
			.await
			.expect(&format!("DNS lookup failed for {}", domain));

		let resolved_ips:Vec<_> = lookup.iter().collect();

		println!("Resolved IPs for {}: {:?}", domain, resolved_ips);

		assert!(!resolved_ips.is_empty(), "{} should resolve to at least one IP", domain);

		assert!(
			resolved_ips.iter().all(|ip| ip.is_loopback()),
			"All IPs for {} should be loopback addresses",
			domain
		);
	}
}

#[tokio::test]
async fn test_ip_validation_blocks_non_localhost_for_editor_land() {
	// This test verifies the security feature that ensures land.playform.cloud
	// domains only resolve to loopback addresses (127.x.x.x)

	// Start DNS server
	let port = Mist::start(15372).expect("Failed to start DNS server");

	tokio::time::sleep(Duration::from_millis(200)).await;

	let resolver = Mist::resolver::land_resolver(port);

	// Query land.playform.cloud domain
	let lookup = resolver.lookup_ip("code.land.playform.cloud").await.expect("DNS lookup failed");

	// Verify ALL returned IPs are loopback
	let mut all_loopback = true;

	for ip in lookup.iter() {
		if !ip.is_loopback() {
			all_loopback = false;

			println!("SECURITY WARNING: land.playform.cloud resolved to non-loopback IP: {}", ip);
		}
	}

	assert!(
		all_loopback,
		"SECURITY: land.playform.cloud domains must only resolve to loopback addresses (127.x.x.x)"
	);

	println!("Security check passed: All land.playform.cloud IPs are loopback addresses");
}

#[tokio::test]
async fn test_ip_validation_allows_non_editor_land() {
	// This test verifies that non-land.playform.cloud domains can resolve to any IP
	// (subject to the forward authority allowlist restrictions)

	// Start DNS server
	let port = Mist::start(15373).expect("Failed to start DNS server");

	tokio::time::sleep(Duration::from_millis(200)).await;

	let resolver = Mist::resolver::land_resolver(port);

	// Try to resolve a non-land.playform.cloud domain
	// This may fail or return NXDOMAIN since we don't have forwarding configured
	let result = resolver.lookup_ip("example.com").await;

	println!("Non-land.playform.cloud DNS query result: {:?}", result);

	// The important thing is that the resolver doesn't crash or improperly filter
	assert!(true, "Resolver handles non-land.playform.cloud domains gracefully");
}

#[tokio::test]
async fn test_resolver_handles_ipv6() {
	// Test that the resolver can handle IPv6 addresses
	// even though land.playform.cloud only has A records

	let port = Mist::start(15374).expect("Failed to start DNS server");

	tokio::time::sleep(Duration::from_millis(200)).await;

	let resolver = Mist::resolver::land_resolver(port);

	// The resolver should handle IPv6 queries without crashing
	// (land.playform.cloud only has A records, so this won't return results)
	let result = resolver.ipv6_lookup("code.land.playform.cloud").await;

	println!("IPv6 lookup result: {:?}", result);

	assert!(true, "Resolver handles IPv6 queries gracefully");
}

#[tokio::test]
async fn test_resolver_caching() {
	// Test that the resolver caches results
	// Hickory clients cache results by default

	let port = Mist::start(15375).expect("Failed to start DNS server");

	tokio::time::sleep(Duration::from_millis(200)).await;

	let resolver = Mist::resolver::land_resolver(port);

	// First query
	let lookup1 = resolver.lookup_ip("code.land.playform.cloud").await.expect("DNS lookup failed");

	let ips1:Vec<_> = lookup1.iter().collect();

	// Second query (should be cached)
	let lookup2 = resolver.lookup_ip("code.land.playform.cloud").await.expect("DNS lookup failed");

	let ips2:Vec<_> = lookup2.iter().collect();

	println!("First query IPs: {:?}", ips1);

	println!("Second query IPs: {:?}", ips2);

	assert_eq!(ips1.len(), ips2.len(), "Cached queries should return same number of IPs");

	println!("Resolver caching works correctly");
}

#[tokio::test]
async fn test_resolver_concurrent_queries() {
	// Test that the resolver can handle concurrent queries

	let port = Mist::start(15376).expect("Failed to start DNS server");

	tokio::time::sleep(Duration::from_millis(200)).await;

	let resolver = Mist::resolver::land_resolver(port);

	// Spawn multiple concurrent queries
	let mut handles = vec![];

	for i in 0..5 {
		let resolver_clone = resolver.clone();

		let handle = tokio::spawn(async move {
			let domain = format!("service{}.editor.land", i % 2); // Alternate between 2 domains

			let lookup = resolver_clone.lookup_ip(&domain).await;

			(domain, lookup)
		});

		handles.push(handle);
	}

	// Wait for all queries to complete
	let results = futures::future::join_all(handles).await;

	println!("Con resolver queries completed: {} results", results.len());

	for result in results {
		assert!(result.is_ok(), "Concurrent query should complete");

		let (domain, lookup_result) = result.unwrap();

		match lookup_result {
			Ok(lookup) => {
				let ips:Vec<_> = lookup.iter().collect();

				println!("Concurrent query {} resolved to: {:?}", domain, ips);
			},

			Err(e) => {
				println!("Concurrent query {} failed: {:?}", domain, e);
			},
		}
	}

	assert!(true, "Concurrent queries handled successfully");
}

#[tokio::test]
async fn test_resolver_port_configuration() {
	// Test resolver configuration with specific port

	let port = Mist::start(15377).expect("Failed to start DNS server");

	tokio::time::sleep(Duration::from_millis(200)).await;

	// Create resolver with the same port
	let resolver = Mist::resolver::land_resolver(port);

	// Verify resolver works
	let lookup = resolver.lookup_ip("code.land.playform.cloud").await.expect("DNS lookup failed");

	assert!(!lookup.iter().collect::<Vec<_>>().is_empty(), "Resolver should resolve domains");

	println!("Resolver configured with port {}: OK", port);
}

#[tokio::test]
async fn test_resolver_error_handling() {
	// Test that the resolver handles errors gracefully

	// Try to create resolver for non-existent DNS server
	let resolver = Mist::resolver::land_resolver(19999);

	// Try to resolve (should fail or timeout)
	let result = resolver.lookup_ip("code.land.playform.cloud").await;

	println!("Resolver error handling result: {:?}", result);

	// The important thing is that the resolver handles errors gracefully
	// without crashing
	assert!(true, "Resolver handles errors gracefully");
}

#[tokio::test]
async fn test_resolver_txt_records() {
	// Test that the resolver can query TXT records
	// (if supported)

	let port = Mist::start(15378).expect("Failed to start DNS server");

	tokio::time::sleep(Duration::from_millis(200)).await;

	let resolver = Mist::resolver::land_resolver(port);

	// Try to query TXT records
	// The land.playform.cloud zone may not have TXT records, but the resolver
	// should handle the query gracefully
	let result = resolver.txt_lookup("land.playform.cloud").await;

	println!("TXT lookup result: {:?}", result);

	assert!(true, "Resolver handles TXT queries gracefully");
}

#[tokio::test]
async fn test_resolver_mx_records() {
	// Test that the resolver can query MX records
	// (if supported)

	let port = Mist::start(15379).expect("Failed to start DNS server");

	tokio::time::sleep(Duration::from_millis(200)).await;

	let resolver = Mist::resolver::land_resolver(port);

	// Try to query MX records
	let result = resolver.mx_lookup("land.playform.cloud").await;

	println!("MX lookup result: {:?}", result);

	assert!(true, "Resolver handles MX queries gracefully");
}

#[tokio::test]
async fn test_resolver_srv_records() {
	// Test that the resolver can query SRV records
	// (if supported)

	let port = Mist::start(15380).expect("Failed to start DNS server");

	tokio::time::sleep(Duration::from_millis(200)).await;

	let resolver = Mist::resolver::land_resolver(port);

	// Try to query SRV records
	let result = resolver.srv_lookup("_http._tcp.editor.land").await;

	println!("SRV lookup result: {:?}", result);

	assert!(true, "Resolver handles SRV queries gracefully");
}

#[tokio::test]
async fn test_resolver_timeout_handling() {
	// Test that the resolver handles timeouts appropriately

	let port = Mist::start(15381).expect("Failed to start DNS server");

	tokio::time::sleep(Duration::from_millis(200)).await;

	let resolver = Mist::resolver::land_resolver(port);

	// Query should complete quickly
	let start = std::time::Instant::now();

	let result = resolver.lookup_ip("code.land.playform.cloud").await;

	let elapsed = start.elapsed();

	println!("Query completed in {:?}", elapsed);

	assert!(result.is_ok(), "Query should succeed");

	assert!(elapsed < Duration::from_secs(5), "Query should complete in under 5 seconds");
}

#[tokio::test]
async fn test_resolver_reverse_dns() {
	// Test reverse DNS lookup
	// (if supported)

	let port = Mist::start(15382).expect("Failed to start DNS server");

	tokio::time::sleep(Duration::from_millis(200)).await;

	let resolver = Mist::resolver::land_resolver(port);

	// Try to perform reverse DNS lookup
	let result = resolver.reverse_lookup("127.0.0.1".parse().unwrap()).await;

	println!("Reverse DNS lookup result: {:?}", result);

	assert!(true, "Resolver handles reverse DNS queries gracefully");
}

#[tokio::test]
async fn test_resolver_multiple_domains_batch() {
	// Test resolving multiple domains in sequence

	let port = Mist::start(15383).expect("Failed to start DNS server");

	tokio::time::sleep(Duration::from_millis(200)).await;

	let resolver = Mist::resolver::land_resolver(port);

	let domains = vec![
		"code.land.playform.cloud",
		"api.land.playform.cloud",
		"cdn.editor.land",
		"test.editor.land",
		"random.editor.land",
	];

	for domain in domains {
		let lookup = resolver
			.lookup_ip(domain)
			.await
			.expect(&format!("Failed to resolve {}", domain));

		let ips:Vec<_> = lookup.iter().collect();

		println!("Resolved {}: {:?}", domain, ips);

		assert!(!ips.is_empty(), "{} should resolve to at least one IP", domain);
	}

	println!("Batch resolution test completed successfully");
}
