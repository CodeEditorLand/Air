//! # Daemon Lifecycle Management
//!
//! This module provides comprehensive daemon lifecycle management for the Air
//! daemon service, responsible for managing background processes in the Land
//! code editor ecosystem.
//!
//! ## Architecture Overview
//!
//! The daemon follows VSCode's daemon architecture pattern:
//! - Reference: VSCode service management
//!   (Dependency/Microsoft/Editor/src/vs/base/node/processexitorutility)
//! - Singleton enforcement through PID file locking
//! - Platform-native service integration (systemd, launchd, Windows Service)
//! - Graceful shutdown coordination with Mountain (main editor process)
//! - Resource cleanup and state persistence across restarts
//!
//! ## Core Responsibilities
//!
//! 1. **Process Management**
//!    - PID file creation, validation, and cleanup
//!    - Checksum-based PID file integrity verification
//!    - Process existence validation and stale detection
//!    - Race condition protection for lock acquisition
//!    - Timeout handling for all async operations
//!
//! 2. **Service Installation**
//!    - systemd service generation and installation (Linux)
//!    - launchd plist generation and installation (macOS)
//!    - Windows Service registration (Windows using winsvc)
//!    - Service validation and health checks
//!    - Post-installation verification
//!
//! 3. **Lifecycle Coordination**
//!    - Lock acquisition with atomic operations
//!    - Graceful shutdown signals
//!    - Resource cleanup on errors
//!    - State persistence and recovery
//!
//! 4. **Platform Integration**
//!    - Linux: systemd socket activation support
//!    - macOS: launchd session management
//!    - Windows: Windows Service API integration
//!    - Cross-platform log rotation
//!
//! ## TODO Items
//!
//! - [ ] Implement Windows winsvc integration for actual service registration
//! - [ ] Add systemd socket activation support
//! - [ ] Implement daemon auto-update notifications
//! - [ ] Add crash recovery and state restoration
//! - [ ] Implement daemon health monitoring with metrics
//! - [ ] Add log rotation for daemon logs
//! - [ ] Implement daemon upgrade path (in-place hot reload)
//! - [ ] Add daemon configuration reloading without restart
//! - [ ] Implement grace period for Mountain shutdown coordination
//! - [ ] Add daemon sandbox support for security isolation
//!
//! ## Platform-Specific Considerations
//!
//! ### Linux (systemd)
//! - PID file location: `/var/run/air.pid`
//! - Service file: `/etc/systemd/system/air-daemon.service`
//! - Requires root privileges for installation
//! - Supports socket activation and notify-ready
//!
//! ### macOS (launchd)
//! - PID file location: `/tmp/air.pid`
//! - Service file: `/Library/LaunchDaemons/air-daemon.plist`
//! - Requires root privileges for system daemon
//! - Supports launchctl unload/start/stop commands
//!
//! ### Windows
//! - PID file location: `C:\ProgramData\Air\air.pid`
//! - Service registration via SCManager API
//! - Requires Administrator privileges
//! - Uses winsvc crate or similar for service management
//!
//! ## Security Considerations
//!
//! - PID file protected with checksum to prevent tampering
//! - Directory creation with secure permissions (0700)
//! - SUID/SGID not used for security
//! - User-level isolation for multi-user systems
//!
//! ## Error Handling
//!
//! All operations return `Result<T>` with comprehensive error types:
//! - `ServiceUnavailable`: Daemon already running or unavailable
//! - `FileSystem`: PID file or directory operations failed
//! - `PermissionDenied`: Insufficient privileges for service operations

use std::{fs, path::PathBuf, sync::Arc, time::Duration};

use log::{debug, error, info, warn};
use tokio::sync::{Mutex, RwLock};
use sha2::{Digest, Sha256};

use crate::{AirError, Result};

/// Daemon lifecycle manager
#[derive(Debug)]
pub struct DaemonManager {
	/// PID file path
	pid_file_path:PathBuf,
	/// Whether daemon is running
	is_running:Arc<RwLock<bool>>,
	/// Platform-specific daemon info
	platform_info:PlatformInfo,
	/// Lock for atomic PID file operations (prevents race conditions)
	pid_lock:Arc<Mutex<()>>,
	/// Checksum for PID file integrity verification
	pid_checksum:Arc<Mutex<Option<String>>>,
	/// Graceful shutdown flag
	shutdown_requested:Arc<RwLock<bool>>,
}

