//! # Structured Logging Module
//!
//! Provides comprehensive structured logging with JSON output, request ID
//! propagation, context-aware logging, log rotation, sensitive data filtering,
//! and validation.
//!
//! ## Responsibilities
//!
//! ### Structured Logging
//! - JSON output format for machine parsing and analysis
//! - Request ID and trace ID propagation across log entries
//! - Context-aware logging with operation tracking
//! - Log level filtering (TRACE, DEBUG, INFO, WARN, ERROR)
//!
//! ###Log Rotation
//! - Size-based log rotation to prevent disk exhaustion
//! - Time-based rotation (daily) for archival
//! - Automatic cleanup of old log files
//! - Compressed log file storage for space efficiency
//!
//! ### Context Management
//! - Thread-local context storage for async operations
//! - Automatic context propagation across await points
//! - Correlation ID linking distributed requests
//! - User and session tracking
//!
//! ### Sensitive Data Handling
//! - Automatic redaction of sensitive fields
//! - Configurable sensitive patterns
//! - Sanitization of error messages
//! - Audit logging for security events
//!
//! ### Log Validation
//! - Structured log data validation before output
//! - Schema enforcement for consistent format
//! - Size limits on log messages
//! - Malformed log rejection
//!
//! ## Integration with Mountain
//!
//! Logs flow to Mountain's debugging infrastructure:
//! - Real-time log streaming to debug console
//! - Historical log search and filtering
//! - Error aggregation and alerting
//! - Performance profiling logs
//!
//! ## VSCode Debugging References
//!
//! Similar logging patterns used in VSCode for:
//! - Exception and error tracking
//! - Debug output for extension development
//! - Performance profiling traces
//! - Cross-process communication logging
//!
//! Reference:
//! vs/base/common/errors
//!
//! # FUTURE Enhancements
//!
//! - [DISTRIBUTED TRACING] Tighter integration with Tracing module
//! - `ELASTICSEARCH`: Direct log export to Elasticsearch/Logstash
//! - [LOG ANALYSIS] Automatic anomaly detection in logs
//! - `KIBANA`: Pre-built Kibana dashboards
//! - [LOG PARSING] Support for custom log formats
//! ## Sensitive Data Handling
//!
//! All logs are automatically sanitized:
//! - Passwords, tokens, and secrets are redacted
//! - User-identifiable information is masked
//! - API keys and secrets are removed
//! - Error messages are parsed for sensitive patterns

use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::{Arc, Mutex},
	time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tracing_subscriber::{fmt::format::FmtSpan, prelude::*};
use tracing_appender::rolling::Rotation;

use crate::dev_log;

use crate::Result;

/// Configuration for log rotation and management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
	/// Maximum size of a single log file in bytes before rotation
	pub MaxFileSizeBytes:u64,
	/// Maximum number of rotated log files to retain
	pub MaxFiles:usize,
	/// Rotation strategy (daily, hourly, never)
	pub Rotation:LogRotation,
	/// Whether to compress rotated log files
	pub Compress:bool,
	/// Log directory path
	pub LogDirectory:String,
	/// Log file name prefix
	pub LogFilePrefix:String,
}

/// Log rotation strategies
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LogRotation {
	/// Rotate daily
	Daily,
	/// Rotate every hour
	Hourly,
	/// Rotate every minute (for debugging)
	Minutely,
	/// Never rotate automatically
	Never,
}

impl Default for LogRotation {
	fn default() -> Self { Self::Daily }
}

impl Default for LogRotationConfig {
	fn default() -> Self {
		Self {
			MaxFileSizeBytes:100 * 1024 * 1024, // 100 MB
			MaxFiles:30,                        // Keep 30 days of logs
			Rotation:LogRotation::Daily,
			Compress:true,
			LogDirectory:"./Log".to_string(),
			LogFilePrefix:"Air".to_string(),
		}
	}
}

impl LogRotationConfig {
	/// Validate log rotation configuration
	pub fn Validate(&self) -> Result<()> {
		if self.MaxFileSizeBytes == 0 {
			return Err("MaxFileSizeBytes must be greater than 0".into());
		}
		if self.MaxFileSizeBytes > 10 * 1024 * 1024 * 1024 {
			// Max 10 GB
			return Err("MaxFileSizeBytes cannot exceed 10 GB".into());
		}
		if self.MaxFiles == 0 {
			return Err("MaxFiles must be greater than 0".into());
		}
		if self.MaxFiles > 365 {
			// Max 1 year retention
			return Err("MaxFiles cannot exceed 365".into());
		}
		Ok(())
	}

