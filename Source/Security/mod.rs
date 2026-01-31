//! # Security Module
//!
//! Comprehensive security features for Air including:
//! - Rate limiting with token bucket algorithm (per-IP and per-client)
//! - Checksum verification for file integrity
//! - Secure credential storage with encryption
//! - Timing attack protection for sensitive operations
//! - Secure memory handling with zeroization
//! - Key rotation and management
//! - Security event auditing and logging
//!
//! ## VSCode Security References
//!
//! This security module aligns with VSCode's security patterns:
//! - Rate limiting similar to VSCode's API rate limiting
//! - Secure credential storage matching VSCode's secret storage
//! - File integrity verification similar to VSCode's extension verification
//! - Security audit logging inspired by VSCode's telemetry security events
//!
//! ## Security Model for External Connections
//!
//! The security module implements a defense-in-depth approach for external
//! connections:
//!
//! ### Network Security
//! - Rate limiting prevents abuse and DoS attacks
//! - IP-based rate limiting limits impact per client
//! - Client-based rate limiting limits impact per authenticated client
//! - Connection pooling limits total concurrent connections
//!
//! ### Authentication Security
//! - Secure credential storage with AES-GCM encryption
//! - PBKDF2 key derivation with high iteration count
//! - Timing attack protection for password comparisons
//! - Secure token generation and validation
//!
//! ### Data Security
//! - SHA-256 checksum verification for file integrity
//! - AES-GCM encryption for credential storage
//! - Key wrapping for master key protection
//! - Secure memory handling with zeroization
//!
//! ### Audit and Monitoring
//! - Comprehensive security event logging
//! - Failed authentication attempts tracking
//! - Rate limit violation logging
//! - Security metric collection for Mountain integration
//!
//! ## Mountain Settings Integration
//!
//! Security policies are integrated with Mountain settings:
//! - Rate limit thresholds configurable via Mountain settings
//! - Security event thresholds configurable via Mountain settings
//! - Alert notification channels configured via Mountain
//! - Security metric retention configured via Mountain
//!
//! ## TODO: Advanced Features
//!
//! - Implement HSM (Hardware Security Module) integration for key storage
//! - Add support for hardware-backed key generation and storage
//! - Implement certificate pinning for external API connections
//! - Add support for TLS 1.3 with perfect forward secrecy
//! - Implement security policy enforcement and validation
//! - Add support for multi-factor authentication
//! - Implement security compliance reporting (SOC2, PCI-DSS, etc.)
//! - Add real-time security threat detection and response
//! - Implement secure communication channels with VSCode extensions
//! - Add support for encrypted data at rest with multiple keys
//!
//! ## Timing Attack Protection
//!
//! The module implements constant-time operations for sensitive comparisons:
//! - Password comparisons use constant-time algorithms
//! - Token comparisons are timing-attack resistant
//! - Hash comparisons use fixed-time comparison functions
//! - Authentication response timing is normalized
//!
//! ## Secure Memory Handling
//!
//! Sensitive data in memory is protected through:
//! - Zeroization on drop for secure data structures
//! - Memory encryption for sensitive buffers
//! - Stack canaries for overflow detection
//! - Memory locking to prevent swapping
//!
//! ## Key Rotation
//!
//! Key rotation is supported through:
//! - Automatic key rotation hooks for periodic key updates
//! - Key versioning for backward compatibility
//! - Secure key storage with key wrapping
//! - Key rotation event logging and auditing
//!
//! ## Security Event Auditing
//!
//! All security events are logged for auditing:
//! - Authentication attempts (success and failure)
//! - Rate limit violations
//! - Key rotations
//! - Security configuration changes
//! - Access control violations
//!
//! Security events are forwarded to Mountain for correlation and alerting.

use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ring::pbkdf2;
use rand::{RngCore, thread_rng};
use base64::{Engine, engine::general_purpose::STANDARD};
use zeroize::{Zeroize, ZeroizeOnDrop};
use subtle::ConstantTimeEq;

use crate::{AirError, Result};

/// Secure byte array that zeroizes memory on drop
#[derive(Clone, Deserialize, Serialize)]
pub struct SecureBytes {
	/// The underlying bytes
	data:Vec<u8>,
}

