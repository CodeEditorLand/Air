//! # Structured Logging Module
//!
//! Provides structured logging with JSON output format, request ID propagation,
//! context-aware logging, and log rotation support.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn, error};
use tracing_subscriber::prelude::*;
use tracing_subscriber::fmt::format::FmtSpan;

use crate::Result;

/// Context for structured logging with request IDs and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogContext {
    pub request_id: String,
    pub trace_id: String,
    pub span_id: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub operation: String,
    pub metadata: HashMap<String, String>,
}

impl LogContext {
    /// Create a new log context
    pub fn new(operation: impl Into<String>) -> Self {
        let request_id = crate::utils::GenerateRequestId();
        let trace_id = crate::utils::GenerateRequestId();
        let span_id = uuid::Uuid::new_v4().to_string();
        
        Self {
            request_id,
            trace_id,
            span_id,
            user_id: None,
            session_id: None,
            operation: operation.into(),
            metadata: HashMap::new(),
        }
    }
    
    /// Set user ID in context
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
    
    /// Set session ID in context
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }
    
    /// Add metadata to context
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
    
    /// Add multiple metadata entries
    pub fn with_metadata_map(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata.extend(metadata);
        self
    }
}

thread_local! {
    static LOG_CONTEXT: std::cell::RefCell<Option<LogContext>> = std::cell::RefCell::new(None);
}

/// Set the log context for the current thread
pub fn set_log_context(context: LogContext) {
    LOG_CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = Some(context);
    });
}

/// Get the current log context
pub fn get_log_context() -> Option<LogContext> {
    LOG_CONTEXT.with(|ctx| ctx.borrow().clone())
}

/// Clear the log context for the current thread
pub fn clear_log_context() {
    LOG_CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = None;
    });
}

/// Context-aware logger for structured logging
#[derive(Debug, Clone)]
pub struct ContextLogger {
    json_output: bool,
    log_file_path: Option<String>,
}

impl ContextLogger {
    /// Create a new context logger
    pub fn new(json_output: bool, log_file_path: Option<String>) -> Self {
        Self {
            json_output,
            log_file_path,
        }
    }
    
    /// Initialize the logging system with tracing
    pub fn initialize(&self) -> Result<()> {
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
            
            let registry = tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer);
            
            // Set up log file if specified
            if let Some(ref log_path) = self.log_file_path {
                let file_appender = tracing_appender::rolling::daily(
                    std::path::Path::new(log_path).parent().unwrap_or(std::path::Path::new(".")),
                    std::path::Path::new(log_path).file_name().unwrap_or(std::ffi::OsStr::new("air.log"))
                );
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
                
                registry.with(file_layer)
                    .init();
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
            
            let registry = tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer);
            
            registry.init();
        }
        
        info!("[Logging] ContextLogger initialized - JSON output: {}", self.json_output);
        Ok(())
    }
    
    /// Log with context at info level
    pub fn info(&self, message: impl Into<String>) {
        let msg = message.into();
        if let Some(ctx) = get_log_context() {
            info!(
                request_id = %ctx.request_id,
                trace_id = %ctx.trace_id,
                span_id = %ctx.span_id,
                operation = %ctx.operation,
                "{}", msg
            );
        } else {
            info!("{}", msg);
        }
    }
    
    /// Log with context at debug level
    pub fn debug(&self, message: impl Into<String>) {
        let msg = message.into();
        if let Some(ctx) = get_log_context() {
            debug!(
                request_id = %ctx.request_id,
                trace_id = %ctx.trace_id,
                span_id = %ctx.span_id,
                operation = %ctx.operation,
                "{}", msg
            );
        } else {
            debug!("{}", msg);
        }
    }
    
    /// Log with context at warn level
    pub fn warn(&self, message: impl Into<String>) {
        let msg = message.into();
        if let Some(ctx) = get_log_context() {
            warn!(
                request_id = %ctx.request_id,
                trace_id = %ctx.trace_id,
                span_id = %ctx.span_id,
                operation = %ctx.operation,
                "{}", msg
            );
        } else {
            warn!("{}", msg);
        }
    }
    
    /// Log with context at error level
    pub fn error(&self, message: impl Into<String>) {
        let msg = message.into();
        if let Some(ctx) = get_log_context() {
            error!(
                request_id = %ctx.request_id,
                trace_id = %ctx.trace_id,
                span_id = %ctx.span_id,
                operation = %ctx.operation,
                "{}", msg
            );
        } else {
            error!("{}", msg);
        }
    }
}

/// Global context logger instance
static LOGGER_INSTANCE: std::sync::OnceLock<ContextLogger> = std::sync::OnceLock::new();

/// Get the global context logger
pub fn get_logger() -> &'static ContextLogger {
    LOGGER_INSTANCE.get_or_init(|| ContextLogger::new(false, None))
}

/// Initialize the global context logger
pub fn initialize_logger(json_output: bool, log_file_path: Option<String>) -> Result<()> {
    let logger = ContextLogger::new(json_output, log_file_path);
    logger.initialize()?;
    let _old = LOGGER_INSTANCE.set(logger);
    Ok(())
}

/// Macro for creating a log span with context
#[macro_export]
macro_rules! log_span {
    ($operation:expr) => {{
        let context = $crate::Logging::LogContext::new($operation);
        $crate::Logging::set_log_context(context.clone());
        tracing::info_span!(
            "operation",
            request_id = %&context.request_id,
            trace_id = %&context.trace_id,
            span_id = %&context.span_id,
            operation = %context.operation,
        )
    }};
}

/// Macro for automatic context propagation in async functions
#[macro_export]
macro_rules! with_log_context {
    ($context:expr, $future:expr) => {{
        $crate::Logging::set_log_context($context);
        let result = $future.await;
        $crate::Logging::clear_log_context();
        result
    }};
}
