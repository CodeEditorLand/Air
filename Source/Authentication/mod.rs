//! # Authentication Service
//!
//! Handles user authentication, token management, and cryptographic operations
//! for the Air daemon. This service manages secure storage of credentials
//! and provides authentication services to Mountain with resilient patterns.

pub mod AuthSession;

pub mod CredentialsStore;

pub mod CryptoKeys;

pub mod AuthenticationService;