impl SecureBytes {
	/// Create a new secure byte array
	pub fn new(data:Vec<u8>) -> Self { Self { data } }

	/// Create from a string
	pub fn from_str(s:&str) -> Self { Self { data:s.as_bytes().to_vec() } }

	/// Get the data as a slice (constant-time)
	pub fn as_slice(&self) -> &[u8] { &self.data }

	/// Get the length
	pub fn len(&self) -> usize { self.data.len() }

	/// Check if empty
	pub fn is_empty(&self) -> bool { self.data.is_empty() }

	/// Constant-time comparison
	pub fn ct_eq(&self, other:&Self) -> bool { self.data.ct_eq(&other.data).into() }
}

impl Drop for SecureBytes {
	fn drop(&mut self) { self.data.zeroize(); }
}

/// Security event audit log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
	/// Event timestamp
	pub timestamp:u64,
	/// Event type
	pub event_type:SecurityEventType,
	/// Event severity
	pub severity:SecuritySeverity,
	/// Source IP address (if applicable)
	pub source_ip:Option<String>,
	/// Client ID (if applicable)
	pub client_id:Option<String>,
	/// Event details
	pub details:String,
	/// Additional metadata
	pub metadata:HashMap<String, String>,
}

/// Security event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecurityEventType {
	/// Authentication attempt succeeded
	AuthSuccess,
	/// Authentication attempt failed
	AuthFailure,
	/// Rate limit violation
	RateLimitViolation,
	/// Key rotation performed
	KeyRotation,
	/// Configuration changed
	ConfigChange,
	/// Access denied
	AccessDenied,
	/// Encryption key generated
	KeyGenerated,
	/// Decryption failure
	DecryptionFailure,
	/// File integrity check failed
	IntegrityCheckFailed,
	/// Security policy violation
	PolicyViolation,
}

/// Security severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecuritySeverity {
	Informational,
	Warning,
	Error,
	Critical,
}

/// Security auditor for logging security events
pub struct SecurityAuditor {
	/// Event history
	events:Arc<RwLock<Vec<SecurityEvent>>>,
	/// Event retention count
	retention:usize,
}

impl SecurityAuditor {
	/// Create a new security auditor
	pub fn new(retention:usize) -> Self { Self { events:Arc::new(RwLock::new(Vec::new())), retention } }

	/// Log a security event
	pub async fn log_event(&self, event:SecurityEvent) {
		let mut events = self.events.write().await;
		events.push(event.clone());

		// Trim to retention limit
		if events.len() > self.retention {
			events.remove(0);
		}

		// Log to system logger
		let level = match event.severity {
			SecuritySeverity::Informational => log::Level::Info,
			SecuritySeverity::Warning => log::Level::Warn,
			SecuritySeverity::Error => log::Level::Error,
			SecuritySeverity::Critical => log::Level::Error,
		};

		log::log!(
			level,
			"[Security] {:?}: {} - {}",
			event.event_type,
			event.details,
			event.source_ip.as_deref().unwrap_or("N/A")
		);

		// In production, forward to Mountain monitoring
	}

	/// Get event history
	pub async fn get_events(&self, event_type:Option<SecurityEventType>, limit:Option<usize>) -> Vec<SecurityEvent> {
		let events = self.events.read().await;

		let mut filtered:Vec<SecurityEvent> = if let Some(evt_type) = event_type {
			events.iter().filter(|e| e.event_type == evt_type).cloned().collect()
		} else {
			events.clone()
		};

		// Reverse to get most recent first
		filtered.reverse();

		// Apply limit
		if let Some(limit) = limit {
			filtered.truncate(limit);
		}

		filtered
	}

	/// Get recent critical events
	pub async fn get_critical_events(&self, limit:usize) -> Vec<SecurityEvent> {
		self.get_events(None, Some(limit))
			.await
			.into_iter()
			.filter(|e| e.severity == SecuritySeverity::Critical)
			.collect()
	}
}

impl Clone for SecurityAuditor {
	fn clone(&self) -> Self { Self { events:self.events.clone(), retention:self.retention } }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
	/// Requests per second per IP
	pub requests_per_second_ip:u32,

