use serde::{Deserialize, Serialize};

// =============================================================================
// Configuration Main Structure
// =============================================================================

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirConfiguration {
	/// Configuration schema version for migration tracking
	#[serde(default = "default_schema_version")]
	pub SchemaVersion:String,

	/// Profile name (dev, staging, prod, custom)
	#[serde(default = "default_profile")]
	pub Profile:String,

	/// gRPC server configuration
	pub gRPC:gRPCConfig,

	/// Authentication configuration
	pub Authentication:AuthConfig,

	/// Update configuration
	pub Updates:UpdateConfig,

	/// Download configuration
	pub Downloader:DownloadConfig,

	/// Indexing configuration
	pub Indexing:IndexingConfig,

	/// Logging configuration
	pub Logging:LoggingConfig,

	/// Performance configuration
	pub Performance:PerformanceConfig,
}

fn default_schema_version() -> String { "1.0.0".to_string() }

fn default_profile() -> String { "dev".to_string() }

/// gRPC server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct gRPCConfig {
	/// Bind address for gRPC server
	/// Validation: Must be a valid IP:port or hostname:port combination
	/// Format: `[IPv6]`:port or IPv4:port or hostname:port
	/// Example: `"[::1]:50053"`, `"127.0.0.1:50053"`, `"localhost:50053"`
	#[serde(default = "default_grpc_bind_address")]
	pub BindAddress:String,

	/// Maximum concurrent connections
	/// Validation: Range [10, 10000]
	/// Default: 100
	#[serde(default = "default_grpc_max_connections")]
	pub MaxConnections:u32,

	/// Request timeout in seconds
	/// Validation: Range [1, 3600] (1 second to 1 hour)
	/// Default: 30
	#[serde(default = "default_grpc_request_timeout")]
	pub RequestTimeoutSecs:u64,
}

fn default_grpc_bind_address() -> String { "[::1]:50053".to_string() }

fn default_grpc_max_connections() -> u32 { 100 }

fn default_grpc_request_timeout() -> u64 { 30 }

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
	/// Enable authentication service
	#[serde(default = "default_auth_enabled")]
	pub Enabled:bool,

	/// Path to credentials storage
	/// Validation: Must be a valid absolute or home-relative path
	/// Security: Ensures directory traversal prevention
	/// Default: "~/.Air/credentials"
	#[serde(default = "default_auth_credentials_path")]
	pub CredentialsPath:String,

	/// Token expiration in hours
	/// Validation: Range [1, 8760] (1 hour to 1 year)
	/// Default: 24
	#[serde(default = "default_auth_token_expiration")]
	pub TokenExpirationHours:u32,

	/// Maximum concurrent auth sessions
	/// Validation: Range [1, 1000]
	/// Default: 10
	#[serde(default = "default_auth_max_sessions")]
	pub MaxSessions:u32,
}

fn default_auth_enabled() -> bool { true }

fn default_auth_credentials_path() -> String { "~/.Air/credentials".to_string() }

fn default_auth_token_expiration() -> u32 { 24 }

fn default_auth_max_sessions() -> u32 { 10 }

/// Update configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
	/// Enable update service
	#[serde(default = "default_update_enabled")]
	pub Enabled:bool,

	/// Update check interval in hours
	/// Validation: Range [1, 168] (1 hour to 1 week)
	/// Default: 6
	#[serde(default = "default_update_check_interval")]
	pub CheckIntervalHours:u32,

	/// Update server URL
	/// Validation: Must be a valid HTTPS URL
	/// Security: HTTPS required for security
	/// Default: <https://update.editor.land>
	#[serde(default = "default_update_server_url")]
	pub UpdateServerUrl:String,

	/// Auto-download updates
	#[serde(default = "default_update_auto_download")]
	pub AutoDownload:bool,

	/// Auto-install updates
	/// Warning: Use with caution in production
	#[serde(default = "default_update_auto_install")]
	pub AutoInstall:bool,

	/// Update channel
	/// Validation: Must be one of: "stable", "insiders", "preview"
	/// Default: "stable"
	#[serde(default = "default_update_channel")]
	pub Channel:String,
}