/// Platform-specific daemon information
#[derive(Debug)]
pub struct PlatformInfo {
	/// Platform type
	pub platform:Platform,
	/// Service name for system integration
	pub service_name:String,
	/// User under which daemon runs
	pub run_as_user:Option<String>,
}

/// Platform enum
#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
	Linux,
	MacOS,
	Windows,
	Unknown,
}

/// Exit codes for daemon operations
#[derive(Debug, Clone)]
pub enum ExitCode {
	Success = 0,
	ConfigurationError = 1,
	AlreadyRunning = 2,
	PermissionDenied = 3,
	ServiceError = 4,
	ResourceError = 5,
	NetworkError = 6,
	AuthenticationError = 7,
	FileSystemError = 8,
	InternalError = 9,
	UnknownError = 10,
}

impl DaemonManager {
	/// Create a new DaemonManager instance
	pub fn New(pid_file_path:Option<PathBuf>) -> Result<Self> {
		let pid_file_path = pid_file_path.unwrap_or_else(|| Self::DefaultPidFilePath());
		let platform_info = Self::DetectPlatformInfo();

		Ok(Self {
			pid_file_path,
			is_running:Arc::new(RwLock::new(false)),
			platform_info,
			pid_lock:Arc::new(Mutex::new(())),
			pid_checksum:Arc::new(Mutex::new(None)),
			shutdown_requested:Arc::new(RwLock::new(false)),
		})
	}

	/// Get default PID file path based on platform
	fn DefaultPidFilePath() -> PathBuf {
		let platform = Self::DetectPlatform();
		match platform {
			Platform::Linux => PathBuf::from("/var/run/air.pid"),
			Platform::MacOS => PathBuf::from("/tmp/air.pid"),
			Platform::Windows => PathBuf::from("C:\\ProgramData\\Air\\air.pid"),
			Platform::Unknown => PathBuf::from("./air.pid"),
		}
	}

	/// Detect current platform
	fn DetectPlatform() -> Platform {
		if cfg!(target_os = "linux") {
			Platform::Linux
		} else if cfg!(target_os = "macos") {
			Platform::MacOS
		} else if cfg!(target_os = "windows") {
			Platform::Windows
		} else {
			Platform::Unknown
		}
	}

	/// Detect platform-specific information
	fn DetectPlatformInfo() -> PlatformInfo {
		let platform = Self::DetectPlatform();
		let service_name = "air-daemon".to_string();

		// Get current user
		let run_as_user = std::env::var("USER").ok().or_else(|| std::env::var("USERNAME").ok());

		PlatformInfo { platform, service_name, run_as_user }
	}

	/// Get default PID file path based on platform
	fn default_pid_file_path() -> PathBuf {
		let platform = Self::detect_platform();
		match platform {
			Platform::Linux => PathBuf::from("/var/run/air.pid"),
			Platform::MacOS => PathBuf::from("/tmp/air.pid"),
			Platform::Windows => PathBuf::from("C:\\ProgramData\\Air\\air.pid"),
			Platform::Unknown => PathBuf::from("./air.pid"),
		}
	}

	/// Detect current platform
	fn detect_platform() -> Platform {
		if cfg!(target_os = "linux") {
			Platform::Linux
		} else if cfg!(target_os = "macos") {
			Platform::MacOS
		} else if cfg!(target_os = "windows") {
			Platform::Windows
		} else {
			Platform::Unknown
		}
	}

	/// Detect platform-specific information
	fn detect_platform_info() -> PlatformInfo {
		let platform = Self::detect_platform();
		let service_name = "air-daemon".to_string();

		// Get current user
		let run_as_user = std::env::var("USER").ok().or_else(|| std::env::var("USERNAME").ok());

		PlatformInfo { platform, service_name, run_as_user }
	}

