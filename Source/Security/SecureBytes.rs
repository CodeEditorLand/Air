use serde::{Deserialize, Serialize};
use zeroize::Zeroize;
use subtle::ConstantTimeEq;

/// Secure byte array that zeroizes memory on drop
#[derive(Clone, Deserialize, Serialize)]
pub struct SecureBytes {
	/// The underlying bytes
	pub(crate) Data:Vec<u8>,
}

impl SecureBytes {
	/// Create a new secure byte array
	pub fn new(Data:Vec<u8>) -> Self { Self { Data } }

	/// Create from a string
	pub fn from_str(S:&str) -> Self { Self { Data:S.as_bytes().to_vec() } }

	/// Get the data as a slice (constant-time)
	pub fn as_slice(&self) -> &[u8] { &self.Data }

	/// Get the length
	pub fn len(&self) -> usize { self.Data.len() }

	/// Check if empty
	pub fn is_empty(&self) -> bool { self.Data.is_empty() }

	/// Constant-time comparison
	pub fn ct_eq(&self, Other:&Self) -> bool { self.Data.ct_eq(&Other.Data).into() }
}

impl Drop for SecureBytes {
	fn drop(&mut self) { self.Data.zeroize(); }
}