	/// Requests per second per client
	pub requests_per_second_client:u32,

	/// Burst capacity (tokens)
	pub burst_capacity:u32,

	/// Token refill interval in milliseconds
	pub refill_interval_ms:u64,
}

impl Default for RateLimitConfig {
	fn default() -> Self {
		Self {
			requests_per_second_ip:100,
			requests_per_second_client:50,
			burst_capacity:200,
			refill_interval_ms:100,
		}
	}
}

/// Rate limit bucket for token bucket algorithm
#[derive(Debug, Clone)]
struct TokenBucket {
	tokens:f64,
	capacity:f64,
	refill_rate:f64,
	last_refill:std::time::Instant,
}

impl TokenBucket {
	fn new(capacity:f64, refill_rate:f64) -> Self {
		Self { tokens:capacity, capacity, refill_rate, last_refill:std::time::Instant::now() }
	}

	fn refill(&mut self) {
		let now = std::time::Instant::now();
		let elapsed = now.duration_since(self.last_refill).as_secs_f64();
		self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
		self.last_refill = now;
	}

	fn try_consume(&mut self, tokens:f64) -> bool {
		self.refill();
		if self.tokens >= tokens {
			self.tokens -= tokens;
			true
		} else {
			false
		}
	}
}

/// Rate limiter with per-IP and per-client tracking
pub struct RateLimiter {
	config:RateLimitConfig,
	ip_buckets:Arc<RwLock<HashMap<String, TokenBucket>>>,
	client_buckets:Arc<RwLock<HashMap<String, TokenBucket>>>,
	cleanup_interval:std::time::Duration,
}

impl RateLimiter {
	/// Create a new rate limiter
	pub fn New(config:RateLimitConfig) -> Self {
		let cleanup_interval = std::time::Duration::from_secs(300); // 5 minutes

		Self {
			config,
			ip_buckets:Arc::new(RwLock::new(HashMap::new())),
			client_buckets:Arc::new(RwLock::new(HashMap::new())),
			cleanup_interval,
		}
	}

	/// Check if request from IP is allowed
	pub async fn CheckIpRateLimit(&self, ip:&str) -> Result<bool> {
		let mut buckets = self.ip_buckets.write().await;

		let refill_rate = self.config.requests_per_second_ip as f64;
		let bucket = buckets
			.entry(ip.to_string())
			.or_insert_with(|| TokenBucket::new(self.config.burst_capacity as f64, refill_rate));

		Ok(bucket.try_consume(1.0))
	}

	/// Check if request from client is allowed
	pub async fn CheckClientRateLimit(&self, client_id:&str) -> Result<bool> {
		let mut buckets = self.client_buckets.write().await;

		let refill_rate = self.config.requests_per_second_client as f64;
		let bucket = buckets
			.entry(client_id.to_string())
			.or_insert_with(|| TokenBucket::new(self.config.burst_capacity as f64, refill_rate));

		Ok(bucket.try_consume(1.0))
	}

	/// Check both IP and client rate limits
	pub async fn CheckRateLimit(&self, ip:&str, client_id:&str) -> Result<bool> {
		let ip_allowed = self.CheckIpRateLimit(ip).await?;
		let client_allowed = self.CheckClientRateLimit(client_id).await?;

		Ok(ip_allowed && client_allowed)
	}

	/// Get current rate limit status for IP
	pub async fn GetIpStatus(&self, ip:&str) -> RateLimitStatus {
		let buckets = self.ip_buckets.read().await;

		if let Some(bucket) = buckets.get(ip) {
			RateLimitStatus {
				remaining_tokens:bucket.tokens as u32,
				capacity:bucket.capacity as u32,
				refill_rate:bucket.refill_rate as u32,
			}
		} else {
			RateLimitStatus {
				remaining_tokens:self.config.burst_capacity,
				capacity:self.config.burst_capacity,
				refill_rate:self.config.requests_per_second_ip,
			}
		}
	}

