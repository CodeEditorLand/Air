//! # Daemon Lifecycle Management
//!
//! Provides robust daemon lifecycle management including PID file handling,
//! singleton enforcement, graceful shutdown, and platform-specific daemon setup.

use std::{fs, path::PathBuf, sync::Arc};
use log::{info, warn};
use tokio::sync::RwLock;

use crate::{Result, AirError};

/// Daemon lifecycle manager
#[derive(Debug)]
pub struct DaemonManager {
    /// PID file path
    pid_file_path: PathBuf,
    /// Whether daemon is running
    is_running: Arc<RwLock<bool>>,
    /// Platform-specific daemon info
    platform_info: PlatformInfo,
}

/// Platform-specific daemon information
#[derive(Debug)]
pub struct PlatformInfo {
    /// Platform type
    pub platform: Platform,
    /// Service name for system integration
    pub service_name: String,
    /// User under which daemon runs
    pub run_as_user: Option<String>,
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
    pub fn new(pid_file_path: Option<PathBuf>) -> Result<Self> {
        let pid_file_path = pid_file_path.unwrap_or_else(|| Self::default_pid_file_path());
        let platform_info = Self::detect_platform_info();
        
        Ok(Self {
            pid_file_path,
            is_running: Arc::new(RwLock::new(false)),
            platform_info,
        })
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
        let run_as_user = std::env::var("USER").ok()
            .or_else(|| std::env::var("USERNAME").ok());
        
        PlatformInfo {
            platform,
            service_name,
            run_as_user,
        }
    }
    