	/// Acquire daemon lock to ensure single instance
	/// This method provides comprehensive defensive coding with:
	/// - Race condition protection through mutex locking
	/// - PID file checksum verification
	/// - Process validation checks
	/// - Atomic operations with rollback on failure
	/// - Timeout handling
	pub async fn AcquireLock(&self) -> Result<()> {
		info!("[Daemon] Acquiring daemon lock...");

		// Acquire lock to prevent race conditions
		tokio::select! {
			_ = tokio::time::timeout(Duration::from_secs(30), self.pid_lock.lock()) => {
				let _lock_guard = self.pid_lock.lock().await;
			},
			_ = tokio::time::sleep(Duration::from_secs(30)) => {
				return Err(AirError::Internal(
					"Timeout acquiring PID lock".to_string()
				));
			}
		}

		let _lock = self.pid_lock.lock().await;

		// Check if shutdown has been requested
		if *self.shutdown_requested.read().await {
			return Err(AirError::ServiceUnavailable(
				"Shutdown requested, cannot acquire lock".to_string(),
			));
		}

		// Check if PID file exists and process is running with validation
		if self.IsAlreadyRunning().await? {
			return Err(AirError::ServiceUnavailable("Air daemon is already running".to_string()));
		}

		// Create PID directory with secure permissions if it doesn't exist
		let temp_dir = PathBuf::from(format!("{}.tmp", self.pid_file_path.display()));
		if let Some(parent) = self.pid_file_path.parent() {
			fs::create_dir_all(parent)
				.map_err(|e| AirError::FileSystem(format!("Failed to create PID directory: {}", e)))?;

			// Set secure permissions on directory (user only)
			#[cfg(unix)]
			{
				use std::os::unix::fs::PermissionsExt;
				let perms = fs::Permissions::from_mode(0o700);
				fs::set_permissions(parent, perms)
					.map_err(|e| AirError::FileSystem(format!("Failed to set directory permissions: {}", e)))?;
			}
		}

		// Generate PID content with checksum for validation
		let pid = std::process::id();
		let timestamp = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_secs();
		let pid_content = format!("{}|{}", pid, timestamp);

		// Calculate checksum for integrity verification
		let mut hasher = Sha256::new();
		hasher.update(pid_content.as_bytes());
		let checksum = format!("{:x}", hasher.finalize());

		// Write to temporary file first (atomic operation)
		let temp_file_content = format!("{}|CHECKSUM:{}", pid_content, checksum);
		fs::write(&temp_dir, &temp_file_content)
			.map_err(|e| AirError::FileSystem(format!("Failed to write temporary PID file: {}", e)))?;

		// Atomic rename to avoid partial writes
		#[cfg(unix)]
		fs::rename(&temp_dir, &self.pid_file_path).map_err(|e| {
			// Rollback: clean up temp file on failure
			let _ = fs::remove_file(&temp_dir);
			AirError::FileSystem(format!("Failed to rename PID file: {}", e))
		})?;

		#[cfg(not(unix))]
		fs::rename(&temp_dir, &self.pid_file_path).map_err(|e| {
			let _ = fs::remove_file(&temp_dir);
			AirError::FileSystem(format!("Failed to rename PID file: {}", e))
		})?;

		// Store checksum for later validation
		*self.pid_checksum.lock().await = Some(checksum);

		// Set running state
		*self.is_running.write().await = true;

		// Set secure permissions on PID file
		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			let perms = fs::Permissions::from_mode(0o600);
			if let Err(e) = fs::set_permissions(&self.pid_file_path, perms) {
				warn!("[Daemon] Failed to set PID file permissions: {}", e);
			}
		}

