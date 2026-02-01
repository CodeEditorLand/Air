//! # Authentication Service
//!
//! Handles user authentication, token management, and cryptographic operations
//! for the Air daemon. This service manages secure storage of credentials
//! and provides authentication services to Mountain with resilient patterns.

use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use base64::{Engine as _, engine::general_purpose::URL_SAFE};
use ring::{aead, rand::SecureRandom};

use crate::{AirError, ApplicationState::ApplicationState, Configuration::ConfigurationManager, Result, utils};

/// Authentication service implementation
pub struct AuthenticationService {
	/// Application state
	app_state:Arc<ApplicationState>,

	/// Active sessions
	sessions:Arc<RwLock<HashMap<String, AuthSession>>>,

	/// Credentials storage
	credentials:Arc<Mutex<CredentialsStore>>,

	/// Cryptographic keys
	crypto_keys:Arc<Mutex<CryptoKeys>>,
	/// AEAD algorithm for encryption/decryption
	aead_algo:&'static aead::Algorithm,
}

/// Authentication session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
	pub session_id:String,
	pub user_id:String,
	pub provider:String,
	pub token:String,
	pub created_at:DateTime<Utc>,
	pub expires_at:DateTime<Utc>,
	pub is_valid:bool,
}

/// Credentials storage
#[derive(Debug, Serialize, Deserialize)]
struct CredentialsStore {
	credentials:HashMap<String, UserCredentials>,
	file_path:String,
}

/// User credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCredentials {
	pub user_id:String,
	pub provider:String,
	pub encrypted_password:String,
	pub last_used:DateTime<Utc>,
	pub is_valid:bool,
}

/// Cryptographic keys
#[derive(Debug)]
struct CryptoKeys {
	signing_key:ring::signature::Ed25519KeyPair,
	encryption_key:[u8; 32],
}

impl AuthenticationService {
	/// Create a new authentication service
	pub async fn new(app_state:Arc<ApplicationState>) -> Result<Self> {
		let config = &app_state.configuration.authentication;

		// Expand credentials path
		let CredentialsPath = ConfigurationManager::ExpandPath(&config.credentials_path)?;

		// Load or create credentials store
		let CredentialsStore = Self::LoadCredentialsStore(&CredentialsPath).await?;

		// Generate cryptographic keys
		let CryptoKeys = Self::GenerateCryptoKeys()?;
		let aead_algo = &aead::AES_256_GCM;

		let Service = Self {
			app_state,
			sessions:Arc::new(RwLock::new(HashMap::new())),
			credentials:Arc::new(Mutex::new(CredentialsStore)),
			crypto_keys:Arc::new(Mutex::new(CryptoKeys)),
			aead_algo,
		};

		// Initialize service status
		Service
			.app_state
			.UpdateServiceStatus("authentication", crate::ApplicationState::ServiceStatus::Running)
			.await
			.map_err(|e| AirError::Authentication(e.to_string()))?;

		Ok(Service)
	}

	/// Authenticate a user
	pub async fn AuthenticateUser(&self, Username:String, Password:String, Provider:String) -> Result<String> {
		// Validate input
		if Username.is_empty() || Password.is_empty() || Provider.is_empty() {
			return Err(AirError::Authentication("Invalid authentication parameters".to_string()));
		}

		// Check credentials
		let _UserCredentials = self.ValidateCredentials(&Username, &Password, &Provider).await?;

		// Generate session token
		let Token = self.GenerateSessionToken(&Username, &Provider).await?;

		// Create session
		let SessionId = utils::GenerateRequestId();
		let Session = AuthSession {
			session_id:SessionId,
			user_id:Username.clone(),
			provider:Provider.clone(),
			token:Token.clone(),
			created_at:chrono::Utc::now(),
			expires_at:chrono::Utc::now()
				+ chrono::Duration::hours(self.app_state.configuration.authentication.token_expiration_hours as i64),
			is_valid:true,
		};

		// Store session
		{
			let mut Sessions = self.sessions.write().await;
			Sessions.insert(Session.session_id.clone(), Session);
		}

		// Update credentials usage
		self.UpdateCredentialsUsage(&Username, &Provider).await?;

		Ok(Token)
	}

