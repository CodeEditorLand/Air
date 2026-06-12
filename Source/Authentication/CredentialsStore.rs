use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::Authentication::AuthSession::UserCredentials;

/// Credentials storage
#[derive(Debug, Serialize, Deserialize)]
pub struct CredentialsStore {
	pub Credentials:HashMap<String, UserCredentials>,

	pub FilePath:String,
}