fn default_update_enabled() -> bool { true }

fn default_update_check_interval() -> u32 { 6 }

fn default_update_server_url() -> String { "https://update.editor.land".to_string() }

fn default_update_auto_download() -> bool { true }

fn default_update_auto_install() -> bool { false }

fn default_update_channel() -> String { "stable".to_string() }

/// Download configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadConfig {
	/// Enable download service
	#[serde(default = "default_download_enabled")]
	pub Enabled:bool,

	/// Maximum concurrent downloads
	/// Validation: Range [1, 50]
	/// Default: 5
	#[serde(default = "default_download_max_concurrent")]
	pub MaxConcurrentDownloads:u32,

	/// Download timeout in seconds
	/// Validation: Range [10, 3600] (10 seconds to 1 hour)
	/// Default: 300
	#[serde(default = "default_download_timeout")]
	pub DownloadTimeoutSecs:u64,

	/// Maximum retry attempts
	/// Validation: Range [0, 10]
	/// Default: 3
	#[serde(default = "default_download_max_retries")]
	pub MaxRetries:u32,

	/// Download cache directory
	/// Validation: Must be a valid absolute or home-relative path
	/// Default: "~/.Air/cache"
	#[serde(default = "default_download_cache_dir")]
	pub CacheDirectory:String,
}

fn default_download_enabled() -> bool { true }

fn default_download_max_concurrent() -> u32 { 5 }

fn default_download_timeout() -> u64 { 300 }

fn default_download_max_retries() -> u32 { 3 }

fn default_download_cache_dir() -> String { "~/.Air/cache".to_string() }

/// Indexing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
	/// Enable indexing service
	#[serde(default = "default_indexing_enabled")]
	pub Enabled:bool,

	/// Maximum file size to index (MB)
	/// Validation: Range [1, 1024] (1MB to 1GB)
	/// Default: 10
	#[serde(default = "default_indexing_max_file_size")]
	pub MaxFileSizeMb:u32,

	/// File types to index
	/// Format: Glob patterns like "*.rs", "*.ts", etc.
	/// Validation: Each pattern must be a valid glob pattern
	/// Default: Common source code file types
	#[serde(default = "default_indexing_file_types")]
	pub FileTypes:Vec<String>,

	/// Index update interval in minutes
	/// Validation: Range [1, 1440] (1 minute to 1 day)
	/// Default: 30
	#[serde(default = "default_indexing_update_interval")]
	pub UpdateIntervalMinutes:u32,

	/// Index storage directory
	/// Validation: Must be a valid absolute or home-relative path
	/// Default: "~/.Air/index"
	#[serde(default = "default_indexing_directory")]
	pub IndexDirectory:String,

	/// Maximum parallel indexing operations
	/// Validation: Range [1, 100] (1 to 100 concurrent operations)
	/// Default: 10
	#[serde(default = "default_max_parallel_indexing")]
	pub MaxParallelIndexing:u32,
}

fn default_indexing_enabled() -> bool { true }

fn default_indexing_max_file_size() -> u32 { 10 }

fn default_indexing_file_types() -> Vec<String> {
	vec![
		"*.rs".to_string(),
		"*.ts".to_string(),
		"*.js".to_string(),
		"*.json".to_string(),
		"*.toml".to_string(),
		"*.md".to_string(),
	]
}

fn default_indexing_update_interval() -> u32 { 30 }

fn default_indexing_directory() -> String { "~/.Air/index".to_string() }

fn default_max_parallel_indexing() -> u32 { 10 }

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
	/// Log level
	/// Validation: Must be one of: "trace", "debug", "info", "warn", "error"
	/// Default: "info"
	#[serde(default = "default_logging_level")]
	pub Level:String,

	/// Log file path
	/// Validation: Must be a valid absolute or home-relative path if provided
	/// Default: "~/.Air/logs/Air.log"
	#[serde(default = "default_logging_file_path")]
	pub FilePath:Option<String>,

	/// Enable console logging
	#[serde(default = "default_logging_console_enabled")]
	pub ConsoleEnabled:bool,

	/// Maximum log file size (MB)
	/// Validation: Range [1, 1000]
	/// Default: 10
	#[serde(default = "default_logging_max_file_size")]
	pub MaxFileSizeMb:u32,

	/// Maximum log files to keep
	/// Validation: Range [1, 50]
	/// Default: 5
	#[serde(default = "default_logging_max_files")]
	pub MaxFiles:u32,
}