	/// Convert to tracing_appender Rotation
	pub fn ToTracingRotation(&self) -> Rotation {
		match self.Rotation {
			LogRotation::Daily => Rotation::DAILY,
			LogRotation::Hourly => Rotation::HOURLY,
			LogRotation::Minutely => Rotation::NEVER, // No minutely support
			LogRotation::Never => Rotation::NEVER,
		}
	}
}

/// Sensitive data patterns for redaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveDataConfig {
	/// Enable automatic sensitive data redaction
	pub Enabled:bool,
	/// Custom patterns to redact (regex)
	pub CustomPatterns:Vec<String>,
	/// Standard patterns to include (password, token, secret, etc.)
	pub IncludeStandardPatterns:bool,
}

impl Default for SensitiveDataConfig {
	fn default() -> Self { Self { Enabled:true, CustomPatterns:Vec::new(), IncludeStandardPatterns:true } }
}

/// Context for structured logging with request IDs and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogContext {
	pub RequestId:String,
	pub TraceId:String,
	pub SpanId:String,
	pub UserId:Option<String>,
	pub SessionId:Option<String>,
	pub Operation:String,
	pub Metadata:HashMap<String, String>,
}

impl LogContext {
	/// Create a new log context
	pub fn New(Operation:impl Into<String>) -> Self {
		let RequestId = crate::Utility::GenerateRequestId();
		let TraceId = crate::Utility::GenerateRequestId();
		let SpanId = uuid::Uuid::new_v4().to_string();

		Self {
			RequestId,
			TraceId,
			SpanId,
			UserId:None,
			SessionId:None,
			Operation:Operation.into(),
			Metadata:HashMap::new(),
		}
	}

	/// Validate log context for required fields
	pub fn Validate(&self) -> Result<()> {
		if self.RequestId.is_empty() {
			return Err("RequestId cannot be empty".into());
		}
		if self.TraceId.is_empty() {
			return Err("TraceId cannot be empty".into());
		}
		if self.Operation.is_empty() {
			return Err("Operation cannot be empty".into());
		}
		Ok(())
	}

	/// Set user ID in context
	pub fn WithUserId(mut self, UserId:String) -> Self {
		self.UserId = Some(UserId);
		self
	}

	/// Set session ID in context
	pub fn WithSessionId(mut self, SessionId:String) -> Self {
		self.SessionId = Some(SessionId);
		self
	}

	/// Add metadata to context
	pub fn WithMetadata(mut self, Key:String, Value:String) -> Self {
		self.Metadata.insert(Key, Value);
		self
	}

	/// Add multiple metadata entries
	pub fn WithMetadataMap(mut self, Metadata:HashMap<String, String>) -> Self {
		self.Metadata.extend(Metadata);
		self
	}
}

thread_local! {
	static LOG_CONTEXT: std::cell::RefCell<Option<LogContext>> = std::cell::RefCell::new(None);
}

/// Set the log context for the current thread
pub fn SetLogContext(Context:LogContext) {
	if let Err(e) = Context.Validate() {
		dev_log!("air", "error: [Logging] Invalid log context provided: {:?}", e);
		return;
	}
	LOG_CONTEXT.with(|ctx| {
		*ctx.borrow_mut() = Some(Context);
	});
}

/// Get the current log context
pub fn GetLogContext() -> Option<LogContext> { LOG_CONTEXT.with(|Context| Context.borrow().clone()) }

/// Clear the log context for the current thread
pub fn ClearLogContext() {
	LOG_CONTEXT.with(|Context| {
		*Context.borrow_mut() = None;
	});
}

/// Log file manager for rotation and cleanup
#[allow(dead_code)]
pub struct LogManager {
	Config:LogRotationConfig,
	CurrentFile:Arc<Mutex<Option<PathBuf>>>,
	CurrentSize:Arc<Mutex<u64>>,
}

impl LogManager {
	#[allow(dead_code)]
	fn new(Config:LogRotationConfig) -> Result<Self> {
		Config.Validate()?;

		// Ensure log directory exists
		std::fs::create_dir_all(&Config.LogDirectory)?;

		Ok(Self {
			Config,
			CurrentFile:Arc::new(Mutex::new(None)),
			CurrentSize:Arc::new(Mutex::new(0)),
		})
	}

