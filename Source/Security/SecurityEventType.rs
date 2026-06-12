use serde::{Deserialize, Serialize};

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
