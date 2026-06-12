#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use crate::{Security::{SecurityAuditor, SecurityEvent, SecurityEventType, SecuritySeverity}};

	use super::super::{SecureBytes, ChecksumVerifier, RateLimitConfig, RateLimiter, SecureStorage};

	#[tokio::test]
	async fn test_rate_limiter() {
		let config = RateLimitConfig::default();

		let limiter = RateLimiter::New(config);

		// Should allow requests within limit
		for _ in 0..50 {
			let allowed = limiter.CheckIpRateLimit("127.0.0.1").await.unwrap();

			assert!(allowed);
		}

		// After burst, should eventually deny
		let mut denied_count = 0;

		for _ in 0..200 {
			if !limiter.CheckIpRateLimit("127.0.0.1").await.unwrap() {
				denied_count += 1;
			}
		}

		assert!(denied_count > 0);
	}

	#[tokio::test]
	async fn test_checksum_verification() {
		let verifier = ChecksumVerifier::New();

		let data = b"test data";

		let checksum = verifier.CalculateSha256Bytes(data);

		assert_eq!(checksum.len(), 64); // SHA-256 hex is 64 chars

		assert!(!checksum.is_empty());
	}

	#[tokio::test]
	async fn test_secure_storage() {
		let master_key = vec![1u8; 32];

		let auditor = SecurityAuditor::new(100);

		let storage = SecureStorage::New(master_key, auditor);

		storage.Store("test_key", "secret_value").await.unwrap();

		let retrieved = storage.Retrieve("test_key").await.unwrap();

		assert_eq!(retrieved, Some("secret_value".to_string()));
	}

	#[tokio::test]
	async fn test_constant_time_comparison() {
		let verifier = ChecksumVerifier::New();

		// Test equal strings
		assert!(verifier.ConstantTimeCompare("abc123", "abc123"));

		// Test unequal strings
		assert!(!verifier.ConstantTimeCompare("abc123", "def456"));

		// Test different lengths
		assert!(!verifier.ConstantTimeCompare("abc", "abcd"));
	}

	#[tokio::test]
	async fn test_security_auditor() {
		let auditor = SecurityAuditor::new(10);

		let event = SecurityEvent {
			Timestamp:crate::Utility::CurrentTimestamp(),

			EventType:SecurityEventType::AuthSuccess,

			Severity:SecuritySeverity::Informational,

			SourceIp:Some("127.0.0.1".to_string()),

			ClientId:Some("test_client".to_string()),

			Details:"Test event".to_string(),

			Metadata:HashMap::new(),
		};

		auditor.LogEvent(event).await;

		let events = auditor.GetEvents(Some(SecurityEventType::AuthSuccess), None).await;

		assert_eq!(events.len(), 1);

		assert_eq!(events[0].EventType, SecurityEventType::AuthSuccess);
	}

	#[tokio::test]
	async fn test_secure_bytes() {
		let bytes1 = SecureBytes::from_str("secret_password");

		let bytes2 = SecureBytes::from_str("secret_password");

		let bytes3 = SecureBytes::from_str("different_password");

		assert!(bytes1.ct_eq(&bytes2));

		assert!(!bytes1.ct_eq(&bytes3));
	}

	#[tokio::test]
	async fn test_rate_limit_combined() {
		let config = RateLimitConfig::default();

		let limiter = RateLimiter::New(config);

		// Check combined rate limit
		let allowed = limiter.CheckRateLimit("127.0.0.1", "client_1").await.unwrap();

		assert!(allowed);

		// Get status
		let ip_status = limiter.GetIpStatus("127.0.0.1").await;

		let client_status = limiter.GetClientStatus("client_1").await;

		assert!(ip_status.remaining_tokens > 0);

		assert!(client_status.remaining_tokens > 0);
	}
}
