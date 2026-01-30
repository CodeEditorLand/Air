//! # Authentication Service
//!
//! Handles user authentication, token management, and cryptographic operations
//! for the Air daemon. This service manages secure storage of credentials
//! and provides authentication services to Mountain with resilient patterns.
//!
//! ## File Responsibilities
//! - User authentication and session management
//! - Cryptographic key generation and management
//! - Secure credential storage and encryption
//! - Token generation and validation
//! - Background session cleanup and maintenance
//! - Integration with application state management
//! - Error handling and recovery patterns
//!
//! ## TODO
//! - [ ] Implement multi-factor authentication support
//! - [ ] Add OAuth2 and OpenID Connect integration
//! - [ ] Implement biometric authentication support
//! - [ ] Add password policy enforcement
//! - [ ] Implement session timeout and renewal
//! - [ ] Add audit logging for authentication events
//! - [ ] Implement secure password reset mechanisms
//! - [ ] Add support for hardware security modules
//! - [ ] Implement rate limiting and brute force protection
//! - [ ] Add comprehensive security testing
//! - [ ] Implement proper key rotation and management
//! - [ ] Add support for certificate-based authentication
//! - [ ] Implement secure session storage
//! - [ ] Add support for federated identity providers
//! - [ ] Implement proper error handling and recovery
//! - [ ] Add performance optimization and caching

use std::{collections::HashMap, sync::Arc};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use ring::rand::SecureRandom;
use ring::aead;

use crate::{ApplicationState::ApplicationState, Result, AirError, Configuration::ConfigurationManager, utils};

/// Authentication service implementation
pub struct AuthenticationService {
    /// Application state
    app_state: Arc<ApplicationState>,
    
    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, AuthSession>>>,
    
    /// Credentials storage
    credentials: Arc<Mutex<CredentialsStore>>,
    
    /// Cryptographic keys
    crypto_keys: Arc<Mutex<CryptoKeys>>,
    /// AEAD algorithm for encryption/decryption
    aead_algo: &'static aead::Algorithm,
}

/// Authentication session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub session_id: String,
    pub user_id: String,
    pub provider: String,
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_valid: bool,
}

/// Credentials storage
#[derive(Debug, Serialize, Deserialize)]
struct CredentialsStore {
    credentials: HashMap<String, UserCredentials>,
    file_path: String,
}

/// User credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCredentials {
    pub user_id: String,
    pub provider: String,
    pub encrypted_password: String,
    pub last_used: DateTime<Utc>,
    pub is_valid: bool,
}

/// Cryptographic keys
#[derive(Debug)]
struct CryptoKeys {
    signing_key: ring::signature::Ed25519KeyPair,
    encryption_key: [u8; 32],
}

impl AuthenticationService {
    /// Create a new authentication service
    pub async fn new(app_state: Arc<ApplicationState>) -> Result<Self> {
        let config = &app_state.configuration.authentication;
        
        // Expand credentials path
        let credentials_path = ConfigurationManager::expand_path(&config.credentials_path)?;
        
        // Load or create credentials store
        let credentials_store = Self::load_credentials_store(&credentials_path).await?;
        
        // Generate cryptographic keys
        let crypto_keys = Self::generate_crypto_keys()?;
        let aead_algo = &aead::AES_256_GCM;
        
        let service = Self {
            app_state,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            credentials: Arc::new(Mutex::new(credentials_store)),
            crypto_keys: Arc::new(Mutex::new(crypto_keys)),
            aead_algo,
        };
        
        // Initialize service status
        service.app_state.update_service_status("authentication", crate::ApplicationState::ServiceStatus::Running)
            .await
            .map_err(|e| AirError::Authentication(e.to_string()))?;
        
        Ok(service)
    }
    
