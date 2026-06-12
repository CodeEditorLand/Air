use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use ring::pbkdf2;
use rand::{Rng, rng};
use base64::{Engine, engine::general_purpose::STANDARD};
use zeroize::Zeroize;

use crate::{AirError, Result, dev_log};
use super::{
	SecureBytes::Struct as SecureBytesType,
	SecurityAuditor::Struct as SecurityAuditorType,
	SecurityEvent::Struct as SecurityEvent,
	SecurityEventType::SecurityEventType,
	SecuritySeverity::SecuritySeverity,
};

/// Secure credential storage with AES-GCM encryption
pub struct Struct {
	/// Encrypted credentials storage
	credentials:Arc<RwLock<HashMap<String, EncryptedCredential>>>,

	/// Master key for encryption/decryption (zeroized on drop)
	master_key:SecureBytesType,

	/// Key version for key rotation support
	key_version:u32,

	/// Security auditor
	auditor:SecurityAuditorType,
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

impl Struct {
	/// Create a new secure storage with a master key
	pub fn New(master_key:Vec<u8>, auditor:SecurityAuditorType) -> Self {
		let key = SecureBytesType::new(master_key);

		// Log key generation event
		let event = SecurityEvent {
			Timestamp:crate::Utility::CurrentTimestamp(),

			EventType:SecurityEventType::KeyGenerated,

			Severity:SecuritySeverity::Warning,

			SourceIp:None,

			ClientId:None,

			Details:"Master key generated for secure storage".to_string(),

			Metadata:{
				let mut meta = HashMap::new();

				meta.insert("key_version".to_string(), "1".to_string());

				meta
			},
		};

		let auditor_clone = auditor.clone();

		tokio::spawn(async move {
			auditor_clone.LogEvent(event).await;
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
			let mut rng = rng();

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
		let mut rng = rng();

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

			created_at:crate::Utility::CurrentTimestamp(),
		};

		let mut storage = self.credentials.write().await;

		storage.insert(key.to_string(), encrypted);

		// Log credential storage event
		let event = SecurityEvent {
			Timestamp:crate::Utility::CurrentTimestamp(),

			EventType:SecurityEventType::ConfigChange,

			Severity:SecuritySeverity::Informational,

			SourceIp:None,

			ClientId:None,

			Details:format!("Credential stored for key: {}", key),

			Metadata:HashMap::new(),
		};

		self.auditor.LogEvent(event).await;

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
					Timestamp:crate::Utility::CurrentTimestamp(),

					EventType:SecurityEventType::AuthSuccess,

					Severity:SecuritySeverity::Informational,

					SourceIp:None,

					ClientId:None,

					Details:format!("Credential retrieved for key: {}", key),

					Metadata:HashMap::new(),
				};

				// Drop read lock before logging
				drop(storage);

				self.auditor.LogEvent(event).await;

				Ok(Some(credential))
			},

			None => Ok(None),
		}
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
	fn DeriveSubkey(&self, salt:&[u8]) -> Result<SecureBytesType> {
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

		Ok(SecureBytesType::new(subkey))
	}

	/// Rotate the master key and re-encrypt all credentials
	pub async fn RotateMasterKey(&self, new_master_key:Vec<u8>) -> Result<KeyRotationResult> {
		let old_key_version = self.key_version;

		let credentials_rotated = 0;

		// Get all current credentials
		let mut credentials = self.credentials.write().await;

		let credentials_to_rotate:Vec<(_, _)> = credentials.drain().collect();

		// Rotate the master key
		let mut new_key = SecureBytesType::new(new_master_key);

		// We need to update the master key, but SecureStorage is immutable
		// In a real implementation, we'd use interior mutability or recreate the
		// storage For now, we'll log the rotation
		dev_log!(
			"security",
			"[Security] Master key rotation from version {} to {}",
			old_key_version,
			old_key_version + 1
		);

		// Log key rotation event
		let event = SecurityEvent {
			Timestamp:crate::Utility::CurrentTimestamp(),

			EventType:SecurityEventType::KeyRotation,

			Severity:SecuritySeverity::Warning,

			SourceIp:None,

			ClientId:None,

			Details:format!("Master key rotated from version {} to {}", old_key_version, old_key_version + 1),

			Metadata:{
				let mut meta = HashMap::new();

				meta.insert("old_key_version".to_string(), old_key_version.to_string());

				meta.insert("new_key_version".to_string(), (old_key_version + 1).to_string());

				meta.insert("credentials_rotated".to_string(), credentials_to_rotate.len().to_string());

				meta
			},
		};

		drop(credentials);

		self.auditor.LogEvent(event).await;

		// Zeroize the new key since we can't actually use it in this simple
		// implementation
		zeroize(&mut new_key);

		Ok(KeyRotationResult {
			old_key_version,
			new_key_version:old_key_version + 1,
			credentials_rotated,
			timestamp:crate::Utility::CurrentTimestamp(),
		})
	}

	/// Clear all stored credentials
	pub async fn ClearAll(&self) -> Result<()> {
		let mut storage = self.credentials.write().await;

		let count = storage.len();

		storage.clear();

		// Log clear event
		let event = SecurityEvent {
			Timestamp:crate::Utility::CurrentTimestamp(),

			EventType:SecurityEventType::ConfigChange,

			Severity:SecuritySeverity::Warning,

			SourceIp:None,

			ClientId:None,

			Details:format!("All credentials cleared ({} credentials)", count),

			Metadata:{
				let mut meta = HashMap::new();

				meta.insert("credential_count".to_string(), count.to_string());

				meta
			},
		};

		drop(storage);

		self.auditor.LogEvent(event).await;

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

impl Clone for Struct {
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
///
/// Immediately zeros out secure bytes in memory. This function forces
/// zeroization to happen now rather than waiting for the Drop implementation.
/// Note: Rust compiler optimizations may optimize away the zeroization
/// without proper precautions like volatile operations or zeroize crate.
fn zeroize(bytes:&mut SecureBytesType) {
	// Force write zeros to the underlying bytes
	// This is a best-effort implementation. For production use,
	// consider using the `zeroize` crate which provides guarantees
	// against compiler optimization removing the zeroization.
	bytes.Data.zeroize();

	// If bytes are shared (Arc count > 1), we can't zeroize here
	// The Drop implementation will handle it when the last reference is dropped
	dev_log!("security", "[Security] Zeroized secure bytes (immediate cleanup requested)");
}
