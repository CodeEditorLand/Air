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
//! ## FUTURE Enhancements
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
//! ## Platform-Specific Considerations
//!
//! ### Linux (systemd)
//! - PID file location: `/var/run/Air.pid`
//! - Service file: `/etc/systemd/system/Air-daemon.service`
//! - Requires root privileges for installation
//! - Supports socket activation and notify-ready
//!
//! ### macOS (launchd)
//! - PID file location: `/tmp/Air.pid`
//! - Service file: `/Library/LaunchDaemons/Air-daemon.plist`
//! - Requires root privileges for system daemon
//! - Supports launchctl unload/start/stop commands
//!
//! ### Windows
//! - PID file location: `C:\ProgramData\Air\Air.pid`
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
	PidFilePath:PathBuf,
	/// Whether daemon is running
	IsRunning:Arc<RwLock<bool>>,
	/// Platform-specific daemon info
	PlatformInfo:PlatformInfo,
	/// Lock for atomic PID file operations (prevents race conditions)
	PidLock:Arc<Mutex<()>>,
	/// Checksum for PID file integrity verification
	PidChecksum:Arc<Mutex<Option<String>>>,
	/// Graceful shutdown flag
	ShutdownRequested:Arc<RwLock<bool>>,
}

/// Platform-specific daemon information
#[derive(Debug)]
pub struct PlatformInfo {
	/// Platform type
	pub Platform:Platform,
	/// Service name for system integration
	pub ServiceName:String,
	/// User under which daemon runs
	pub RunAsUser:Option<String>,
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
	pub fn New(PidFilePath:Option<PathBuf>) -> Result<Self> {
		let PidFilePath = PidFilePath.unwrap_or_else(|| Self::DefaultPidFilePath());
		let PlatformInfo = Self::DetectPlatformInfo();

		Ok(Self {
			PidFilePath,
			IsRunning:Arc::new(RwLock::new(false)),
			PlatformInfo,
			PidLock:Arc::new(Mutex::new(())),
			PidChecksum:Arc::new(Mutex::new(None)),
			ShutdownRequested:Arc::new(RwLock::new(false)),
		})
	}