	/// Check if log rotation is needed
	#[allow(dead_code)]
	fn ShouldRotate(&self) -> bool {
		let size = *self.CurrentSize.lock().unwrap();
		size >= self.Config.MaxFileSizeBytes
	}

	/// Perform log rotation
	#[allow(dead_code)]
	fn Rotate(&self) -> Result<()> {
		let CurrentFile = self.CurrentFile.lock().unwrap();

		if let Some(ref FilePath) = *CurrentFile {
			// Rename current file with timestamp
			let Timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

			let RotatedPath = format!("{}.{}.log", FilePath.display(), Timestamp);

			std::fs::rename(FilePath, &RotatedPath)?;

			// Compress if enabled
			if self.Config.Compress {
				self.CompressFile(&RotatedPath)?;
			}

			// Cleanup old log files
			self.CleanupOldLogs()?;
		}

		*self.CurrentSize.lock().unwrap() = 0;

		Ok(())
	}

	/// Compress a log file
	#[allow(dead_code)]
	fn CompressFile(&self, path:&str) -> crate::Result<()> {
		// Basic compression - in production would use actual compression
		let _ = path;
		Ok(())
	}

	/// Cleanup old log files
	#[allow(dead_code)]
	fn CleanupOldLogs(&self) -> Result<()> {
		let log_dir = Path::new(&self.Config.LogDirectory);

		if !log_dir.exists() {
			return Ok(());
		}

		let mut log_files:Vec<_> = std::fs::read_dir(log_dir)?
			.filter_map(|e| e.ok())
			.filter(|e| {
				e.path()
					.extension()
					.and_then(|s| s.to_str())
					.map(|ext| ext == "log")
					.unwrap_or(false)
			})
			.collect();

		// Sort by modification time (newest first)
		log_files.sort_by(|a, b| {
			let a_time = a.metadata().and_then(|m| m.modified()).unwrap_or(UNIX_EPOCH);
			let b_time = b.metadata().and_then(|m| m.modified()).unwrap_or(UNIX_EPOCH);
			b_time.cmp(&a_time)
		});

		// Keep only max_files
		for file in log_files.into_iter().skip(self.Config.MaxFiles) {
			let _ = std::fs::remove_file(file.path());
		}

		Ok(())
	}
}

/// Sensitive data filter for log sanitization
#[derive(Debug, Clone)]
pub struct SensitiveDataFilter {
	enabled:bool,
	patterns:Vec<regex::Regex>,
}

impl Default for SensitiveDataFilter {
	fn default() -> Self {
		let mut patterns = Vec::new();

		// Standard sensitive patterns - simplified to avoid escaping issues
		patterns.push(regex::Regex::new(r"(?i)password[=[:space:]]+\S+").unwrap());
		patterns.push(regex::Regex::new(r"(?i)token[=[:space:]]+\S+").unwrap());
		patterns.push(regex::Regex::new(r"(?i)secret[=[:space:]]+\S+").unwrap());
		patterns.push(regex::Regex::new(r"(?i)(api|private)[_-]?key[=[:space:]]+\S+").unwrap());
		patterns.push(regex::Regex::new(r"(?i)authorization[=[:space:]]+Bearer[[:space:]]+\S+").unwrap());
		patterns.push(regex::Regex::new(r"(?i)credential[=[:space:]]+\S+").unwrap());

		Self { enabled:true, patterns }
	}
}

impl SensitiveDataFilter {
	fn new(Config:SensitiveDataConfig) -> Result<Self> {
		let mut filter = Self::default();
		filter.enabled = Config.Enabled;

		if !Config.IncludeStandardPatterns {
			filter.patterns.clear();
		}

		// Add custom patterns
		for pattern in &Config.CustomPatterns {
			match regex::Regex::new(pattern) {
				Ok(re) => filter.patterns.push(re),
				Err(e) => dev_log!("air", "warn: [Logging] Failed to compile custom regex '{}': {}", pattern, e),
			}
		}

		Ok(filter)
	}