	/// Validate user credentials
	async fn ValidateCredentials(&self, Username:&str, Password:&str, Provider:&str) -> Result<UserCredentials> {
		let CredentialsStore = self.credentials.lock().await;

		let Key = format!("{}:{}", Provider, Username);

		if let Some(UserCredentials) = CredentialsStore.credentials.get(&Key) {
			if !UserCredentials.is_valid {
				return Err(AirError::Authentication("Credentials are invalid".to_string()));
			}

			// Verify password (in a real implementation, this would decrypt and verify)
			// For now, we'll use a simple approach
			let DecryptedPassword = self.DecryptPassword(&UserCredentials.encrypted_password).await?;

			if DecryptedPassword == Password {
				Ok(UserCredentials.clone())
			} else {
				Err(AirError::Authentication("Invalid password".to_string()))
			}
		} else {
			Err(AirError::Authentication("User not found".to_string()))
		}
	}

	/// Generate a session token
	async fn GenerateSessionToken(&self, Username:&str, Provider:&str) -> Result<String> {
		let CryptoKeys = self.crypto_keys.lock().await;

		let Payload = format!("{}:{}:{}", Username, Provider, utils::CurrentTimestamp());

		// Sign the payload
		let Signature = CryptoKeys.signing_key.sign(Payload.as_bytes());

		// Encode token
		let Token = URL_SAFE.encode(format!("{}:{}", Payload, URL_SAFE.encode(Signature.as_ref())));

		Ok(Token)
	}

	/// Update credentials usage timestamp
	async fn UpdateCredentialsUsage(&self, Username:&str, Provider:&str) -> Result<()> {
		let mut CredentialsStore = self.credentials.lock().await;

		let Key = format!("{}:{}", Provider, Username);

		if let Some(UserCredentials) = CredentialsStore.credentials.get_mut(&Key) {
			UserCredentials.last_used = Utc::now();
		}

		// Save updated credentials
		self.SaveCredentialsStore(&CredentialsStore).await?;

		Ok(())
	}

	/// Encrypt password
	async fn EncryptPassword(&self, Password:&str) -> Result<String> {
		let CryptoKeys = self.crypto_keys.lock().await;

		// Use AES-256-GCM via ring::aead. Prefix nonce to ciphertext and base64 encode.
		let UnboundKey = aead::UnboundKey::new(&aead::AES_256_GCM, &CryptoKeys.encryption_key)
			.map_err(|e| AirError::Authentication(format!("Failed to create AEAD key: {:?}", e)))?;

		let LessSafe = aead::LessSafeKey::new(UnboundKey);
		let mut NonceBytes = [0u8; 12];
		ring::rand::SystemRandom::new()
			.fill(&mut NonceBytes)
			.map_err(|e| AirError::Authentication(format!("Failed to generate nonce: {:?}", e)))?;

		let Nonce = aead::Nonce::assume_unique_for_key(NonceBytes);

		let mut InOut = Password.as_bytes().to_vec();
		// Reserve space for tag
		InOut.extend_from_slice(&[0u8; 16]); // AES_256_GCM tag length is 16 bytes

		LessSafe
			.seal_in_place_append_tag(Nonce, aead::Aad::empty(), &mut InOut)
			.map_err(|e| AirError::Authentication(format!("Encryption failed: {:?}", e)))?;

		// Store nonce + ciphertext
		let mut Out = Vec::with_capacity(NonceBytes.len() + InOut.len());
		Out.extend_from_slice(&NonceBytes);
		Out.extend_from_slice(&InOut);

		Ok(URL_SAFE.encode(&Out))
	}

	/// Decrypt password
	async fn DecryptPassword(&self, EncryptedPassword:&str) -> Result<String> {
		let CryptoKeys = self.crypto_keys.lock().await;

		let Data = URL_SAFE
			.decode(EncryptedPassword)
			.map_err(|e| AirError::Authentication(format!("Failed to decode password: {}", e)))?;

		if Data.len() < 12 + aead::AES_256_GCM.tag_len() {
			return Err(AirError::Authentication("Encrypted data too short".to_string()));
		}

		let (NonceBytes, CipherBytes) = Data.split_at(12);

		let mut NonceArr = [0u8; 12];
		NonceArr.copy_from_slice(&NonceBytes[0..12]);

		let UnboundKey = aead::UnboundKey::new(&aead::AES_256_GCM, &CryptoKeys.encryption_key)
			.map_err(|e| AirError::Authentication(format!("Failed to create AEAD key: {:?}", e)))?;

		let LessSafe = aead::LessSafeKey::new(UnboundKey);
		let Nonce = aead::Nonce::assume_unique_for_key(NonceArr);

		let mut CipherVec = CipherBytes.to_vec();
		let Plain = LessSafe
			.open_in_place(Nonce, aead::Aad::empty(), &mut CipherVec)
			.map_err(|e| AirError::Authentication(format!("Decryption failed: {:?}", e)))?;

		String::from_utf8(Plain.to_vec())
			.map_err(|e| AirError::Authentication(format!("Failed to decode password string: {}", e)))
	}

