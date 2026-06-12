use ring::signature;

/// Cryptographic keys
#[derive(Debug)]
pub struct CryptoKeys {
	pub SigningKey:ring::signature::Ed25519KeyPair,

	pub EncryptionKey:[u8; 32],
}