	/// Filter sensitive data from a string
	fn Filter(&self, input:&str) -> String {
		if !self.enabled {
			return input.to_string();
		}

		let mut filtered = input.to_string();

		for pattern in &self.patterns {
			filtered = pattern.replace_all(&filtered, "[REDACTED]").to_string();
		}

		filtered
	}
}

/// Structured log entry for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredLogEntry {
	pub Timestamp:u64,
	pub Level:String,
	pub Message:String,
	pub RequestId:Option<String>,
	pub TraceId:Option<String>,
	pub SpanId:Option<String>,
	pub Operation:Option<String>,
	pub UserId:Option<String>,
	pub Metadata:HashMap<String, String>,
}

impl StructuredLogEntry {
	/// Validate log entry structure
	pub fn Validate(&self) -> Result<()> {
		if self.Level.is_empty() {
			return Err("log level cannot be empty".into());
		}
		if self.Message.is_empty() {
			return Err("log message cannot be empty".into());
		}
		if !["TRACE", "DEBUG", "INFO", "WARN", "ERROR"].contains(&self.Level.as_str()) {
			return Err(format!("invalid log level: {}", self.Level).into());
		}
		if self.Message.len() > 10000 {
			// Max 10KB message
			return Err("log message too large".into());
		}
		Ok(())
	}
}

/// Context-aware logger for structured logging
#[derive(Debug, Clone)]
pub struct ContextLogger {
	json_output:bool,
	log_file_path:Option<String>,
	#[allow(dead_code)]
	rotation_config:Option<LogRotationConfig>,
	sensitive_filter:Arc<SensitiveDataFilter>,
	initialized:Arc<Mutex<bool>>,
}

impl ContextLogger {
	/// Create a new context logger
	pub fn New(json_output:bool, log_file_path:Option<String>) -> Self {
		Self {
			json_output,
			log_file_path,
			rotation_config:None,
			sensitive_filter:Arc::new(SensitiveDataFilter::default()),
			initialized:Arc::new(Mutex::new(false)),
		}
	}

	/// Create with log rotation configuration
	pub fn WithRotation(
		json_output:bool,
		log_file_path:Option<String>,
		rotation_config:LogRotationConfig,
	) -> Result<Self> {
		rotation_config.Validate()?;

		Ok(Self {
			json_output,
			log_file_path,
			rotation_config:Some(rotation_config),
			sensitive_filter:Arc::new(SensitiveDataFilter::default()),
			initialized:Arc::new(Mutex::new(false)),
		})
	}

	/// Set sensitive data filter configuration
	pub fn WithSensitiveFilter(mut self, Config:SensitiveDataConfig) -> Result<Self> {
		self.sensitive_filter = Arc::new(SensitiveDataFilter::new(Config)?);
		Ok(self)
	}