	/// Load credentials store from file
	async fn LoadCredentialsStore(FilePath:&std::path::Path) -> Result<CredentialsStore> {
		if FilePath.exists() {
			let Content = tokio::fs::read_to_string(FilePath)
				.await
				.map_err(|e| AirError::Authentication(format!("Failed to read credentials file: {}", e)))?;

			let Credentials:HashMap<String, UserCredentials> = serde_json::from_str(&Content)
				.map_err(|e| AirError::Authentication(format!("Failed to parse credentials file: {}", e)))?;

			Ok(CredentialsStore { credentials:Credentials, file_path:FilePath.to_string_lossy().to_string() })
		} else {
			// Create new credentials store
			Ok(CredentialsStore { credentials:HashMap::new(), file_path:FilePath.to_string_lossy().to_string() })
		}
	}

	/// Save credentials store to file
	async fn SaveCredentialsStore(&self, Store:&CredentialsStore) -> Result<()> {
		let Content = serde_json::to_string_pretty(&Store.credentials)
			.map_err(|e| AirError::Authentication(format!("Failed to serialize credentials: {}", e)))?;

		// Create directory if it doesn't exist
		if let Some(Parent) = std::path::Path::new(&Store.file_path).parent() {
			tokio::fs::create_dir_all(Parent)
				.await
				.map_err(|e| AirError::Authentication(format!("Failed to create credentials directory: {}", e)))?;
		}

		tokio::fs::write(&Store.file_path, Content)
			.await
			.map_err(|e| AirError::Authentication(format!("Failed to write credentials file: {}", e)))?;

		Ok(())
	}

	/// Generate cryptographic keys
	fn GenerateCryptoKeys() -> Result<CryptoKeys> {
		// Generate signing key
		let Rng = ring::rand::SystemRandom::new();
		let Pkcs8Bytes = ring::signature::Ed25519KeyPair::generate_pkcs8(&Rng)
			.map_err(|e| AirError::Authentication(format!("Failed to generate signing key: {}", e)))?;

		let SigningKey = ring::signature::Ed25519KeyPair::from_pkcs8(Pkcs8Bytes.as_ref())
			.map_err(|e| AirError::Authentication(format!("Failed to load signing key: {}", e)))?;

		// Generate encryption key
		let mut EncryptionKey = [0u8; 32];
		ring::rand::SystemRandom::new()
			.fill(&mut EncryptionKey)
			.map_err(|e| AirError::Authentication(format!("Failed to generate encryption key: {}", e)))
			.map_err(|e| AirError::Authentication(format!("Failed to generate encryption key: {}", e)))?;

		Ok(CryptoKeys { signing_key:SigningKey, encryption_key:EncryptionKey })
	}

	/// Start background tasks
	pub async fn StartBackgroundTasks(&self) -> Result<tokio::task::JoinHandle<()>> {
		let Service = self.clone();

		let Handle = tokio::spawn(async move {
			Service.BackgroundTask().await;
		});

		Ok(Handle)
	}

	/// Background task for session cleanup
	async fn BackgroundTask(&self) {
		let mut Interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes

		loop {
			Interval.tick().await;

			// Clean up expired sessions
			self.CleanupExpiredSessions().await;

			// Save credentials periodically
			if let Err(E) = self.SaveCredentialsPeriodically().await {
				log::error!("[Authentication] Failed to save credentials: {}", E);
			}
		}
	}

	/// Clean up expired sessions
	async fn CleanupExpiredSessions(&self) {
		let Now = Utc::now();
		let mut Sessions = self.sessions.write().await;

		Sessions.retain(|_, Session| Session.expires_at > Now && Session.is_valid);

		log::debug!("[Authentication] Cleaned up expired sessions");
	}

	/// Save credentials periodically
	async fn SaveCredentialsPeriodically(&self) -> Result<()> {
		let CredentialsStore = self.credentials.lock().await;
		self.SaveCredentialsStore(&CredentialsStore).await
	}

	/// Stop background tasks
	pub async fn StopBackgroundTasks(&self) {
		// Implementation for graceful shutdown
		log::info!("[Authentication] Stopping background tasks");
	}
}

impl Clone for AuthenticationService {
	fn clone(&self) -> Self {
		Self {
			app_state:self.app_state.clone(),
			sessions:self.sessions.clone(),
			credentials:self.credentials.clone(),
			crypto_keys:self.crypto_keys.clone(),
			aead_algo:self.aead_algo,
		}
	}
}