    /// Authenticate a user
    pub async fn authenticate_user(&self, username: String, password: String, provider: String) -> Result<String> {
        // Validate input
        if username.is_empty() || password.is_empty() || provider.is_empty() {
            return Err(AirError::Authentication("Invalid authentication parameters".to_string()));
        }
        
        // Check credentials
        let _user_credentials = self.validate_credentials(&username, &password, &provider).await?;
        
        // Generate session token
        let token = self.generate_session_token(&username, &provider).await?;
        
        // Create session
        let session = AuthSession {
            session_id: utils::GenerateRequestId(),
            user_id: username.clone(),
            provider: provider.clone(),
            token: token.clone(),
            created_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(self.app_state.configuration.authentication.token_expiration_hours as i64),
            is_valid: true,
        };
        
        // Store session
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session.session_id.clone(), session);
        }
        
        // Update credentials usage
        self.update_credentials_usage(&username, &provider).await?;
        
        Ok(token)
    }
    
    /// Validate user credentials
    async fn validate_credentials(&self, username: &str, password: &str, provider: &str) -> Result<UserCredentials> {
        let credentials_store = self.credentials.lock().await;
        
        let key = format!("{}:{}", provider, username);
        
        if let Some(user_credentials) = credentials_store.credentials.get(&key) {
            if !user_credentials.is_valid {
                return Err(AirError::Authentication("Credentials are invalid".to_string()));
            }
            
            // Verify password (in a real implementation, this would decrypt and verify)
            // For now, we'll use a simple approach
            let decrypted_password = self.decrypt_password(&user_credentials.encrypted_password).await?;
            
            if decrypted_password == password {
                Ok(user_credentials.clone())
            } else {
                Err(AirError::Authentication("Invalid password".to_string()))
            }
        } else {
            Err(AirError::Authentication("User not found".to_string()))
        }
    }
    
    /// Generate a session token
    async fn generate_session_token(&self, username: &str, provider: &str) -> Result<String> {
        let crypto_keys = self.crypto_keys.lock().await;
        
        let payload = format!("{}:{}:{}", username, provider, utils::CurrentTimestamp());
        
        // Sign the payload
        let signature = crypto_keys.signing_key.sign(payload.as_bytes());
        
        // Encode token
        let token = URL_SAFE.encode(
            format!("{}:{}", payload, URL_SAFE.encode(signature.as_ref()))
        );
        
        Ok(token)
    }
    
    /// Update credentials usage timestamp
    async fn update_credentials_usage(&self, username: &str, provider: &str) -> Result<()> {
        let mut credentials_store = self.credentials.lock().await;
        
        let key = format!("{}:{}", provider, username);
        
        if let Some(user_credentials) = credentials_store.credentials.get_mut(&key) {
            user_credentials.last_used = Utc::now();
        }
        
        // Save updated credentials
        self.save_credentials_store(&credentials_store).await?;
        
        Ok(())
    }
    
    /// Encrypt password
    async fn encrypt_password(&self, password: &str) -> Result<String> {
        let crypto_keys = self.crypto_keys.lock().await;

        // Use AES-256-GCM via ring::aead. Prefix nonce to ciphertext and base64 encode.
        let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, &crypto_keys.encryption_key)
            .map_err(|e| AirError::Authentication(format!("Failed to create AEAD key: {:?}", e)))?;

        let less_safe = aead::LessSafeKey::new(unbound_key);
        let mut nonce_bytes = [0u8; 12];
        ring::rand::SystemRandom::new().fill(&mut nonce_bytes)
            .map_err(|e| AirError::Authentication(format!("Failed to generate nonce: {:?}", e)))?;

        let nonce = aead::Nonce::assume_unique_for_key(nonce_bytes);

        let mut in_out = password.as_bytes().to_vec();
        // Reserve space for tag
        in_out.extend_from_slice(&[0u8; 16]); // AES_256_GCM tag length is 16 bytes

        less_safe.seal_in_place_append_tag(nonce, aead::Aad::empty(), &mut in_out)
            .map_err(|e| AirError::Authentication(format!("Encryption failed: {:?}", e)))?;

        // Store nonce + ciphertext
        let mut out = Vec::with_capacity(nonce_bytes.len() + in_out.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&in_out);

        Ok(URL_SAFE.encode(&out))
    }
    
    /// Decrypt password
    async fn decrypt_password(&self, encrypted_password: &str) -> Result<String> {
        let crypto_keys = self.crypto_keys.lock().await;

        let data = URL_SAFE.decode(encrypted_password)
            .map_err(|e| AirError::Authentication(format!("Failed to decode password: {}", e)))?;

        if data.len() < 12 + aead::AES_256_GCM.tag_len() {
            return Err(AirError::Authentication("Encrypted data too short".to_string()));
        }

        let (nonce_bytes, cipher_bytes) = data.split_at(12);

        let mut nonce_arr = [0u8; 12];
        nonce_arr.copy_from_slice(&nonce_bytes[0..12]);

        let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, &crypto_keys.encryption_key)
            .map_err(|e| AirError::Authentication(format!("Failed to create AEAD key: {:?}", e)))?;

        let less_safe = aead::LessSafeKey::new(unbound_key);
        let nonce = aead::Nonce::assume_unique_for_key(nonce_arr);

        let mut cipher_vec = cipher_bytes.to_vec();
        let plain = less_safe.open_in_place(nonce, aead::Aad::empty(), &mut cipher_vec)
            .map_err(|e| AirError::Authentication(format!("Decryption failed: {:?}", e)))?;

        String::from_utf8(plain.to_vec())
            .map_err(|e| AirError::Authentication(format!("Failed to decode password string: {}", e)))
    }
    
    /// Load credentials store from file
    async fn load_credentials_store(file_path: &std::path::Path) -> Result<CredentialsStore> {
        if file_path.exists() {
            let content = tokio::fs::read_to_string(file_path).await
                .map_err(|e| AirError::Authentication(format!("Failed to read credentials file: {}", e)))?;
            
            let credentials: HashMap<String, UserCredentials> = serde_json::from_str(&content)
                .map_err(|e| AirError::Authentication(format!("Failed to parse credentials file: {}", e)))?;
            
            Ok(CredentialsStore {
                credentials,
                file_path: file_path.to_string_lossy().to_string(),
            })
        } else {
            // Create new credentials store
            Ok(CredentialsStore {
                credentials: HashMap::new(),
                file_path: file_path.to_string_lossy().to_string(),
            })
        }
    }
    
    /// Save credentials store to file
    async fn save_credentials_store(&self, store: &CredentialsStore) -> Result<()> {
        let content = serde_json::to_string_pretty(&store.credentials)
            .map_err(|e| AirError::Authentication(format!("Failed to serialize credentials: {}", e)))?;
        
        // Create directory if it doesn't exist
        if let Some(parent) = std::path::Path::new(&store.file_path).parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| AirError::Authentication(format!("Failed to create credentials directory: {}", e)))?;
        }
        
        tokio::fs::write(&store.file_path, content).await
            .map_err(|e| AirError::Authentication(format!("Failed to write credentials file: {}", e)))?;
        
        Ok(())
    }
    
    /// Generate cryptographic keys
    fn generate_crypto_keys() -> Result<CryptoKeys> {
        // Generate signing key
        let rng = ring::rand::SystemRandom::new();
        let pkcs8_bytes = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng)
            .map_err(|e| AirError::Authentication(format!("Failed to generate signing key: {}", e)))?;
        
        let signing_key = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref())
            .map_err(|e| AirError::Authentication(format!("Failed to load signing key: {}", e)))?;
        
        // Generate encryption key
        let mut encryption_key = [0u8; 32];
        ring::rand::SystemRandom::new().fill(&mut encryption_key).map_err(|e| AirError::Authentication(format!("Failed to generate encryption key: {}", e)))
            .map_err(|e| AirError::Authentication(format!("Failed to generate encryption key: {}", e)))?;
        
        Ok(CryptoKeys {
            signing_key,
            encryption_key,
        })
    }
    
    /// Start background tasks
    pub async fn start_background_tasks(&self) -> Result<tokio::task::JoinHandle<()>> {
        let service = self.clone();
        
        let handle = tokio::spawn(async move {
            service.background_task().await;
        });
        
        Ok(handle)
    }
    
    /// Background task for session cleanup
    async fn background_task(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes
        
        loop {
            interval.tick().await;
            
            // Clean up expired sessions
            self.cleanup_expired_sessions().await;
            
            // Save credentials periodically
            if let Err(e) = self.save_credentials_periodically().await {
                log::error!("[Authentication] Failed to save credentials: {}", e);
            }
        }
    }
    
    /// Clean up expired sessions
    async fn cleanup_expired_sessions(&self) {
        let now = Utc::now();
        let mut sessions = self.sessions.write().await;
        
        sessions.retain(|_, session| session.expires_at > now && session.is_valid);
        
        log::debug!("[Authentication] Cleaned up expired sessions");
    }
    
    /// Save credentials periodically
    async fn save_credentials_periodically(&self) -> Result<()> {
        let credentials_store = self.credentials.lock().await;
        self.save_credentials_store(&credentials_store).await
    }
    
    /// Stop background tasks
    pub async fn stop_background_tasks(&self) {
        // Implementation for graceful shutdown
        log::info!("[Authentication] Stopping background tasks");
    }
}

impl Clone for AuthenticationService {
    fn clone(&self) -> Self {
        Self {
            app_state: self.app_state.clone(),
            sessions: self.sessions.clone(),
            credentials: self.credentials.clone(),
            crypto_keys: self.crypto_keys.clone(),
            aead_algo: self.aead_algo,
        }
    }
}
