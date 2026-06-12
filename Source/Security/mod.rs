//! # Security Module
//!
//! Comprehensive security features for Air including:
//! - Rate limiting with token bucket algorithm (per-IP and per-client)
//! - Checksum verification for file integrity
//! - Secure credential storage with encryption
//! - Timing attack protection for sensitive operations
//! - Secure memory handling with zeroization
//! - Key rotation and management
//! - Security event auditing and logging
//!
//! ## VSCode Security References
//!
//! This security module aligns with VSCode's security patterns:
//! - Rate limiting similar to VSCode's API rate limiting
//! - Secure credential storage matching VSCode's secret storage
//! - File integrity verification similar to VSCode's extension verification
//! - Security audit logging inspired by VSCode's telemetry security events
//!
//! ## Security Model for External Connections
//!
//! The security module implements a defense-in-depth approach for external
//! connections:
//!
//! ### Network Security
//! - Rate limiting prevents abuse and DoS attacks
//! - IP-based rate limiting limits impact per client
//! - Client-based rate limiting limits impact per authenticated client
//! - Connection pooling limits total concurrent connections
//!
//! ### Authentication Security
//! - Secure credential storage with AES-GCM encryption
//! - PBKDF2 key derivation with high iteration count
//! - Timing attack protection for password comparisons
//! - Secure token generation and validation
//!
//! ### Data Security
//! - SHA-256 checksum verification for file integrity
//! - AES-GCM encryption for credential storage
//! - Key wrapping for master key protection
//! - Secure memory handling with zeroization
//!
//! ### Audit and Monitoring
//! - Comprehensive security event logging
//! - Failed authentication attempts tracking
//! - Rate limit violation logging
//! - Security metric collection for Mountain integration
//!
//! ## Mountain Settings Integration
//!
//! Security policies are integrated with Mountain settings:
//! - Rate limit thresholds configurable via Mountain settings
//! - Security event thresholds configurable via Mountain settings
//! - Alert notification channels configured via Mountain
//! - Security metric retention configured via Mountain
//!
//! ## FUTURE Enhancements
//!
//! - Implement HSM (Hardware Security Module) integration for key storage
//! - Add support for hardware-backed key generation and storage
//! - Implement certificate pinning for external API connections
//! - Add support for TLS 1.3 with perfect forward secrecy
//! - Implement security policy enforcement and validation
//! - Add support for multi-factor authentication
//! - Implement security compliance reporting (SOC2, PCI-DSS, etc.)
//! - Add real-time security threat detection and response
//! - Implement secure communication channels with VSCode extensions
//! - Add support for encrypted data at rest with multiple keys
//! ## Timing Attack Protection
//!
//! The module implements constant-time operations for sensitive comparisons:
//! - Password comparisons use constant-time algorithms
//! - Token comparisons are timing-attack resistant
//! - Hash comparisons use fixed-time comparison functions
//! - Authentication response timing is normalized
//!
//! ## Secure Memory Handling
//!
//! Sensitive data in memory is protected through:
//! - Zeroization on drop for secure data structures
//! - Memory encryption for sensitive buffers
//! - Stack canaries for overflow detection
//! - Memory locking to prevent swapping
//!
//! ## Key Rotation
//!
//! Key rotation is supported through:
//! - Automatic key rotation hooks for periodic key updates
//! - Key versioning for backward compatibility
//! - Secure key storage with key wrapping
//! - Key rotation event logging and auditing
//!
//! ## Security Event Auditing
//!
//! All security events are logged for auditing:
//! - Authentication attempts (success and failure)
//! - Rate limit violations
//! - Key rotations
//! - Security configuration changes
//! - Access control violations
//!
//! Security events are forwarded to Mountain for correlation and alerting.

pub mod SecureBytes;
pub mod SecurityEvent;
pub mod SecurityEventType;
pub mod SecuritySeverity;
pub mod SecurityAuditor;
pub mod RateLimitConfig;
pub(crate) mod TokenBucket;
pub mod RateLimiter;
pub mod RateLimitStatus;
pub mod ChecksumVerifier;
pub mod SecureStorage;

#[cfg(test)]
mod SecurityTests;