fn default_logging_level() -> String { "info".to_string() }

fn default_logging_file_path() -> Option<String> { Some("~/.Air/logs/Air.log".to_string()) }

fn default_logging_console_enabled() -> bool { true }

fn default_logging_max_file_size() -> u32 { 10 }

fn default_logging_max_files() -> u32 { 5 }

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
	/// Memory usage limit (MB)
	/// Validation: Range [64, 16384] (64MB to 16GB)
	/// Default: 512
	#[serde(default = "default_perf_memory_limit")]
	pub MemoryLimitMb:u32,

	/// CPU usage limit (%)
	/// Validation: Range [10, 100]
	/// Default: 50
	#[serde(default = "default_perf_cpu_limit")]
	pub CPULimitPercent:u32,

	/// Disk usage limit (MB)
	/// Validation: Range [100, 102400] (100MB to 100GB)
	/// Default: 1024
	#[serde(default = "default_perf_disk_limit")]
	pub DiskLimitMb:u32,

	/// Background task interval in seconds
	/// Validation: Range [1, 3600] (1 second to 1 hour)
	/// Default: 60
	#[serde(default = "default_perf_task_interval")]
	pub BackgroundTaskIntervalSecs:u64,
}

fn default_perf_memory_limit() -> u32 { 512 }

fn default_perf_cpu_limit() -> u32 { 50 }

fn default_perf_disk_limit() -> u32 { 1024 }

fn default_perf_task_interval() -> u64 { 60 }

impl Default for AirConfiguration {
	fn default() -> Self {
		Self {
			SchemaVersion:default_schema_version(),

			Profile:default_profile(),

			gRPC:gRPCConfig {
				BindAddress:default_grpc_bind_address(),

				MaxConnections:default_grpc_max_connections(),

				RequestTimeoutSecs:default_grpc_request_timeout(),
			},

			Authentication:AuthConfig {
				Enabled:default_auth_enabled(),

				CredentialsPath:default_auth_credentials_path(),

				TokenExpirationHours:default_auth_token_expiration(),

				MaxSessions:default_auth_max_sessions(),
			},

			Updates:UpdateConfig {
				Enabled:default_update_enabled(),

				CheckIntervalHours:default_update_check_interval(),

				UpdateServerUrl:default_update_server_url(),

				AutoDownload:default_update_auto_download(),

				AutoInstall:default_update_auto_install(),

				Channel:default_update_channel(),
			},

			Downloader:DownloadConfig {
				Enabled:default_download_enabled(),

				MaxConcurrentDownloads:default_download_max_concurrent(),

				DownloadTimeoutSecs:default_download_timeout(),

				MaxRetries:default_download_max_retries(),

				CacheDirectory:default_download_cache_dir(),
			},

			Indexing:IndexingConfig {
				Enabled:default_indexing_enabled(),

				MaxFileSizeMb:default_indexing_max_file_size(),

				FileTypes:default_indexing_file_types(),

				UpdateIntervalMinutes:default_indexing_update_interval(),

				IndexDirectory:default_indexing_directory(),

				MaxParallelIndexing:default_max_parallel_indexing(),
			},

			Logging:LoggingConfig {
				Level:default_logging_level(),

				FilePath:default_logging_file_path(),

				ConsoleEnabled:default_logging_console_enabled(),

				MaxFileSizeMb:default_logging_max_file_size(),

				MaxFiles:default_logging_max_files(),
			},

			Performance:PerformanceConfig {
				MemoryLimitMb:default_perf_memory_limit(),

				CPULimitPercent:default_perf_cpu_limit(),

				DiskLimitMb:default_perf_disk_limit(),

				BackgroundTaskIntervalSecs:default_perf_task_interval(),
			},
		}
	}
}
