use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Authentication session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
	pub SessionId:String,

	pub UserId:String,

	pub Provider:String,

	pub Token:String,

	pub CreatedAt:DateTime<Utc>,

	pub ExpiresAt:DateTime<Utc>,

	pub IsValid:bool,
}

/// User credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCredentials {
	pub UserId:String,

	pub Provider:String,

	pub EncryptedPassword:String,

	pub LastUsed:DateTime<Utc>,

	pub IsValid:bool,
}
