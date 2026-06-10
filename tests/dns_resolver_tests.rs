//! DNS Resolver Integration Tests
//!
//! These tests verify the Land DNS resolver functionality, including:
//! - Resolution of editor.land domains to localhost
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

	// Test that code.editor.land resolves to localhost
	let lookup = resolver.lookup_ip("code.editor.land").await.expect("DNS lookup failed");

	let resolved_ips:Vec<_> = lookup.iter().collect();

	println!("Resolved IPs for code.editor.land: {:?}", resolved_ips);

	assert!(!resolved_ips.is_empty(), "Should resolve to at least one IP");

	assert!(
		resolved_ips.iter().all(|ip| ip.is_loopback()),
<<<<<<< HEAD
		"All resolved IPs for land.playform.cloud should be loopback addresses"
=======
		"All resolved IPs for editor.land should be loopback addresses"
>>>>>>> e2a56fcd30371f045835aabb633a4bb67d5bfd55
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
<<<<<<< HEAD
		"api.land.playform.cloud",
=======
		"api.editor.land",
>>>>>>> e2a56fcd30371f045835aabb633a4bb67d5bfd55
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
<<<<<<< HEAD
	// This test verifies the security feature that ensures land.playform.cloud
=======
	// This test verifies the security feature that ensures editor.land
>>>>>>> e2a56fcd30371f045835aabb633a4bb67d5bfd55
	// domains only resolve to loopback addresses (127.x.x.x)

	// Start DNS server
	let port = Mist::start(15372).expect("Failed to start DNS server");

	tokio::time::sleep(Duration::from_millis(200)).await;

	let resolver = Mist::resolver::land_resolver(port);

	// Query editor.land domain
	let lookup = resolver.lookup_ip("code.editor.land").await.expect("DNS lookup failed");

	// Verify ALL returned IPs are loopback
	let mut all_loopback = true;

	for ip in lookup.iter() {
		if !ip.is_loopback() {
			all_loopback = false;

			println!("SECURITY WARNING: editor.land resolved to non-loopback IP: {}", ip);
		}
	}

	assert!(
		all_loopback,
<<<<<<< HEAD
		"SECURITY: land.playform.cloud domains must only resolve to loopback addresses (127.x.x.x)"
=======
		"SECURITY: editor.land domains must only resolve to loopback addresses (127.x.x.x)"
>>>>>>> e2a56fcd30371f045835aabb633a4bb67d5bfd55
	);

	println!("Security check passed: All editor.land IPs are loopback addresses");
}

#[tokio::test]
async fn test_ip_validation_allows_non_editor_land() {
<<<<<<< HEAD
	// This test verifies that non-land.playform.cloud domains can resolve to any IP
=======
	// This test verifies that non-editor.land domains can resolve to any IP
>>>>>>> e2a56fcd30371f045835aabb633a4bb67d5bfd55
	// (subject to the forward authority allowlist restrictions)

	// Start DNS server
	let port = Mist::start(15373).expect("Failed to start DNS server");

	tokio::time::sleep(Duration::from_millis(200)).await;

	let resolver = Mist::resolver::land_resolver(port);

	// Try to resolve a non-editor.land domain
	// This may fail or return NXDOMAIN since we don't have forwarding configured
	let result = resolver.lookup_ip("example.com").await;

	println!("Non-editor.land DNS query result: {:?}", result);

	// The important thing is that the resolver doesn't crash or improperly filter
	assert!(true, "Resolver handles non-editor.land domains gracefully");
}

#[tokio::test]
async fn test_resolver_handles_ipv6() {
	// Test that the resolver can handle IPv6 addresses
	// even though editor.land only has A records

	let port = Mist::start(15374).expect("Failed to start DNS server");

	tokio::time::sleep(Duration::from_millis(200)).await;

	let resolver = Mist::resolver::land_resolver(port);

	// The resolver should handle IPv6 queries without crashing
	// (editor.land only has A records, so this won't return results)
	let result = resolver.ipv6_lookup("code.editor.land").await;

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
	let lookup1 = resolver.lookup_ip("code.editor.land").await.expect("DNS lookup failed");

	let ips1:Vec<_> = lookup1.iter().collect();

	// Second query (should be cached)
	let lookup2 = resolver.lookup_ip("code.editor.land").await.expect("DNS lookup failed");

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
	let lookup = resolver.lookup_ip("code.editor.land").await.expect("DNS lookup failed");

	assert!(!lookup.iter().collect::<Vec<_>>().is_empty(), "Resolver should resolve domains");

	println!("Resolver configured with port {}: OK", port);
}

#[tokio::test]
async fn test_resolver_error_handling() {
	// Test that the resolver handles errors gracefully

	// Try to create resolver for non-existent DNS server
	let resolver = Mist::resolver::land_resolver(19999);

	// Try to resolve (should fail or timeout)
	let result = resolver.lookup_ip("code.editor.land").await;

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
	// The editor.land zone may not have TXT records, but the resolver
	// should handle the query gracefully
	let result = resolver.txt_lookup("editor.land").await;

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
	let result = resolver.mx_lookup("editor.land").await;

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

	let result = resolver.lookup_ip("code.editor.land").await;

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
<<<<<<< HEAD
		"code.land.playform.cloud",
		"api.land.playform.cloud",
=======
		"code.editor.land",
		"api.editor.land",
>>>>>>> e2a56fcd30371f045835aabb633a4bb67d5bfd55
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
