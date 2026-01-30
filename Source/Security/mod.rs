//! # Security Module
//!
//! Comprehensive security features for Air including:
//! - Rate limiting with token bucket algorithm (per-IP and per-client)
//! - Checksum verification for file integrity
//! - Secure credential storage with encryption

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use ring::pbkdf2;
use rand::Rng;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::{Result, AirError};

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Requests per second per IP
    pub requests_per_second_ip: u32,
    
    /// Requests per second per client
    pub requests_per_second_client: u32,
    
    /// Burst capacity (tokens)
    pub burst_capacity: u32,
    
    /// Token refill interval in milliseconds
    pub refill_interval_ms: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second_ip: 100,
            requests_per_second_client: 50,
            burst_capacity: 200,
            refill_interval_ms: 100,
        }
    }
}

/// Rate limit bucket for token bucket algorithm
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_rate: f64,
    last_refill: std::time::Instant,
}

impl TokenBucket {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            capacity,
            refill_rate,
            last_refill: std::time::Instant::now(),
        }
    }
    
    fn refill(&mut self) {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }
    
    fn try_consume(&mut self, tokens: f64) -> bool {
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
    config: RateLimitConfig,
    ip_buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    client_buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    cleanup_interval: std::time::Duration,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        let cleanup_interval = std::time::Duration::from_secs(300); // 5 minutes
        
        Self {
            config,
            ip_buckets: Arc::new(RwLock::new(HashMap::new())),
            client_buckets: Arc::new(RwLock::new(HashMap::new())),
            cleanup_interval,
        }
    }
    
    /// Check if request from IP is allowed
    pub async fn check_ip_rate_limit(&self, ip: &str) -> Result<bool> {
        let mut buckets = self.ip_buckets.write().await;
        
        let refill_rate = self.config.requests_per_second_ip as f64;
        let bucket = buckets.entry(ip.to_string())
            .or_insert_with(|| TokenBucket::new(self.config.burst_capacity as f64, refill_rate));
        
        Ok(bucket.try_consume(1.0))
    }
    
    /// Check if request from client is allowed
    pub async fn check_client_rate_limit(&self, client_id: &str) -> Result<bool> {
        let mut buckets = self.client_buckets.write().await;
        
        let refill_rate = self.config.requests_per_second_client as f64;
        let bucket = buckets.entry(client_id.to_string())
            .or_insert_with(|| TokenBucket::new(self.config.burst_capacity as f64, refill_rate));
        
        Ok(bucket.try_consume(1.0))
    }
    
    /// Check both IP and client rate limits
    pub async fn check_rate_limit(&self, ip: &str, client_id: &str) -> Result<bool> {
        let ip_allowed = self.check_ip_rate_limit(ip).await?;
        let client_allowed = self.check_client_rate_limit(client_id).await?;
        
        Ok(ip_allowed && client_allowed)
    }
    
    /// Get current rate limit status for IP
    pub async fn get_ip_status(&self, ip: &str) -> RateLimitStatus {
        let buckets = self.ip_buckets.read().await;
        
        if let Some(bucket) = buckets.get(ip) {
            RateLimitStatus {
                remaining_tokens: bucket.tokens as u32,
                capacity: bucket.capacity as u32,
                refill_rate: bucket.refill_rate as u32,
            }
        } else {
            RateLimitStatus {
                remaining_tokens: self.config.burst_capacity,
                capacity: self.config.burst_capacity,
                refill_rate: self.config.requests_per_second_ip,
            }
        }
    }
    
    /// Get current rate limit status for client
    pub async fn get_client_status(&self, client_id: &str) -> RateLimitStatus {
        let buckets = self.client_buckets.read().await;
        
        if let Some(bucket) = buckets.get(client_id) {
            RateLimitStatus {
                remaining_tokens: bucket.tokens as u32,
                capacity: bucket.capacity as u32,
                refill_rate: bucket.refill_rate as u32,
            }
        } else {
            RateLimitStatus {
                remaining_tokens: self.config.burst_capacity,
                capacity: self.config.burst_capacity,
                refill_rate: self.config.requests_per_second_client,
            }
        }
    }
    
    /// Clean up old buckets
    pub async fn cleanup_stale_buckets(&self) {
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
            config: self.config.clone(),
            ip_buckets: self.ip_buckets.clone(),
            client_buckets: self.client_buckets.clone(),
            cleanup_interval: self.cleanup_interval,
        }
    }
}

/// Rate limit status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStatus {
    pub remaining_tokens: u32,
    pub capacity: u32,
    pub refill_rate: u32,
}

/// Checksum verification for file integrity
pub struct ChecksumVerifier;

impl ChecksumVerifier {
    /// Calculate SHA-256 checksum of a file
    pub async fn calculate_sha256(&self, file_path: &std::path::Path) -> Result<String> {
        let content = tokio::fs::read(file_path).await
            .map_err(|e| AirError::FileSystem(format!("Failed to read file: {}", e)))?;
        
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let checksum = format!("{:x}", hasher.finalize());
        
        Ok(checksum)
    }
    
    /// Verify file checksum
    pub async fn verify_sha256(&self, file_path: &std::path::Path, expected_checksum: &str) -> Result<bool> {
        let actual = self.calculate_sha256(file_path).await?;
        Ok(actual == expected_checksum)
    }
    
    /// Calculate checksum from bytes
    pub fn calculate_sha256_bytes(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
    
    /// Calculate MD5 checksum (legacy support)
    pub async fn calculate_md5(&self, file_path: &std::path::Path) -> Result<String> {
        let content = tokio::fs::read(file_path).await
            .map_err(|e| AirError::FileSystem(format!("Failed to read file: {}", e)))?;
        
        let digest = md5::compute(&content);
        Ok(format!("{:x}", digest))
    }
}

/// Secure credential storage with encryption
pub struct SecureStorage {
    /// Encrypted credentials storage
    credentials: Arc<RwLock<HashMap<String, EncryptedCredential>>>,
    