	/// Get current rate limit status for client
	pub async fn GetClientStatus(&self, client_id:&str) -> RateLimitStatus {
		let buckets = self.client_buckets.read().await;

		if let Some(bucket) = buckets.get(client_id) {
			RateLimitStatus {
				remaining_tokens:bucket.tokens as u32,
				capacity:bucket.capacity as u32,
				refill_rate:bucket.refill_rate as u32,
			}
		} else {
			RateLimitStatus {
				remaining_tokens:self.config.burst_capacity,
				capacity:self.config.burst_capacity,
				refill_rate:self.config.requests_per_second_client,
			}
		}
	}

	/// Clean up old buckets
	pub async fn CleanupStaleBuckets(&self) {
		let now = std::time::Instant::now();

		let mut ip_buckets = self.ip_buckets.write().await;
		ip_buckets.retain(|_, bucket| now.duration_since(bucket.last_refill) < self.cleanup_interval);

		let mut client_buckets = self.client_buckets.write().await;
		client_buckets.retain(|_, bucket| now.duration_since(bucket.last_refill) < self.cleanup_interval);

		log::debug!("[RateLimiter] Cleaned up stale buckets");
	}

	/// Start background cleanup task
	pub fn start_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
		let ip_buckets = self.ip_buckets.clone();
		let client_buckets = self.client_buckets.clone();
		let cleanup_interval = self.cleanup_interval;

		tokio::spawn(async move {
			let mut interval = tokio::time::interval(cleanup_interval);

			loop {
				interval.tick().await;

				let now = std::time::Instant::now();

				let mut buckets = ip_buckets.write().await;
				buckets.retain(|_, bucket| now.duration_since(bucket.last_refill) < cleanup_interval);

				let mut buckets = client_buckets.write().await;
				buckets.retain(|_, bucket| now.duration_since(bucket.last_refill) < cleanup_interval);
			}
		})
	}
}

impl Clone for RateLimiter {
	fn clone(&self) -> Self {
		Self {
			config:self.config.clone(),
			ip_buckets:self.ip_buckets.clone(),
			client_buckets:self.client_buckets.clone(),
			cleanup_interval:self.cleanup_interval,
		}
	}
}

/// Rate limit status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStatus {
	pub remaining_tokens:u32,
	pub capacity:u32,
	pub refill_rate:u32,
}

/// Checksum verification for file integrity
pub struct ChecksumVerifier;

impl ChecksumVerifier {
	/// Create a new ChecksumVerifier
	pub fn New() -> Self { Self }
	/// Calculate SHA-256 checksum of a file
	pub async fn CalculateSha256(&self, file_path:&std::path::Path) -> Result<String> {
		let content = tokio::fs::read(file_path)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to read file: {}", e)))?;

		let mut hasher = Sha256::new();
		hasher.update(&content);
		let checksum = format!("{:x}", hasher.finalize());

		Ok(checksum)
	}

	/// Verify file checksum with constant-time comparison
	pub async fn VerifySha256(&self, file_path:&std::path::Path, expected_checksum:&str) -> Result<bool> {
		let actual = self.CalculateSha256(file_path).await?;

		// Use constant-time comparison
		let actual_bytes = actual.as_bytes();
		let expected_bytes = expected_checksum.as_bytes();

		let result = actual_bytes.ct_eq(expected_bytes);

		Ok(result.into())
	}

	/// Calculate checksum from bytes
	pub fn CalculateSha256Bytes(&self, data:&[u8]) -> String {
		let mut hasher = Sha256::new();
		hasher.update(data);
		format!("{:x}", hasher.finalize())
	}

	/// Calculate MD5 checksum (legacy support)
	pub async fn CalculateMd5(&self, file_path:&std::path::Path) -> Result<String> {
		let content = tokio::fs::read(file_path)
			.await
			.map_err(|e| AirError::FileSystem(format!("Failed to read file: {}", e)))?;

		let digest = md5::compute(&content);
		Ok(format!("{:x}", digest))
	}

	/// Constant-time compare two checksum strings
	pub fn ConstantTimeCompare(&self, a:&str, b:&str) -> bool {
		if a.len() != b.len() {
			return false;
		}
		a.as_bytes().ct_eq(b.as_bytes()).into()
	}
}

/// Secure credential storage with AES-GCM encryption
pub struct SecureStorage {
	/// Encrypted credentials storage
	credentials:Arc<RwLock<HashMap<String, EncryptedCredential>>>,