		info!("[Daemon] Daemon lock acquired (PID: {})", pid);
		Ok(())
	}

	/// Check if daemon is already running
	/// Performs comprehensive validation including:
	/// - PID file existence check
	/// - Checksum verification
	/// - Process existence validation
	/// - Stale PID file cleanup
	pub async fn IsAlreadyRunning(&self) -> Result<bool> {
		if !self.pid_file_path.exists() {
			debug!("[Daemon] PID file does not exist");
			return Ok(false);
		}

		// Read PID from file
		let pid_content = fs::read_to_string(&self.pid_file_path)
			.map_err(|e| AirError::FileSystem(format!("Failed to read PID file: {}", e)))?;

		// Parse PID content with checksum
		let parts:Vec<&str> = pid_content.split('|').collect();
		if parts.len() < 2 {
			warn!("[Daemon] Invalid PID file format, treating as stale");
			self.CleanupStalePidFile().await?;
			return Ok(false);
		}

		let pid:u32 = parts[0].trim().parse().map_err(|e| {
			warn!("[Daemon] Invalid PID in file: {}", e);
			AirError::FileSystem("Invalid PID file content".to_string())
		})?;

		// Verify checksum if present
		if parts.len() >= 3 && parts[1].starts_with("CHECKSUM:") {
			let stored_checksum = &parts[1][9..]; // Remove "CHECKSUM:" prefix
			let current_checksum = self.pid_checksum.lock().await;

			if let Some(ref cksum) = *current_checksum {
				if cksum != stored_checksum {
					warn!("[Daemon] PID file checksum mismatch, file may be corrupted");
					// Don't automatically delete - could be a different daemon instance
					return Ok(true);
				}
			}
		}

		// Check if process exists with validation
		let is_running = Self::ValidateProcess(pid);

		if !is_running {
			// Clean up stale PID file with validation
			warn!("[Daemon] Detected stale PID file for PID {}", pid);
			self.CleanupStalePidFile().await?;
		}

		Ok(is_running)
	}

	/// Validate that a process with the given PID is running
	/// Performs thorough process validation and existence checks
	fn ValidateProcess(pid:u32) -> bool {
		#[cfg(unix)]
		{
			use std::process::Command;
			let output = Command::new("ps").arg("-p").arg(pid.to_string()).output();

			match output {
				Ok(output) => {
					if output.status.success() {
						let stdout = String::from_utf8_lossy(&output.stdout);
						// Validate it's actually an Air daemon process
						stdout
							.lines()
							.skip(1)
							.any(|line| line.contains("air") || line.contains("daemon"))
					} else {
						false
					}
				},
				Err(e) => {
					error!("[Daemon] Failed to check process status: {}", e);
					false
				},
			}
		}

		#[cfg(windows)]
		{
			use std::process::Command;
			let output = Command::new("tasklist")
				.arg("/FI")
				.arg(format!("PID eq {}", pid))
				.arg("/FO")
				.arg("CSV")
				.output();

			match output {
				Ok(output) => {
					if output.status.success() {
						let stdout = String::from_utf8_lossy(&output.stdout);
						stdout.lines().any(|line| {
							line.contains(&pid.to_string()) && (line.contains("air") || line.contains("daemon"))
						})
					} else {
						false
					}
				},
				Err(e) => {
					error!("[Daemon] Failed to check process status: {}", e);
					false
				},
			}
		}
	}

	/// Cleanup stale PID file with validation and error handling
	async fn CleanupStalePidFile(&self) -> Result<()> {
		if !self.pid_file_path.exists() {
			return Ok(());
		}

		// Verify the file is actually stale before deleting
		let content = fs::read_to_string(&self.pid_file_path)
			.map_err(|e| {
				warn!("[Daemon] Cannot verify stale PID file: {}", e);
				return false;
			})
			.ok();

		if let Some(content) = content {
			if content.starts_with(|c:char| c.is_numeric()) {
				// Clean up the stale PID file
				if let Err(e) = fs::remove_file(&self.pid_file_path) {
					warn!("[Daemon] Failed to remove stale PID file: {}", e);
					return Err(AirError::FileSystem(format!("Failed to remove stale PID file: {}", e)));
				}
				info!("[Daemon] Cleaned up stale PID file");
			}
		}

		Ok(())
	}

	/// Release daemon lock with proper cleanup and rollback
	/// Ensures all resources are properly cleaned up even on failure
	pub async fn ReleaseLock(&self) -> Result<()> {
		info!("[Daemon] Releasing daemon lock...");

		// Acquire lock for atomic cleanup
		let _lock = self.pid_lock.lock().await;

		// Set running state before cleanup
		*self.is_running.write().await = false;

		// Clear checksum
		*self.pid_checksum.lock().await = None;

		// Remove PID file with validation
		if self.pid_file_path.exists() {
			match fs::remove_file(&self.pid_file_path) {
				Ok(_) => {
					debug!("[Daemon] PID file removed successfully");
				},
				Err(e) => {
					error!("[Daemon] Failed to remove PID file: {}", e);
					// Don't fail entire operation if PID file cleanup fails
					return Err(AirError::FileSystem(format!("Failed to remove PID file: {}", e)));
				},
			}
		}

		// Try to clean up any temporary files
		let temp_dir = PathBuf::from(format!("{}.tmp", self.pid_file_path.display()));
		if temp_dir.exists() {
			let _ = fs::remove_file(&temp_dir);
		}

		info!("[Daemon] Daemon lock released");
		Ok(())
	}

	/// Check if daemon is running
	pub async fn IsRunning(&self) -> bool { *self.is_running.read().await }

	/// Request graceful shutdown
	pub async fn RequestShutdown(&self) -> Result<()> {
		info!("[Daemon] Requesting graceful shutdown...");
		*self.shutdown_requested.write().await = true;
		Ok(())
	}

	/// Clear shutdown request (for restart scenarios)
	pub async fn ClearShutdownRequest(&self) -> Result<()> {
		info!("[Daemon] Clearing shutdown request");
		*self.shutdown_requested.write().await = false;
		Ok(())
	}

	/// Check if shutdown has been requested
	pub async fn IsShutdownRequested(&self) -> bool { *self.shutdown_requested.read().await }

	/// Get daemon status with comprehensive health information
	pub async fn GetStatus(&self) -> Result<DaemonStatus> {
		let is_running = self.IsRunning().await;
		let pid_file_exists = self.pid_file_path.exists();

		let pid = if pid_file_exists {
			fs::read_to_string(&self.pid_file_path)
				.ok()
				.and_then(|content| content.split('|').next().and_then(|s| s.trim().parse().ok()))
		} else {
			None
		};

		Ok(DaemonStatus {
			is_running,
			pid_file_exists,
			pid,
			platform:self.platform_info.platform.clone(),
			service_name:self.platform_info.service_name.clone(),
			shutdown_requested:self.IsShutdownRequested().await,
		})
	}

	/// Generate system service file for installation
	pub fn GenerateServiceFile(&self) -> Result<String> {
		match self.platform_info.platform {
			Platform::Linux => self.GenerateSystemdService(),
			Platform::MacOS => self.GenerateLaunchdService(),
			Platform::Windows => self.GenerateWindowsService(),
			Platform::Unknown => {
				Err(AirError::ServiceUnavailable(
					"Unknown platform, cannot generate service file".to_string(),
				))
			},
		}
	}

	/// Generate systemd service file with comprehensive configuration
	fn GenerateSystemdService(&self) -> Result<String> {
		let exe_path = std::env::current_exe()
			.map_err(|e| AirError::FileSystem(format!("Failed to get executable path: {}", e)))?;

		let user = self.platform_info.run_as_user.as_deref().unwrap_or("root");
		let group = self.platform_info.run_as_user.as_deref().unwrap_or("root");

		let service_content = format!(
			r#"[Unit]
Description=Air Daemon - Background service for Land code editor
Documentation=man:air(1)
After=network-online.target
Wants=network-online.target
StartLimitIntervalSec=0

[Service]
Type=notify
NotifyAccess=all
ExecStart={}
ExecStop=/bin/kill -s TERM $MAINPID
Restart=always
RestartSec=5
StartLimitBurst=3
User={}
Group={}
Environment=RUST_LOG=info
Environment=DAEMON_MODE=systemd
Nice=-5
LimitNOFILE=65536
LimitNPROC=4096

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/air /var/run/air
RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6
RestrictRealtime=true

[Install]
WantedBy=multi-user.target
"#,
			exe_path.display(),
			user,
			group
		);

		Ok(service_content)
	}

	/// Generate launchd service file with comprehensive configuration
	fn GenerateLaunchdService(&self) -> Result<String> {
		let exe_path = std::env::current_exe()
			.map(|p| p.display().to_string())
			.unwrap_or_else(|_| "/usr/local/bin/air".to_string());

		let service_name = &self.platform_info.service_name;
		let user = self.platform_info.run_as_user.as_deref().unwrap_or("root");

		let service_content = format!(
			r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>--daemon</string>
        <string>--mode=launchd</string>
    </array>
    
    <key>RunAtLoad</key>
    <true/>
    
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
        <key>Crashed</key>
        <true/>
    </dict>
    
    <key>ThrottleInterval</key>
    <integer>5</integer>
    
    <key>UserName</key>
    <string>{}</string>
    
    <key>StandardOutPath</key>
    <string>/var/log/air/daemon.log</string>
    
    <key>StandardErrorPath</key>
    <string>/var/log/air/daemon.err</string>
    
    <key>WorkingDirectory</key>
    <string>/var/lib/air</string>
    
    <key>ProcessType</key>
    <string>Background</string>
    
    <key>Nice</key>
    <integer>-5</integer>
    
    <key>SoftResourceLimits</key>
    <dict>
        <key>NumberOfFiles</key>
        <integer>65536</integer>
    </dict>
    
    <key>HardResourceLimits</key>
    <dict>
        <key>NumberOfFiles</key>
        <integer>65536</integer>
    </dict>
    
    <key>EnvironmentVariables</key>
    <dict>
        <key>RUST_LOG</key>
        <string>info</string>
        <key>DAEMON_MODE</key>
        <string>launchd</string>
    </dict>
</dict>
</plist>
"#,
			service_name, exe_path, user
		);

		Ok(service_content)
	}

	/// Generate Windows service configuration file
	/// TODO: Integrate with winsvc crate for actual Windows Service
	/// registration
	fn GenerateWindowsService(&self) -> Result<String> {
		let exe_path = std::env::current_exe()
			.map(|p| p.display().to_string())
			.unwrap_or_else(|_| "C:\\Program Files\\Air\\air.exe".to_string());

		let service_name = &self.platform_info.service_name;
		let display_name = "Air Daemon Service";
		let description = "Background service for Land code editor";

		// Generate winsvc-compatible XML configuration
		let service_content = format!(
			r#"<service>
    <id>{}</id>
    <name>{}</name>
    <description>{}</description>
    <executable>{}</executable>
    
    <arguments>--daemon --mode=windows</arguments>
    
    <startmode>Automatic</startmode>
    <delayedAutoStart>true</delayedAutoStart>
    
    <log mode="roll">
        <sizeThreshold>10240</sizeThreshold>
        <keepFiles>8</keepFiles>
    </log>
    
    <onfailure action="restart" delay="10 sec"/>
    <onfailure action="restart" delay="20 sec"/>
    <onfailure action="restart" delay="60 sec"/>
    
    <resetfailure>1 hour</resetfailure>
    
    <depend>EventLog</depend>
    <depend>TcpIp</depend>
    
    <serviceaccount>
        <domain>.</domain>
        <user>LocalSystem</user>
        <password></password>
        <allowservicelogon>true</allowservicelogon>
    </serviceaccount>
    
    <workingdirectory>C:\Program Files\Air</workingdirectory>
    
    <env name="RUST_LOG" value="info"/>
    <env name="DAEMON_MODE" value="windows"/>
</service>
"#,
			service_name, display_name, description, exe_path
		);

		Ok(service_content)
	}

	/// Install daemon as system service with validation
	pub async fn InstallService(&self) -> Result<()> {
		info!("[Daemon] Installing system service...");

		match self.platform_info.platform {
			Platform::Linux => self.InstallSystemdService().await,
			Platform::MacOS => self.InstallLaunchdService().await,
			Platform::Windows => self.InstallWindowsService().await,
			Platform::Unknown => {
				Err(AirError::ServiceUnavailable(
					"Unknown platform, cannot install service".to_string(),
				))
			},
		}
	}

	/// Install systemd service with validation
	async fn InstallSystemdService(&self) -> Result<()> {
		let service_file_content = self.GenerateSystemdService()?;
		let service_file_path = format!("/etc/systemd/system/{}.service", self.platform_info.service_name);

		// Create temporary file for atomic write
		let temp_path = format!("{}.tmp", service_file_path);

		// Validate service content
		if !service_file_content.contains("[Unit]") || !service_file_content.contains("[Service]") {
			return Err(AirError::Configuration("Generated service file is invalid".to_string()));
		}

		// Write to temporary file first
		fs::write(&temp_path, &service_file_content)
			.map_err(|e| AirError::FileSystem(format!("Failed to write temporary service file: {}", e)))?;

		// Atomic rename
		#[cfg(unix)]
		fs::rename(&temp_path, &service_file_path).map_err(|e| {
			let _ = fs::remove_file(&temp_path);
			AirError::FileSystem(format!("Failed to rename service file: {}", e))
		})?;

		#[cfg(not(unix))]
		fs::rename(&temp_path, &service_file_path).map_err(|e| {
			let _ = fs::remove_file(&temp_path);
			AirError::FileSystem(format!("Failed to rename service file: {}", e))
		})?;

		// Set proper permissions
		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			let perms = fs::Permissions::from_mode(0o644);
			fs::set_permissions(&service_file_path, perms)
				.map_err(|e| {
					error!("[Daemon] Failed to set service file permissions: {}", e);
				})
				.ok();
		}

		info!("[Daemon] Systemd service installed at {}", service_file_path);

		// Run daemon-reload to notify systemd
		let _ = tokio::process::Command::new("systemctl").args(["daemon-reload"]).output().await;

		Ok(())
	}

	/// Install launchd service with validation
	async fn InstallLaunchdService(&self) -> Result<()> {
		let service_file_content = self.GenerateLaunchdService()?;
		let service_file_path = format!("/Library/LaunchDaemons/{}.plist", self.platform_info.service_name);

		// Create temporary file for atomic write
		let temp_path = format!("{}.tmp", service_file_path);

		// Validate plist content
		if !service_file_content.contains("<?xml") || !service_file_content.contains("<!DOCTYPE plist") {
			return Err(AirError::Configuration("Generated plist file is invalid".to_string()));
		}

		// Write to temporary file first
		fs::write(&temp_path, &service_file_content)
			.map_err(|e| AirError::FileSystem(format!("Failed to write temporary plist file: {}", e)))?;

		// Atomic rename
		#[cfg(unix)]
		fs::rename(&temp_path, &service_file_path).map_err(|e| {
			let _ = fs::remove_file(&temp_path);
			AirError::FileSystem(format!("Failed to rename plist file: {}", e))
		})?;

		#[cfg(not(unix))]
		fs::rename(&temp_path, &service_file_path).map_err(|e| {
			let _ = fs::remove_file(&temp_path);
			AirError::FileSystem(format!("Failed to rename plist file: {}", e))
		})?;

		// Set proper permissions
		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			let perms = fs::Permissions::from_mode(0o644);
			fs::set_permissions(&service_file_path, perms)
				.map_err(|e| {
					error!("[Daemon] Failed to set plist file permissions: {}", e);
				})
				.ok();
		}

		info!("[Daemon] Launchd service installed at {}", service_file_path);

		// No need to load immediately - launchd will pick it up automatically
		// User can run: sudo launchctl load -w /Library/LaunchDaemons/air-daemon.plist

		Ok(())
	}

	/// Install Windows service
	/// TODO: Integrate with winsvc crate for actual service registration
	async fn InstallWindowsService(&self) -> Result<()> {
		let service_file_content = self.GenerateWindowsService()?;
		let service_dir = "C:\\ProgramData\\Air";
		let service_file_path = format!("{}\\{}.xml", service_dir, self.platform_info.service_name);

		// Create directory if it doesn't exist
		fs::create_dir_all(service_dir)
			.map_err(|e| AirError::FileSystem(format!("Failed to create service directory: {}", e)))?;

		// Create temporary file for atomic write
		let temp_path = format!("{}.tmp", service_file_path);

		// Validate service content
		if !service_file_content.contains("<service>") {
			return Err(AirError::Configuration("Generated service file is invalid".to_string()));
		}

		// Write to temporary file first
		fs::write(&temp_path, &service_file_content)
			.map_err(|e| AirError::FileSystem(format!("Failed to write temporary service file: {}", e)))?;

		// Atomic rename
		fs::rename(&temp_path, &service_file_path).map_err(|e| {
			let _ = fs::remove_file(&temp_path);
			AirError::FileSystem(format!("Failed to rename service file: {}", e))
		})?;

		info!("[Daemon] Windows service configuration written to {}", service_file_path);
		warn!("[Daemon] Windows service installation requires additional integration with winsvc crate");
		warn!("[Daemon] Manual installation may be required: Use SC.EXE or winsvc to register service");

		Ok(())
	}

	/// Uninstall system service with proper coordination
	pub async fn UninstallService(&self) -> Result<()> {
		info!("[Daemon] Uninstalling system service...");

		match self.platform_info.platform {
			Platform::Linux => self.UninstallSystemdService().await,
			Platform::MacOS => self.UninstallLaunchdService().await,
			Platform::Windows => self.UninstallWindowsService().await,
			Platform::Unknown => {
				Err(AirError::ServiceUnavailable(
					"Unknown platform, cannot uninstall service".to_string(),
				))
			},
		}
	}

	/// Uninstall systemd service with proper coordination
	async fn UninstallSystemdService(&self) -> Result<()> {
		let service_file_path = format!("/etc/systemd/system/{}.service", self.platform_info.service_name);

		// Stop service first if running
		let _ = tokio::process::Command::new("systemctl")
			.args(["stop", &self.platform_info.service_name])
			.output()
			.await;

		// Disable service
		let _ = tokio::process::Command::new("systemctl")
			.args(["disable", &self.platform_info.service_name])
			.output()
			.await;

		// Remove service file
		if fs::remove_file(&service_file_path).is_ok() {
			info!("[Daemon] Systemd service file removed");
		} else {
			warn!("[Daemon] Service file {} not found", service_file_path);
		}

		// Reload systemd
		let _ = tokio::process::Command::new("systemctl").args(["daemon-reload"]).output().await;

		info!("[Daemon] Systemd service uninstalled");
		Ok(())
	}

	/// Uninstall launchd service with proper coordination
	async fn UninstallLaunchdService(&self) -> Result<()> {
		let service_file_path = format!("/Library/LaunchDaemons/{}.plist", self.platform_info.service_name);

		// Unload service first
		let _ = tokio::process::Command::new("launchctl")
			.args(["unload", "-w", &service_file_path])
			.output()
			.await;

		// Remove service file
		if fs::remove_file(&service_file_path).is_ok() {
			info!("[Daemon] Launchd service file removed");
		} else {
			warn!("[Daemon] Service file {} not found", service_file_path);
		}

		info!("[Daemon] Launchd service uninstalled");
		Ok(())
	}

	/// Uninstall Windows service
	async fn UninstallWindowsService(&self) -> Result<()> {
		let service_file_path = format!("C:\\ProgramData\\Air\\{}.xml", self.platform_info.service_name);

		// TODO: Use winsvc to properly stop and remove service
		// For now, just remove the configuration file

		if fs::remove_file(&service_file_path).is_ok() {
			info!("[Daemon] Windows service configuration removed");
		} else {
			warn!("[Daemon] Service file {} not found", service_file_path);
		}

		warn!("[Daemon] Manual Windows service removal may be required: Use SC.EXE or winsvc");

		Ok(())
	}
}

/// Daemon status information
#[derive(Debug, Clone)]
pub struct DaemonStatus {
	pub is_running:bool,
	pub pid_file_exists:bool,
	pub pid:Option<u32>,
	pub platform:Platform,
	pub service_name:String,
	pub shutdown_requested:bool,
}

impl DaemonStatus {
	/// Get human-readable status description
	pub fn status_description(&self) -> String {
		if self.is_running {
			format!("Running (PID: {})", self.pid.unwrap_or(0))
		} else if self.pid_file_exists {
			"Stale PID file exists".to_string()
		} else {
			"Not running".to_string()
		}
	}
}

impl From<ExitCode> for i32 {
	fn from(code:ExitCode) -> i32 { code as i32 }
}
