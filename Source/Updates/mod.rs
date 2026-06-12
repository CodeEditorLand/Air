//! # Update Management Service
//!
//! Comprehensive update management for the Land ecosystem with full lifecycle
//! support:
//! - Version availability checking against update servers
//! - Secure download with cryptographic signature verification
//! - Multi-checksum integrity validation (SHA256, MD5, CRC32)
//! - Staged installation with atomic application
//! - Automatic rollback on installation failure
//! - Platform-specific update packages (macOS dmg, Windows exe, Linux AppImage)
//! - Update channel management (stable, insiders, preview)
//! - Delta updates for reduced download size
//! - Network interruption recovery with resume capability
//! - Disk space validation before download
//! - Backup creation before applying updates
//!
//! # VSCode Update System References
//! This implementation draws inspiration from VSCode's update architecture:
//! - Background update checking without interrupting user workflow
//! - Deferred installation at application restart
//! - Update verification with multiple checksums
//! - Telemetry for update success/failure tracking
//! - Circuit breakers for update server resilience
//!
//! # Architecture
//! The update manager coordinates with:
//! - Mountain: Provides frontend notification of available updates
//! - The entire Land ecosystem: Updates can apply to multiple components
//! - Update servers: REST API endpoints for version checks and downloads
//!
//! # Connection to VSCode's Update Download Service Architecture
//!
//! The Air update manager draws inspiration from VSCode's update system:
//! 1. **Separate Update Process**: Like VSCode, Air runs updates in the
//!    background
//! 2. **Deferred Installation**: Updates are downloaded and staged, then
//!    applied on restart
//! 3. **Progress Reporting**: Updates report progress to the frontend
//!    (Mountain)
//! 4. **Resilient Downloads**: Support for resume after interruption
//! 5. **Multiple Integrity Checks**: SHA256, MD5, and cryptographic signatures
//! 6. **Automatic Rollback**: Like VSCode's safe mode, Air can roll back on
//!    failure
//!
//! Air handles updates for the entire Land ecosystem:
//! - **Air daemon**: The background service itself
//! - **Mountain**: The frontend Electron application
//! - **Other elements**: Can update other Land components if needed
//!
//! Update coordination with Mountain:
//! - When an update is available, Air notifies Mountain via event bus
//! - Mountain displays update notification to the user
//! - User selects whether to download/install/update
//! - Mountain can request status, cancel downloads, or initiate installation
//!
//! ## VSCode Update System Reference
//!
//! VSCode's update system (similar to Atom's) uses:
//! - Electron's auto-updater module for Windows/macOS AppImages
//! - Native Linux package managers for deb/rpm
//! - Background update checking without interrupting user
//! - Deferred installation on application restart
//! - Multi-channel support (stable, insiders, exploration)
//!
//! # FUTURE Enhancements
//! - Delta update support: Download only changed files between versions
//! - Rollback system: Automatic and manual rollback to previous versions
//! - Staged installations: Pre-verify updates before user prompt
//! - Update signatures: Ed25519 or PGP signature verification
//! - Update package format: Custom package format for cross-platform support

pub mod ChecksumUtil;

pub mod DownloadSession;

pub mod InstallationStatus;

pub mod PackageFormat;

pub mod PlatformConfig;

pub mod PlatformDetect;

pub mod PlatformMetadata;

pub mod RollbackHistory;

pub mod RollbackState;

pub mod Types;

pub mod UpdateChannel;

pub mod UpdateInfo;

pub mod UpdateManager;

pub mod UpdateStatus;

pub mod UpdateTelemetry;

pub mod VersionCompare;