    /// Acquire daemon lock to ensure single instance
    pub async fn acquire_lock(&self) -> Result<()> {
        info!("[Daemon] Acquiring daemon lock...");
        
        // Check if PID file exists and process is running
        if self.is_already_running().await? {
            return Err(AirError::ServiceUnavailable(
                "Air daemon is already running".to_string()
            ));
        }
        
        // Create PID directory if it doesn't exist
        if let Some(parent) = self.pid_file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                AirError::FileSystem(format!("Failed to create PID directory: {}", e))
            })?;
        }
        
        // Write PID file
        let pid = std::process::id();
        fs::write(&self.pid_file_path, pid.to_string()).map_err(|e| {
            AirError::FileSystem(format!("Failed to write PID file: {}", e))
        })?;
        
        // Set running state
        *self.is_running.write().await = true;
        
        info!("[Daemon] Daemon lock acquired (PID: {})", pid);
        Ok(())
    }
    
    /// Check if daemon is already running
    pub async fn is_already_running(&self) -> Result<bool> {
        if !self.pid_file_path.exists() {
            return Ok(false);
        }
        
        // Read PID from file
        let pid_content = fs::read_to_string(&self.pid_file_path).map_err(|e| {
            AirError::FileSystem(format!("Failed to read PID file: {}", e))
        })?;
        
        let pid: u32 = pid_content.trim().parse().map_err(|_| {
            AirError::FileSystem("Invalid PID file content".to_string())
        })?;
        
        // Check if process exists
        let is_running = Self::is_process_running(pid);
        
        if !is_running {
            // Clean up stale PID file
            if let Err(e) = fs::remove_file(&self.pid_file_path) {
                warn!("[Daemon] Failed to remove stale PID file: {}", e);
            }
        }
        
        Ok(is_running)
    }
    
    /// Check if process with given PID is running
    fn is_process_running(pid: u32) -> bool {
        #[cfg(unix)]
        {
            use std::process::Command;
            let output = Command::new("ps")
                .arg("-p")
                .arg(pid.to_string())
                .output();
            
            match output {
                Ok(output) => output.status.success(),
                Err(_) => false,
            }
        }
        
        #[cfg(windows)]
        {
            use std::process::Command;
            let output = Command::new("tasklist")
                .arg("/FI")
                .arg(format!("PID eq {}", pid))
                .output();
            
            match output {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    stdout.contains(&pid.to_string())
                }
                Err(_) => false,
            }
        }
    }
    
    /// Release daemon lock
    pub async fn release_lock(&self) -> Result<()> {
        info!("[Daemon] Releasing daemon lock...");
        
        // Set running state
        *self.is_running.write().await = false;
        
        // Remove PID file
        if self.pid_file_path.exists() {
            fs::remove_file(&self.pid_file_path).map_err(|e| {
                AirError::FileSystem(format!("Failed to remove PID file: {}", e))
            })?;
        }
        
        info!("[Daemon] Daemon lock released");
        Ok(())
    }
    
    /// Check if daemon is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }
    
    /// Get daemon status
    pub async fn get_status(&self) -> Result<DaemonStatus> {
        let is_running = self.is_running().await;
        let pid_file_exists = self.pid_file_path.exists();
        
        let pid = if pid_file_exists {
            fs::read_to_string(&self.pid_file_path).ok()
                .and_then(|content| content.trim().parse().ok())
        } else {
            None
        };
        
        Ok(DaemonStatus {
            is_running,
            pid_file_exists,
            pid,
            platform: self.platform_info.platform.clone(),
            service_name: self.platform_info.service_name.clone(),
        })
    }
    
    /// Generate system service file
    pub fn generate_service_file(&self) -> Result<String> {
        match self.platform_info.platform {
            Platform::Linux => self.generate_systemd_service(),
            Platform::MacOS => self.generate_launchd_service(),
            Platform::Windows => self.generate_windows_service(),
            Platform::Unknown => Err(AirError::ServiceUnavailable(
                "Unknown platform, cannot generate service file".to_string()
            )),
        }
    }
    
    /// Generate systemd service file
    fn generate_systemd_service(&self) -> Result<String> {
        let service_content = format!(
            r#"[Unit]
Description=Air Daemon - Background service for Land code editor
After=network.target

[Service]
Type=simple
ExecStart={}
Restart=always
RestartSec=5
User={}
Group={}
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target"#,
            std::env::current_exe()
                .map_err(|e| AirError::FileSystem(format!("Failed to get executable path: {}", e)))?
                .display(),
            self.platform_info.run_as_user.as_deref().unwrap_or("root"),
            self.platform_info.run_as_user.as_deref().unwrap_or("root")
        );
        
        Ok(service_content)
    }
    
    /// Generate launchd service file
    fn generate_launchd_service(&self) -> Result<String> {
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
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/var/log/air.log</string>
    <key>StandardErrorPath</key>
    <string>/var/log/air.log</string>
</dict>
</plist>"#,
            self.platform_info.service_name,
            std::env::current_exe()
                .map_err(|e| AirError::FileSystem(format!("Failed to get executable path: {}", e)))?
                .display()
        );
        
        Ok(service_content)
    }
    
    /// Generate Windows service file
    fn generate_windows_service(&self) -> Result<String> {
        let service_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<service>
    <id>{}</id>
    <name>Air Daemon</name>
    <description>Background service for Land code editor</description>
    <executable>{}</executable>
    <log mode="roll"/>
    <onfailure action="restart" delay="5 sec"/>
</service>"#,
            self.platform_info.service_name,
            std::env::current_exe()
                .map_err(|e| AirError::FileSystem(format!("Failed to get executable path: {}", e)))?
                .display()
        );
        
        Ok(service_content)
    }
    
    /// Install daemon as system service
    pub async fn install_service(&self) -> Result<()> {
        info!("[Daemon] Installing system service...");
        
        match self.platform_info.platform {
            Platform::Linux => self.install_systemd_service().await,
            Platform::MacOS => self.install_launchd_service().await,
            Platform::Windows => self.install_windows_service().await,
            Platform::Unknown => Err(AirError::ServiceUnavailable(
                "Unknown platform, cannot install service".to_string()
            )),
        }
    }
    
    /// Install systemd service
    async fn install_systemd_service(&self) -> Result<()> {
        let service_file_content = self.generate_systemd_service()?;
        let service_file_path = format!("/etc/systemd/system/{}.service", self.platform_info.service_name);
        
        fs::write(&service_file_path, service_file_content).map_err(|e| {
            AirError::FileSystem(format!("Failed to write service file: {}", e))
        })?;
        
        info!("[Daemon] Systemd service installed at {}", service_file_path);
        Ok(())
    }
    
    /// Install launchd service
    async fn install_launchd_service(&self) -> Result<()> {
        let service_file_content = self.generate_launchd_service()?;
        let service_file_path = format!("/Library/LaunchDaemons/{}.plist", self.platform_info.service_name);
        
        fs::write(&service_file_path, service_file_content).map_err(|e| {
            AirError::FileSystem(format!("Failed to write service file: {}", e))
        })?;
        
        info!("[Daemon] Launchd service installed at {}", service_file_path);
        Ok(())
    }
    
    /// Install Windows service
    async fn install_windows_service(&self) -> Result<()> {
        // Windows service installation would require additional tools
        // For now, just generate the configuration file
        let service_file_content = self.generate_windows_service()?;
        let service_file_path = format!("C:\\ProgramData\\Air\\{}.xml", self.platform_info.service_name);
        
        fs::write(&service_file_path, service_file_content).map_err(|e| {
            AirError::FileSystem(format!("Failed to write service file: {}", e))
        })?;
        
        info!("[Daemon] Windows service configuration written to {}", service_file_path);
        warn!("[Daemon] Manual service installation required on Windows");
        Ok(())
    }
    
    /// Uninstall system service
    pub async fn uninstall_service(&self) -> Result<()> {
        info!("[Daemon] Uninstalling system service...");
        
        match self.platform_info.platform {
            Platform::Linux => self.uninstall_systemd_service().await,
            Platform::MacOS => self.uninstall_launchd_service().await,
            Platform::Windows => self.uninstall_windows_service().await,
            Platform::Unknown => Err(AirError::ServiceUnavailable(
                "Unknown platform, cannot uninstall service".to_string()
            )),
        }
    }
    
    /// Uninstall systemd service
    async fn uninstall_systemd_service(&self) -> Result<()> {
        let service_file_path = format!("/etc/systemd/system/{}.service", self.platform_info.service_name);
        
        if fs::remove_file(&service_file_path).is_err() {
            warn!("[Daemon] Service file {} not found", service_file_path);
        }
        
        info!("[Daemon] Systemd service uninstalled");
        Ok(())
    }
    
    /// Uninstall launchd service
    async fn uninstall_launchd_service(&self) -> Result<()> {
        let service_file_path = format!("/Library/LaunchDaemons/{}.plist", self.platform_info.service_name);
        
        if fs::remove_file(&service_file_path).is_err() {
            warn!("[Daemon] Service file {} not found", service_file_path);
        }
        
        info!("[Daemon] Launchd service uninstalled");
        Ok(())
    }
    
    /// Uninstall Windows service
    async fn uninstall_windows_service(&self) -> Result<()> {
        let service_file_path = format!("C:\\ProgramData\\Air\\{}.xml", self.platform_info.service_name);
        
        if fs::remove_file(&service_file_path).is_err() {
            warn!("[Daemon] Service file {} not found", service_file_path);
        }
        
        info!("[Daemon] Windows service configuration removed");
        Ok(())
    }
}

/// Daemon status information
#[derive(Debug, Clone)]
pub struct DaemonStatus {
    pub is_running: bool,
    pub pid_file_exists: bool,
    pub pid: Option<u32>,
    pub platform: Platform,
    pub service_name: String,
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
    fn from(code: ExitCode) -> i32 {
        code as i32
    }
}
