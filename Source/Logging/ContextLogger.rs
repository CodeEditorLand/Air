//! Context-aware logger for structured logging.

use std::{path::Path,sync::{Arc, Mutex}};
use tracing_subscriber::{fmt::format::FmtSpan, prelude::*};
use tracing_appender::rolling::Rotation;
use crate::{Result, dev_log};
use crate::Logging::LogContext::{GetLogContext, LogContext};
use crate::Logging::LogRotationConfig::LogRotationConfig;
use crate::Logging::SensitiveDataConfig::SensitiveDataConfig;
use crate::Logging::SensitiveDataFilter::SensitiveDataFilter;
/// Context-aware logger for structured logging
#[derive(Debug, Clone)]
pub struct ContextLogger {
	json_output:bool,

	log_file_path:Option<String>,

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
		let mut initialized = self.initialized.lock().unwrap_or_else(|e| e.into_inner());

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

static LOGGER_INSTANCE: std::sync::OnceLock<ContextLogger> = std::sync::OnceLock::new();
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