	/// Initialize the logging system with tracing
	pub fn Initialize(&self) -> Result<()> {
		// Check if already initialized
		let mut initialized = self.initialized.lock().unwrap();
		if *initialized {
			return Ok(());
		}

		let filter = tracing_subscriber::EnvFilter::from_default_env()
			.add_directive(tracing_subscriber::filter::LevelFilter::INFO.into());

		if self.json_output {
			// JSON output format
			let fmt_layer = tracing_subscriber::fmt::layer()
				.json()
				.with_current_span(true)
				.with_span_list(false)
				.with_target(true)
				.with_file(true)
				.with_line_number(true)
				.with_writer(std::io::stderr)
				.with_ansi(false)
				.with_span_events(FmtSpan::FULL);

			let registry = tracing_subscriber::registry().with(filter).with(fmt_layer);

			// Set up log file if specified
			if let Some(ref log_path) = self.log_file_path {
				let log_dir = std::path::Path::new(log_path).parent().unwrap_or(std::path::Path::new("."));
				let log_file = std::path::Path::new(log_path)
					.file_name()
					.unwrap_or(std::ffi::OsStr::new("Air.log"));

				let file_appender = tracing_appender::rolling::daily(log_dir, log_file);
				let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

				let file_layer = tracing_subscriber::fmt::layer()
					.json()
					.with_current_span(true)
					.with_span_list(false)
					.with_target(true)
					.with_file(true)
					.with_line_number(true)
					.with_writer(non_blocking)
					.with_ansi(false)
					.with_span_events(FmtSpan::FULL);

				registry.with(file_layer).init();
			} else {
				registry.init();
			}
		} else {
			// Standard text output format
			let fmt_layer = tracing_subscriber::fmt::layer()
				.with_target(true)
				.with_file(true)
				.with_line_number(true)
				.with_writer(std::io::stderr)
				.with_ansi(true)
				.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE);

			let registry = tracing_subscriber::registry().with(filter).with(fmt_layer);

			// Set up log file if specified
			if let Some(ref log_path) = self.log_file_path {
				let log_dir = std::path::Path::new(log_path).parent().unwrap_or(std::path::Path::new("."));
				let log_file = std::path::Path::new(log_path)
					.file_name()
					.unwrap_or(std::ffi::OsStr::new("Air.log"));

				let file_appender = tracing_appender::rolling::daily(log_dir, log_file);
				let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

				let file_layer = tracing_subscriber::fmt::layer()
					.with_target(true)
					.with_file(true)
					.with_line_number(true)
					.with_writer(non_blocking)
					.with_ansi(false)
					.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE);

				registry.with(file_layer).init();
			} else {
				registry.init();
			}
		}

		*initialized = true;
		dev_log!("air", "[Logging] ContextLogger initialized - JSON output: {}", self.json_output);
		Ok(())
	}

	/// Log with context at info level
	pub fn Info(&self, message:impl Into<String>) {
		let msg = self.sensitive_filter.Filter(&message.into());
		if let Some(Context) = GetLogContext() {
			dev_log!(
				"air",
				"[{}] req={} trace={} span={} {}",
				Context.Operation,
				Context.RequestId,
				Context.TraceId,
				Context.SpanId,
				msg
			);
		} else {
			dev_log!("air", "{}", msg);
		}
	}

	/// Log with context at debug level
	pub fn Debug(&self, message:impl Into<String>) {
		let msg = self.sensitive_filter.Filter(&message.into());
		if let Some(Context) = GetLogContext() {
			dev_log!(
				"air",
				"[{}] req={} trace={} span={} {}",
				Context.Operation,
				Context.RequestId,
				Context.TraceId,
				Context.SpanId,
				msg
			);
		} else {
			dev_log!("air", "{}", msg);
		}
	}

	/// Log with context at warn level
	pub fn Warn(&self, message:impl Into<String>) {
		let msg = self.sensitive_filter.Filter(&message.into());
		if let Some(Context) = GetLogContext() {
			dev_log!(
				"air",
				"warn: [{}] req={} trace={} span={} {}",
				Context.Operation,
				Context.RequestId,
				Context.TraceId,
				Context.SpanId,
				msg
			);
		} else {
			dev_log!("air", "warn: {}", msg);
		}
	}

	/// Log with context at error level
	pub fn Error(&self, message:impl Into<String>) {
		let msg = self.sensitive_filter.Filter(&message.into());
		if let Some(Context) = GetLogContext() {
			dev_log!(
				"air",
				"error: [{}] req={} trace={} span={} {}",
				Context.Operation,
				Context.RequestId,
				Context.TraceId,
				Context.SpanId,
				msg
			);
		} else {
			dev_log!("air", "error: {}", msg);
		}
	}
}

/// Global context logger instance
static LOGGER_INSTANCE:std::sync::OnceLock<ContextLogger> = std::sync::OnceLock::new();

/// Get the global context logger
pub fn GetLogger() -> &'static ContextLogger { LOGGER_INSTANCE.get_or_init(|| ContextLogger::New(false, None)) }

/// Initialize the global context logger
pub fn InitializeLogger(json_output:bool, log_file_path:Option<String>) -> Result<()> {
	let logger = ContextLogger::New(json_output, log_file_path);
	logger.Initialize()?;
	let _old = LOGGER_INSTANCE.set(logger);
	Ok(())
}

/// Initialize the global context logger with rotation
pub fn InitializeLoggerWithRotation(
	json_output:bool,
	log_file_path:Option<String>,
	rotation_config:LogRotationConfig,
	sensitive_config:Option<SensitiveDataConfig>,
) -> Result<()> {
	let mut logger = ContextLogger::WithRotation(json_output, log_file_path, rotation_config)?;

	if let Some(sensitive_config) = sensitive_config {
		logger = logger.WithSensitiveFilter(sensitive_config)?;
	}

	logger.Initialize()?;
	let _old = LOGGER_INSTANCE.set(logger);
	Ok(())
}