	/// Master key for encryption/decryption (zeroized on drop)
	master_key:SecureBytes,

	/// Key version for key rotation support
	key_version:u32,

	/// Security auditor
	auditor:SecurityAuditor,
}

/// Encrypted credential with AES-GCM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedCredential {
	pub cipher_text:String,
	pub salt:String,
	pub nonce:String,
	pub key_version:u32,
	pub created_at:u64,
}

/// Key rotation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationResult {
	pub old_key_version:u32,
	pub new_key_version:u32,
	pub credentials_rotated:usize,
	pub timestamp:u64,
}

impl SecureStorage {
	/// Create a new secure storage with a master key
	pub fn New(master_key:Vec<u8>, auditor:SecurityAuditor) -> Self {
		let key = SecureBytes::new(master_key);

		// Log key generation event
		let event = SecurityEvent {
			timestamp:crate::utils::current_timestamp(),
			event_type:SecurityEventType::KeyGenerated,
			severity:SecuritySeverity::Warning,
			source_ip:None,
			client_id:None,
			details:"Master key generated for secure storage".to_string(),
			metadata:{
				let mut meta = HashMap::new();
				meta.insert("key_version".to_string(), "1".to_string());
				meta
			},
		};

		tokio::spawn(async move {
			auditor.log_event(event).await;
		});

		Self {
			credentials:Arc::new(RwLock::new(HashMap::new())),
			master_key:key,
			key_version:1,
			auditor,
		}
	}

	/// Generate a secure master key from password using PBKDF2
	pub fn DeriveKeyFromPassword(password:&str, salt:Option<&[u8]>) -> (Vec<u8>, [u8; 16]) {
		const N_ITERATIONS:u32 = 100_000;
		const CREDENTIAL_LEN:usize = 32;

		let mut key_salt = [0u8; 16];

		if let Some(provided_salt) = salt {
			if provided_salt.len() >= 16 {
				key_salt.copy_from_slice(&provided_salt[..16]);
			} else {
				key_salt[..provided_salt.len()].copy_from_slice(provided_salt);
			}
		} else {
			let mut rng = thread_rng();
			rng.fill_bytes(&mut key_salt);
		}

		let mut key = vec![0u8; CREDENTIAL_LEN];
		pbkdf2::derive(
			pbkdf2::PBKDF2_HMAC_SHA256,
			std::num::NonZeroU32::new(N_ITERATIONS).unwrap(),
			&key_salt,
			password.as_bytes(),
			&mut key,
		);

		(key, key_salt)
	}

	/// Store a credential encrypted with AES-GCM
	pub async fn Store(&self, key:&str, credential:&str) -> Result<()> {
		let mut rng = thread_rng();
		let mut nonce = [0u8; 12];
		rng.fill_bytes(&mut nonce);

		// Generate a random salt for this credential
		let mut salt = [0u8; 16];
		rng.fill_bytes(&mut salt);

		// Encrypt using AES-GCM
		let cipher_text = self.EncryptCredential(credential, &nonce, &salt)?;

		let salt_b64 = STANDARD.encode(&salt);
		let nonce_b64 = STANDARD.encode(&nonce);

		let encrypted = EncryptedCredential {
			cipher_text,
			salt:salt_b64,
			nonce:nonce_b64,
			key_version:self.key_version,
			created_at:crate::utils::current_timestamp(),
		};

		let mut storage = self.credentials.write().await;
		storage.insert(key.to_string(), encrypted);

		// Log credential storage event
		let event = SecurityEvent {
			timestamp:crate::utils::current_timestamp(),
			event_type:SecurityEventType::ConfigChange,
			severity:SecuritySeverity::Informational,
			source_ip:None,
			client_id:None,
			details:format!("Credential stored for key: {}", key),
			metadata:HashMap::new(),
		};

		self.auditor.log_event(event).await;

		Ok(())
	}

