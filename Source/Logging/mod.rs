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
//! ### Log Rotation
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

pub mod ContextLogger;

pub mod LogContext;

pub mod LogManager;

pub mod LogRotationConfig;

pub mod SensitiveDataConfig;

pub mod SensitiveDataFilter;

pub mod StructuredLogEntry;