    /// Master key for encryption/decryption
    master_key: Vec<u8>,
}

/// Encrypted credential
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedCredential {
    pub cipher_text: String,
    pub salt: String,
    pub nonce: String,
    pub created_at: u64,
}

impl SecureStorage {
    /// Create a new secure storage with a master key
    pub fn new(master_key: Vec<u8>) -> Self {
        Self {
            credentials: Arc::new(RwLock::new(HashMap::new())),
            master_key,
        }
    }
    
    /// Generate a secure master key from password
    pub fn derive_key_from_password(password: &[u8]) -> Vec<u8> {
        const N_ITERATIONS: u32 = 100_000;
        const CREDENTIAL_LEN: usize = 32;
        
        let mut salt = [0u8; 16];
        rand::rng().fill(&mut salt);
        
        let mut key = vec![0u8; CREDENTIAL_LEN];
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            std::num::NonZeroU32::new(N_ITERATIONS).unwrap(),
            &salt,
            password,
            &mut key,
        );
        
        key
    }
    
    /// Store a credential encrypted
    pub async fn store(&self, key: &str, credential: &str) -> Result<()> {
        let mut rng = rand::thread_rng();
        let mut nonce = [0u8; 12];
        rng.fill(&mut nonce);
        
        // Simple XOR encryption (in production, use AES-GCM)
        let cipher_text = self.encrypt_credential(credential, &nonce)?;
        let salt = base64::encode(&nonce);
        let nonce_str = base64::encode(&nonce);
        
        let encrypted = EncryptedCredential {
            cipher_text,
            salt,
            nonce: nonce_str,
            created_at: crate::utils::current_timestamp(),
        };
        
        let mut storage = self.credentials.write().await;
        storage.insert(key.to_string(), encrypted);
        
        Ok(())
    }
    
    /// Retrieve and decrypt a credential
    pub async fn retrieve(&self, key: &str) -> Result<Option<String>> {
        let storage = self.credentials.read().await;
        
        match storage.get(key) {
            Some(encrypted) => {
                let nonce = base64::decode(&encrypted.nonce)
                    .map_err(|e| AirError::Internal(format!("Failed to decode nonce: {}", e)))?;
                
                let credential = self.decrypt_credential(&encrypted.cipher_text, &nonce)?;
                Ok(Some(credential))
            }
            None => Ok(None),
        }
    }
    
    /// Delete a stored credential
    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut storage = self.credentials.write().await;
        storage.remove(key);
        Ok(())
    }
    
    /// Encrypt credential data
    fn encrypt_credential(&self, data: &str, nonce: &[u8]) -> Result<String> {
        let mut result = Vec::with_capacity(data.len());
        
        for (i, byte) in data.bytes().enumerate() {
            let key_byte = self.master_key[i % self.master_key.len()];
            let nonce_byte = nonce[i % nonce.len()];
            result.push(byte ^ key_byte ^ nonce_byte);
        }
        
        Ok(base64::encode(&result))
    }
    
    /// Decrypt credential data
    fn decrypt_credential(&self, cipher_text: &str, nonce: &[u8]) -> Result<String> {
        let encrypted_bytes = base64::decode(cipher_text)
            .map_err(|e| AirError::Internal(format!("Failed to decode cipher text: {}", e)))?;
        
        let mut result = Vec::with_capacity(encrypted_bytes.len());
        
        for (i, byte) in encrypted_bytes.iter().enumerate() {
            let key_byte = self.master_key[i % self.master_key.len()];
            let nonce_byte = nonce[i % nonce.len()];
            result.push(byte ^ key_byte ^ nonce_byte);
        }
        
        String::from_utf8(result)
            .map_err(|e| AirError::Internal(format!("Failed to decode decrypted data: {}", e)))
    }
    
    /// Clear all stored credentials
    pub async fn clear_all(&self) -> Result<()> {
        let mut storage = self.credentials.write().await;
        storage.clear();
        Ok(())
    }
}

impl Clone for SecureStorage {
    fn clone(&self) -> Self {
        Self {
            credentials: self.credentials.clone(),
            master_key: self.master_key.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_rate_limiter() {
        let config = RateLimitConfig::default();
        let limiter = RateLimiter::new(config);
        
        // Should allow requests within limit
        for _ in 0..50 {
            let allowed = limiter.check_ip_rate_limit("127.0.0.1").await.unwrap();
            assert!(allowed);
        }
        
        // After burst, should eventually deny
        let mut denied_count = 0;
        for _ in 0..200 {
            if !limiter.check_ip_rate_limit("127.0.0.1").await.unwrap() {
                denied_count += 1;
            }
        }
        assert!(denied_count > 0);
    }
    
    #[tokio::test]
    async fn test_checksum_verification() {
        let verifier = ChecksumVerifier;
        let data = b"test data";
        let checksum = verifier.calculate_sha256_bytes(data);
        
        assert_eq!(checksum.len(), 64); // SHA-256 hex is 64 chars
        assert!(!checksum.is_empty());
    }
    
    #[tokio::test]
    async fn test_secure_storage() {
        let master_key = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let storage = SecureStorage::new(master_key);
        
        storage.store("test_key", "secret_value").await.unwrap();
        let retrieved = storage.retrieve("test_key").await.unwrap();
        
        assert_eq!(retrieved, Some("secret_value".to_string()));
    }
}