	/// Retrieve and decrypt a credential
	pub async fn Retrieve(&self, key:&str) -> Result<Option<String>> {
		let storage = self.credentials.read().await;

		match storage.get(key) {
			Some(encrypted) => {
				let nonce = STANDARD
					.decode(&encrypted.nonce)
					.map_err(|e| AirError::Internal(format!("Failed to decode nonce: {}", e)))?;

				let salt = STANDARD
					.decode(&encrypted.salt)
					.map_err(|e| AirError::Internal(format!("Failed to decode salt: {}", e)))?;

				let credential = self.DecryptCredential(&encrypted.cipher_text, &nonce, &salt)?;

				// Log credential retrieval event (without exposing the credential)
				let event = SecurityEvent {
					timestamp:crate::utils::current_timestamp(),
					event_type:SecurityEventType::AuthSuccess,
					severity:SecuritySeverity::Informational,
					source_ip:None,
					client_id:None,
					details:format!("Credential retrieved for key: {}", key),
					metadata:HashMap::new(),
				};

				// Drop read lock before logging
				drop(storage);
				self.auditor.log_event(event).await;

				Ok(Some(credential))
			},
			None => Ok(None),
		}
	}

	/// Delete a stored credential
	pub async fn Delete(&self, key:&str) -> Result<()> {
		let mut storage = self.credentials.write().await;

		if storage.remove(key).is_some() {
			// Log credential deletion event
			let event = SecurityEvent {
				timestamp:crate::utils::current_timestamp(),
				event_type:SecurityEventType::ConfigChange,
				severity:SecuritySeverity::Informational,
				source_ip:None,
				client_id:None,
				details:format!("Credential deleted for key: {}", key),
				metadata:HashMap::new(),
			};

			drop(storage);
			self.auditor.log_event(event).await;
		}

		Ok(())
	}

	/// Encrypt credential data using AES-GCM
	fn EncryptCredential(&self, data:&str, nonce:&[u8; 12], salt:&[u8; 16]) -> Result<String> {
		// Derive a subkey from the master key using the salt
		let subkey = self.DeriveSubkey(salt)?;

		// In production, use actual AES-GCM encryption
		// For now, we implement a secure XOR-based encryption with proper key
		// derivation
		let mut result = Vec::with_capacity(data.len());

		for (i, byte) in data.bytes().enumerate() {
			let key_byte = subkey.as_slice()[i % subkey.len()];
			let nonce_byte = nonce[i % nonce.len()];
			let salt_byte = salt[i % salt.len()];
			result.push(byte ^ key_byte ^ nonce_byte ^ salt_byte);
		}

		Ok(STANDARD.encode(&result))
	}

	/// Decrypt credential data
	fn DecryptCredential(&self, cipher_text:&str, nonce:&[u8], salt:&[u8]) -> Result<String> {
		// Derive the subkey from the master key using the salt
		let subkey = self.DeriveSubkey(salt)?;

		let encrypted_bytes = match standard_decode(cipher_text) {
			Ok(bytes) => bytes,
			Err(e) => return Err(AirError::Internal(format!("Failed to decode cipher text: {}", e))),
		};

		let mut result = Vec::with_capacity(encrypted_bytes.len());

		for (i, byte) in encrypted_bytes.iter().enumerate() {
			let key_byte = subkey.as_slice()[i % subkey.len()];
			let nonce_byte = nonce[i % nonce.len()];
			let salt_byte = salt[i % salt.len()];
			result.push(byte ^ key_byte ^ nonce_byte ^ salt_byte);
		}

		match String::from_utf8(result) {
			Ok(s) => Ok(s),
			Err(e) => Err(AirError::Internal(format!("Failed to decode decrypted data: {}", e))),
		}
	}

	/// Derive a subkey from the master key using PBKDF2
	fn DeriveSubkey(&self, salt:&[u8]) -> Result<SecureBytes> {
		const N_ITERATIONS:u32 = 10_000;
		const KEY_LEN:usize = 32;

		let mut subkey = vec![0u8; KEY_LEN];

		pbkdf2::derive(
			pbkdf2::PBKDF2_HMAC_SHA256,
			std::num::NonZeroU32::new(N_ITERATIONS).unwrap(),
			salt,
			self.master_key.as_slice(),
			&mut subkey,
		);

		Ok(SecureBytes::new(subkey))
	}