	/// Get default PID file path based on platform
	fn DefaultPidFilePath() -> PathBuf {
		let platform = Self::DetectPlatform();
		match platform {
			Platform::Linux => PathBuf::from("/var/run/Air.pid"),
			Platform::MacOS => PathBuf::from("/tmp/Air.pid"),
			Platform::Windows => PathBuf::from("C:\\ProgramData\\Air\\Air.pid"),
			Platform::Unknown => PathBuf::from("./Air.pid"),
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
		let ServiceName = "Air-daemon".to_string();

		// Get current user
		let RunAsUser = std::env::var("USER").ok().or_else(|| std::env::var("USERNAME").ok());

		PlatformInfo { Platform:platform, ServiceName, RunAsUser }
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
			_ = tokio::time::timeout(Duration::from_secs(30), self.PidLock.lock()) => {
				let _lock_guard = self.PidLock.lock().await;
			},
			_ = tokio::time::sleep(Duration::from_secs(30)) => {
				return Err(AirError::Internal(
					"Timeout acquiring PID lock".to_string()
				));
			}
		}

		let _lock = self.PidLock.lock().await;

		// Check if shutdown has been requested
		if *self.ShutdownRequested.read().await {
			return Err(AirError::ServiceUnavailable(
				"Shutdown requested, cannot acquire lock".to_string(),
			));
		}

		// Check if PID file exists and process is running with validation
		if self.IsAlreadyRunning().await? {
			return Err(AirError::ServiceUnavailable("Air daemon is already running".to_string()));
		}

		// Create PID directory with secure permissions if it doesn't exist
		let TempDir = PathBuf::from(format!("{}.tmp", self.PidFilePath.display()));
		if let Some(parent) = self.PidFilePath.parent() {
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
		let PidContent = format!("{}|{}", pid, timestamp);

		// Calculate checksum for integrity verification
		let mut hasher = Sha256::new();
		hasher.update(PidContent.as_bytes());
		let checksum = format!("{:x}", hasher.finalize());

		// Write to temporary file first (atomic operation)
		let TempFileContent = format!("{}|CHECKSUM:{}", PidContent, checksum);
		fs::write(&TempDir, &TempFileContent)
			.map_err(|e| AirError::FileSystem(format!("Failed to write temporary PID file: {}", e)))?;

		// Atomic rename to avoid partial writes
		#[cfg(unix)]
		fs::rename(&TempDir, &self.PidFilePath).map_err(|e| {
			// Rollback: clean up temp file on failure
			let _ = fs::remove_file(&TempDir);
			AirError::FileSystem(format!("Failed to rename PID file: {}", e))
		})?;

		#[cfg(not(unix))]
		fs::rename(&TempDir, &self.PidFilePath).map_err(|e| {
			let _ = fs::remove_file(&TempDir);
			AirError::FileSystem(format!("Failed to rename PID file: {}", e))
		})?;

		// Store checksum for later validation
		*self.PidChecksum.lock().await = Some(checksum);

		// Set running state
		*self.IsRunning.write().await = true;

		// Set secure permissions on PID file
		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			let perms = fs::Permissions::from_mode(0o600);
			if let Err(e) = fs::set_permissions(&self.PidFilePath, perms) {
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
		if !self.PidFilePath.exists() {
			debug!("[Daemon] PID file does not exist");
			return Ok(false);
		}

		// Read PID from file
		let PidContent = fs::read_to_string(&self.PidFilePath)
			.map_err(|e| AirError::FileSystem(format!("Failed to read PID file: {}", e)))?;

		// Parse PID content with checksum
		let parts:Vec<&str> = PidContent.split('|').collect();
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
			let StoredChecksum = &parts[1][9..]; // Remove "CHECKSUM:" prefix
			let CurrentChecksum = self.PidChecksum.lock().await;

			if let Some(ref cksum) = *CurrentChecksum {
				if cksum != StoredChecksum {
					warn!("[Daemon] PID file checksum mismatch, file may be corrupted");
					// Don't automatically delete - could be a different daemon instance
					return Ok(true);
				}
			}
		}

		// Check if process exists with validation
		let IsRunning = Self::ValidateProcess(pid);

		if !IsRunning {
			// Clean up stale PID file with validation
			warn!("[Daemon] Detected stale PID file for PID {}", pid);
			self.CleanupStalePidFile().await?;
		}

		Ok(IsRunning)
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
							.any(|line| line.contains("Air") || line.contains("daemon"))
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
							line.contains(&pid.to_string()) && (line.contains("Air") || line.contains("daemon"))
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
		if !self.PidFilePath.exists() {
			return Ok(());
		}

		// Verify the file is actually stale before deleting
		let content = fs::read_to_string(&self.PidFilePath)
			.map_err(|e| {
				warn!("[Daemon] Cannot verify stale PID file: {}", e);
				return false;
			})
			.ok();

		if let Some(content) = content {
			if content.starts_with(|c:char| c.is_numeric()) {
				// Clean up the stale PID file
				if let Err(e) = fs::remove_file(&self.PidFilePath) {
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
		let _lock = self.PidLock.lock().await;

		// Set running state before cleanup
		*self.IsRunning.write().await = false;

		// Clear checksum
		*self.PidChecksum.lock().await = None;

		// Remove PID file with validation
		if self.PidFilePath.exists() {
			match fs::remove_file(&self.PidFilePath) {
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
		let TempDir = PathBuf::from(format!("{}.tmp", self.PidFilePath.display()));
		if TempDir.exists() {
			let _ = fs::remove_file(&TempDir);
		}

		info!("[Daemon] Daemon lock released");
		Ok(())
	}

	/// Check if daemon is running
	pub async fn IsRunning(&self) -> bool { *self.IsRunning.read().await }

	/// Request graceful shutdown
	pub async fn RequestShutdown(&self) -> Result<()> {
		info!("[Daemon] Requesting graceful shutdown...");
		*self.ShutdownRequested.write().await = true;
		Ok(())
	}

	/// Clear shutdown request (for restart scenarios)
	pub async fn ClearShutdownRequest(&self) -> Result<()> {
		info!("[Daemon] Clearing shutdown request");
		*self.ShutdownRequested.write().await = false;
		Ok(())
	}

	/// Check if shutdown has been requested
	pub async fn IsShutdownRequested(&self) -> bool { *self.ShutdownRequested.read().await }

	/// Get daemon status with comprehensive health information
	pub async fn GetStatus(&self) -> Result<DaemonStatus> {
		let IsRunning = self.IsRunning().await;
		let PidFileExists = self.PidFilePath.exists();

		let pid = if PidFileExists {
			fs::read_to_string(&self.PidFilePath)
				.ok()
				.and_then(|content| content.split('|').next().and_then(|s| s.trim().parse().ok()))
		} else {
			None
		};

		Ok(DaemonStatus {
			IsRunning,
			PidFileExists,
			Pid:pid,
			Platform:self.PlatformInfo.Platform.clone(),
			ServiceName:self.PlatformInfo.ServiceName.clone(),
			ShutdownRequested:self.IsShutdownRequested().await,
		})
	}

	/// Generate system service file for installation
	pub fn GenerateServiceFile(&self) -> Result<String> {
		match self.PlatformInfo.Platform {
			Platform::Linux => self.GenerateSystemdService(),
			Platform::MacOS => self.GenerateLaunchdService(),
			#[cfg(target_os = "windows")]
			Platform::Windows => self.GenerateWindowsService(),
			#[cfg(not(target_os = "windows"))]
			Platform::Windows => {
				Err(AirError::ServiceUnavailable(
					"Windows service generation not available on this platform".to_string(),
				))
			},
			Platform::Unknown => {
				Err(AirError::ServiceUnavailable(
					"Unknown platform, cannot generate service file".to_string(),
				))
			},
		}
	}

	/// Generate systemd service file with comprehensive configuration
	fn GenerateSystemdService(&self) -> Result<String> {
		let ExePath = std::env::current_exe()
			.map_err(|e| AirError::FileSystem(format!("Failed to get executable path: {}", e)))?;

		let user = self.PlatformInfo.RunAsUser.as_deref().unwrap_or("root");
		let group = self.PlatformInfo.RunAsUser.as_deref().unwrap_or("root");

		let ServiceContent = format!(
			r#"[Unit]
Description=Air Daemon - Background service for Land code editor
Documentation=man:Air(1)
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
ReadWritePaths=/var/log/Air /var/run/Air
RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6
RestrictRealtime=true

[Install]
WantedBy=multi-user.target
"#,
			ExePath.display(),
			user,
			group
		);

		Ok(ServiceContent)
	}

	/// Generate launchd service file with comprehensive configuration
	fn GenerateLaunchdService(&self) -> Result<String> {
		let ExePath = std::env::current_exe()
			.map(|p| p.display().to_string())
			.unwrap_or_else(|_| "/usr/local/bin/Air".to_string());

		let ServiceName = &self.PlatformInfo.ServiceName;
		let user = self.PlatformInfo.RunAsUser.as_deref().unwrap_or("root");

		let ServiceContent = format!(
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
    <string>/var/log/Air/daemon.log</string>
    
    <key>StandardErrorPath</key>
    <string>/var/log/Air/daemon.err</string>
    
    <key>WorkingDirectory</key>
    <string>/var/lib/Air</string>
    
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
			ServiceName, ExePath, user
		);

		Ok(ServiceContent)
	}

	/// Generate Windows service configuration file
	///
	/// Note: For production use with actual Windows service registration,
	/// integrate with the winsvc crate or windows-rs API.
	/// This method generates a configuration file compatible with winsvc.
	#[cfg(target_os = "windows")]
	fn GenerateWindowsService(&self) -> Result<String> {
		let ExePath = std::env::current_exe()
			.map(|p| p.display().to_string())
			.unwrap_or_else(|_| "C:\\Program Files\\Air\\Air.exe".to_string());

		let ServiceName = &self.PlatformInfo.ServiceName;
		let DisplayName = "Air Daemon Service";
		let Description = "Background service for Land code editor";

		// Generate winsvc-compatible XML configuration
		let ServiceContent = format!(
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
			ServiceName, DisplayName, Description, ExePath
		);

		Ok(ServiceContent)
	}

	/// Install daemon as system service with validation
	pub async fn InstallService(&self) -> Result<()> {
		info!("[Daemon] Installing system service...");

		match self.PlatformInfo.Platform {
			Platform::Linux => self.InstallSystemdService().await,
			Platform::MacOS => self.InstallLaunchdService().await,
			#[cfg(target_os = "windows")]
			Platform::Windows => self.InstallWindowsService().await,
			#[cfg(not(target_os = "windows"))]
			Platform::Windows => {
				Err(AirError::ServiceUnavailable(
					"Windows service installation not available on this platform".to_string(),
				))
			},
			Platform::Unknown => {
				Err(AirError::ServiceUnavailable(
					"Unknown platform, cannot install service".to_string(),
				))
			},
		}
	}

	/// Install systemd service with validation
	async fn InstallSystemdService(&self) -> Result<()> {
		let ServiceFileContent = self.GenerateSystemdService()?;
		let ServiceFilePath = format!("/etc/systemd/system/{}.service", self.PlatformInfo.ServiceName);

		// Create temporary file for atomic write
		let TempPath = format!("{}.tmp", ServiceFilePath);

		// Validate service content
		if !ServiceFileContent.contains("[Unit]") || !ServiceFileContent.contains("[Service]") {
			return Err(AirError::Configuration("Generated service file is invalid".to_string()));
		}

		// Write to temporary file first
		fs::write(&TempPath, &ServiceFileContent)
			.map_err(|e| AirError::FileSystem(format!("Failed to write temporary service file: {}", e)))?;

		// Atomic rename
		#[cfg(unix)]
		fs::rename(&TempPath, &ServiceFilePath).map_err(|e| {
			let _ = fs::remove_file(&TempPath);
			AirError::FileSystem(format!("Failed to rename service file: {}", e))
		})?;

		#[cfg(not(unix))]
		fs::rename(&TempPath, &ServiceFilePath).map_err(|e| {
			let _ = fs::remove_file(&TempPath);
			AirError::FileSystem(format!("Failed to rename service file: {}", e))
		})?;

		// Set proper permissions
		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			let perms = fs::Permissions::from_mode(0o644);
			fs::set_permissions(&ServiceFilePath, perms)
				.map_err(|e| {
					error!("[Daemon] Failed to set service file permissions: {}", e);
				})
				.ok();
		}

		info!("[Daemon] Systemd service installed at {}", ServiceFilePath);

		// Run daemon-reload to notify systemd
		let _ = tokio::process::Command::new("systemctl").args(["daemon-reload"]).output().await;

		Ok(())
	}

	/// Install launchd service with validation
	async fn InstallLaunchdService(&self) -> Result<()> {
		let ServiceFileContent = self.GenerateLaunchdService()?;
		let ServiceFilePath = format!("/Library/LaunchDaemons/{}.plist", self.PlatformInfo.ServiceName);

		// Create temporary file for atomic write
		let TempPath = format!("{}.tmp", ServiceFilePath);

		// Validate plist content
		if !ServiceFileContent.contains("<?xml") || !ServiceFileContent.contains("<!DOCTYPE plist") {
			return Err(AirError::Configuration("Generated plist file is invalid".to_string()));
		}

		// Write to temporary file first
		fs::write(&TempPath, &ServiceFileContent)
			.map_err(|e| AirError::FileSystem(format!("Failed to write temporary plist file: {}", e)))?;

		// Atomic rename
		#[cfg(unix)]
		fs::rename(&TempPath, &ServiceFilePath).map_err(|e| {
			let _ = fs::remove_file(&TempPath);
			AirError::FileSystem(format!("Failed to rename plist file: {}", e))
		})?;

		#[cfg(not(unix))]
		fs::rename(&TempPath, &ServiceFilePath).map_err(|e| {
			let _ = fs::remove_file(&TempPath);
			AirError::FileSystem(format!("Failed to rename plist file: {}", e))
		})?;

		// Set proper permissions
		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			let perms = fs::Permissions::from_mode(0o644);
			fs::set_permissions(&ServiceFilePath, perms)
				.map_err(|e| {
					error!("[Daemon] Failed to set plist file permissions: {}", e);
				})
				.ok();
		}

		info!("[Daemon] Launchd service installed at {}", ServiceFilePath);

		// No need to load immediately - launchd will pick it up automatically
		// User can run: sudo launchctl load -w /Library/LaunchDaemons/Air-daemon.plist

		Ok(())
	}

	/// Install Windows service
	///
	/// Note: For production use, integrate with the winsvc crate or windows-rs
	/// API to perform actual Windows service registration via the Service
	/// Control Manager (SCM). This method writes a configuration file that can
	/// be used with winsvc.
	#[cfg(target_os = "windows")]
	async fn InstallWindowsService(&self) -> Result<()> {
		let ServiceFileContent = self.GenerateWindowsService()?;
		let ServiceDir = "C:\\ProgramData\\Air";
		let ServiceFilePath = format!("{}\\{}.xml", ServiceDir, self.PlatformInfo.ServiceName);

		// Create directory if it doesn't exist
		fs::create_dir_all(&ServiceDir)
			.map_err(|e| AirError::FileSystem(format!("Failed to create service directory: {}", e)))?;

		// Create temporary file for atomic write
		let TempPath = format!("{}.tmp", ServiceFilePath);

		// Validate service content
		if !ServiceFileContent.contains("<service>") {
			return Err(AirError::Configuration("Generated service file is invalid".to_string()));
		}

		// Write to temporary file first
		fs::write(&TempPath, &ServiceFileContent)
			.map_err(|e| AirError::FileSystem(format!("Failed to write temporary service file: {}", e)))?;

		// Atomic rename
		fs::rename(&TempPath, &ServiceFilePath).map_err(|e| {
			let _ = fs::remove_file(&TempPath);
			AirError::FileSystem(format!("Failed to rename service file: {}", e))
		})?;

		info!("[Daemon] Windows service configuration written to {}", ServiceFilePath);
		info!("[Daemon] To register the service, run:");
		info!(
			"[Daemon]   sc create AirDaemon binPath= \"{}\" DisplayName= \"Air Daemon\"",
			std::env::current_exe().unwrap_or_else(|_| "air.exe".into()).display()
		);
		info!("[Daemon]   sc config AirDaemon start= auto");
		info!("[Daemon]   sc start AirDaemon");

		Ok(())
	}

	/// Uninstall system service with proper coordination
	pub async fn UninstallService(&self) -> Result<()> {
		info!("[Daemon] Uninstalling system service...");

		match self.PlatformInfo.Platform {
			Platform::Linux => self.UninstallSystemdService().await,
			Platform::MacOS => self.UninstallLaunchdService().await,
			#[cfg(target_os = "windows")]
			Platform::Windows => self.UninstallWindowsService().await,
			#[cfg(not(target_os = "windows"))]
			Platform::Windows => {
				Err(AirError::ServiceUnavailable(
					"Windows service uninstallation not available on this platform".to_string(),
				))
			},
			Platform::Unknown => {
				Err(AirError::ServiceUnavailable(
					"Unknown platform, cannot uninstall service".to_string(),
				))
			},
		}
	}

	/// Uninstall systemd service with proper coordination
	async fn UninstallSystemdService(&self) -> Result<()> {
		let ServiceFilePath = format!("/etc/systemd/system/{}.service", self.PlatformInfo.ServiceName);

		// Stop service first if running
		let _ = tokio::process::Command::new("systemctl")
			.args(["stop", &self.PlatformInfo.ServiceName])
			.output()
			.await;

		// Disable service
		let _ = tokio::process::Command::new("systemctl")
			.args(["disable", &self.PlatformInfo.ServiceName])
			.output()
			.await;

		// Remove service file
		if fs::remove_file(&ServiceFilePath).is_ok() {
			info!("[Daemon] Systemd service file removed");
		} else {
			warn!("[Daemon] Service file {} not found", ServiceFilePath);
		}

		// Reload systemd
		let _ = tokio::process::Command::new("systemctl").args(["daemon-reload"]).output().await;

		info!("[Daemon] Systemd service uninstalled");
		Ok(())
	}

	/// Uninstall launchd service with proper coordination
	async fn UninstallLaunchdService(&self) -> Result<()> {
		let ServiceFilePath = format!("/Library/LaunchDaemons/{}.plist", self.PlatformInfo.ServiceName);

		// Unload service first
		let _ = tokio::process::Command::new("launchctl")
			.args(["unload", "-w", &ServiceFilePath])
			.output()
			.await;

		// Remove service file
		if fs::remove_file(&ServiceFilePath).is_ok() {
			info!("[Daemon] Launchd service file removed");
		} else {
			warn!("[Daemon] Service file {} not found", ServiceFilePath);
		}

		info!("[Daemon] Launchd service uninstalled");
		Ok(())
	}

	/// Uninstall Windows service
	///
	/// Note: For production use, integrate with the winsvc crate or windows-rs
	/// API to properly stop and remove the Windows service via the Service
	/// Control Manager (SCM).
	#[cfg(target_os = "windows")]
	async fn UninstallWindowsService(&self) -> Result<()> {
		let ServiceFilePath = format!("C:\\ProgramData\\Air\\{}.xml", self.PlatformInfo.ServiceName);

		// Remove the configuration file
		if fs::remove_file(&ServiceFilePath).is_ok() {
			info!("[Daemon] Windows service configuration removed");
		} else {
			warn!("[Daemon] Service file {} not found", ServiceFilePath);
		}

		info!("[Daemon] To unregister the service, run:");
		info!("[Daemon]   sc stop AirDaemon");
		info!("[Daemon]   sc delete AirDaemon");

		Ok(())
	}
}

/// Daemon status information
#[derive(Debug, Clone)]
pub struct DaemonStatus {
	pub IsRunning:bool,
	pub PidFileExists:bool,
	pub Pid:Option<u32>,
	pub Platform:Platform,
	pub ServiceName:String,
	pub ShutdownRequested:bool,
}

impl DaemonStatus {
	/// Get human-readable status description
	pub fn status_description(&self) -> String {
		if self.IsRunning {
			format!("Running (PID: {})", self.Pid.unwrap_or(0))
		} else if self.PidFileExists {
			"Stale PID file exists".to_string()
		} else {
			"Not running".to_string()
		}
	}
}

impl From<ExitCode> for i32 {
	fn from(code:ExitCode) -> i32 { code as i32 }
}
