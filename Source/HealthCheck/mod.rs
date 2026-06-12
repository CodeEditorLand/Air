//! # Health Check System
//!
//! Provides comprehensive health monitoring for Air daemon services,
//! ensuring VSCode stability and security through multi-level health checks,
//! dependency validation, and automatic recovery mechanisms.
//!
//! ## Responsibilities
//!
//! - Monitor critical Air services (authentication, updates, downloader,
//!   indexing, gRPC, connections)
//! - Implement multi-level health checks (Alive, Responsive, Functional)
//! - Provide automatic recovery actions when services fail
//! - Track health history and performance metrics
//! - Integrate with VSCode's stability patterns for service health monitoring
//!
//! ## VSCode Stability References
//!
//! This health check system aligns with VSCode's health monitoring patterns:
//! - Service health tracking similar to VSCode's workbench service health
//! - Dependency validation matching VSCode's extension host health checks
//! - Recovery patterns inspired by VSCode's crash recovery mechanisms
//! - Performance monitoring patterns from VSCode's telemetry system
//!
//! Referenced from:
//! vs/workbench/services/telemetry
//!
//! ## Mountain Monitoring Integration
//!
//! Health check results are integrated with Mountain monitoring system:
//! - Health status updates flow to Mountain's monitoring dashboards
//! - Critical health events trigger alerts in Mountain's alerting system
//! - Health metrics are aggregated for system-wide health assessment
//! - Recovery actions are coordinated with Mountain's service management
//!
//! ## Monitoring Patterns
//!
//! ### Multi-Level Health Checks
//! - **Alive**: Basic service process check
//! - **Responsive**: Service responds to health check queries
//! - **Functional**: Service performs its core operations correctly
//!
//! ### Circuit Breaking
//! - Services are temporarily marked as unhealthy after consecutive failures
//! - Circuit breaker prevents cascading failures
//! - Automatic circuit breaker reset after cool-down period
//! - Manual circuit breaker reset available for administrative overrides
//!
//! ### Timeout Handling
//! - Each health check has a configurable timeout
//! - Timeout events trigger immediate recovery actions
//! - Timeout history tracked to identify performance degradation
//! - Adaptive timeout adjustment based on observed performance
//!
//! ## Recovery Mechanisms
//!
//! Recovery actions are triggered based on:
//! - Consecutive failure count exceeding threshold
//! - Response time exceeding configured threshold
//! - Service unresponsiveness detected
//! - Manual-triggered recovery
//!
//! Recovery actions include:
//! - Service restart (graceful shutdown and restart)
//! - Connection reset (re-establish network connections)
//! - Cache clearing (remove stale or corrupted cache)
//! - Configuration reload (refresh service configuration)
//! - Escalation (notify administrators for manual intervention)
//!
//! ## FUTURE Enhancements
//!
//! - Implement advanced metrics collection (latency percentiles, error rates)
//! - Add health check scheduling automation (cron-like scheduling)
//! - Implement predictive health analysis (machine learning-based)
//! - Add security compliance checks (PCI-DSS, GDPR, etc.)
//! - Implement distributed health checks for clustered deployments
//! - Add health check export formats (Prometheus, Grafana, etc.)
//! - Implement health check alerting through multiple channels (email, Slack,
//! etc.)
//! - Add health check simulation for testing and validation
//! ## Configuration
//!
//! Health check behavior is configurable through HealthCheckConfig:
//! - `default_check_interval`: Time between automatic health checks
//! - `history_retention`: Number of health check records to keep
//! - `consecutive_failures_threshold`: Failures before triggering recovery
//! - `response_time_threshold_ms`: Response time threshold for recovery
//! - `enable_auto_recovery`: Enable/disable automatic recovery
//! - `recovery_timeout_sec`: Maximum time for recovery actions

pub mod DegradationLevel;
pub mod HealthCheckConfig;
pub mod HealthCheckLevel;
pub mod HealthCheckManager;
pub mod HealthCheckRecord;
pub mod HealthCheckResponse;
pub mod HealthStatistics;
pub mod HealthStatus;
pub mod PerformanceIndicators;
pub mod RecoveryAction;
pub mod RecoveryActionType;
pub mod RecoveryTrigger;
pub mod ResourceWarning;
pub mod ResourceWarningType;
pub mod ServiceHealth;
pub mod WarningSeverity;