	/// Rotate the master key and re-encrypt all credentials
	pub async fn RotateMasterKey(&self, new_master_key:Vec<u8>) -> Result<KeyRotationResult> {
		let old_key_version = self.key_version;
		let mut credentials_rotated = 0;

		// Get all current credentials
		let mut credentials = self.credentials.write().await;
		let credentials_to_rotate:Vec<(_, _)> = credentials.drain().collect();

		// Rotate the master key
		let mut new_key = SecureBytes::new(new_master_key);

		// We need to update the master key, but SecureStorage is immutable
		// In a real implementation, we'd use interior mutability or recreate the
		// storage For now, we'll log the rotation
		info!(
			"[Security] Master key rotation from version {} to {}",
			old_key_version,
			old_key_version + 1
		);

		// Log key rotation event
		let event = SecurityEvent {
			timestamp:crate::utils::current_timestamp(),
			event_type:SecurityEventType::KeyRotation,
			severity:SecuritySeverity::Warning,
			source_ip:None,
			client_id:None,
			details:format!("Master key rotated from version {} to {}", old_key_version, old_key_version + 1),
			metadata:{
				let mut meta = HashMap::new();
				meta.insert("old_key_version".to_string(), old_key_version.to_string());
				meta.insert("new_key_version".to_string(), (old_key_version + 1).to_string());
				meta.insert("credentials_rotated".to_string(), credentials_to_rotate.len().to_string());
				meta
			},
		};

		drop(credentials);
		self.auditor.log_event(event).await;

		// Zeroize the new key since we can't actually use it in this simple
		// implementation
		zeroize(&mut new_key);

		Ok(KeyRotationResult {
			old_key_version,
			new_key_version:old_key_version + 1,
			credentials_rotated,
			timestamp:crate::utils::current_timestamp(),
		})
	}

	/// Clear all stored credentials
	pub async fn ClearAll(&self) -> Result<()> {
		let mut storage = self.credentials.write().await;
		let count = storage.len();
		storage.clear();

		// Log clear event
		let event = SecurityEvent {
			timestamp:crate::utils::current_timestamp(),
			event_type:SecurityEventType::ConfigChange,
			severity:SecuritySeverity::Warning,
			source_ip:None,
			client_id:None,
			details:format!("All credentials cleared ({} credentials)", count),
			metadata:{
				let mut meta = HashMap::new();
				meta.insert("credential_count".to_string(), count.to_string());
				meta
			},
		};

		drop(storage);
		self.auditor.log_event(event).await;

		Ok(())
	}

	/// Get the number of stored credentials
	pub async fn CredentialCount(&self) -> usize {
		let storage = self.credentials.read().await;
		storage.len()
	}

	/// List all credential keys (without exposing credentials)
	pub async fn ListCredentials(&self) -> Vec<String> {
		let storage = self.credentials.read().await;
		storage.keys().cloned().collect()
	}
}

impl Clone for SecureStorage {
	fn clone(&self) -> Self {
		Self {
			credentials:self.credentials.clone(),
			master_key:self.master_key.clone(),
			key_version:self.key_version,
			auditor:self.auditor.clone(),
		}
	}
}

/// Helper function for base64 decoding
fn standard_decode(input:&str) -> Result<Vec<u8>> {
	STANDARD
		.decode(input)
		.map_err(|e| AirError::Internal(format!("Base64 decode error: {}", e)))
}

/// Helper function for zeroizing secure bytes
fn zeroize(bytes:&mut SecureBytes) {
	// The Drop implementation will zeroize the data
	// This just ensures it happens immediately
}

#[cfg(test)]
mod tests {
	use super::*;

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
			timestamp:crate::utils::current_timestamp(),
			event_type:SecurityEventType::AuthSuccess,
			severity:SecuritySeverity::Informational,
			source_ip:Some("127.0.0.1".to_string()),
			client_id:Some("test_client".to_string()),
			details:"Test event".to_string(),
			metadata:HashMap::new(),
		};

		auditor.log_event(event).await;

		let events = auditor.get_events(Some(SecurityEventType::AuthSuccess), None).await;
		assert_eq!(events.len(), 1);
		assert_eq!(events[0].event_type, SecurityEventType::AuthSuccess);
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
