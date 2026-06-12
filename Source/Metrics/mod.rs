//! # Metrics Collection and Export Module
//!
//! Provides Prometheus-compatible metrics collection and export for the Air
//! daemon. Includes request latency histograms, resource usage metrics, error
//! rate tracking, thread-safe metric updates with overflow protection, and
//! aggregation validation.
//!
//! ## Responsibilities
//!
//! ### Request Metrics
//! - Total request count tracking with counter overflow protection
//! - Success/failure rate calculation
//! - Latency histogram with bucket aggregation
//! - Request throughput measurement
//!
//! ### Error Metrics
//! - Total error count with classification
//! - Error rate calculation by type
//! - Error aggregation and validation
//! - Error threshold alerting
//!
//! ### Resource Metrics
//! - Memory usage monitoring with allocation tracking
//! - CPU utilization percentage
//! - Active connection count
//! - Thread activity monitoring
//! - Resource pool utilization
//!
//! ### Service-Specific Metrics
//! - Authentication success/failure tracking
//! - Download completion rates
//! - Indexing operation metrics
//! - Update deployment statistics
//!
//! ## Integration with Mountain
//!
//! Metrics flow directly to Mountain's telemetry UI:
//! - Prometheus exposition format endpoint
//! - Real-time metric streaming
//! - Historical data retention
//! - Custom dashboards and alerting
//!
//! ## VSCode Telemetry References
//!
//! Similar telemetry patterns used in VSCode for:
//! - Performance monitoring and profiling
//! - Usage statistics and feature adoption
//! - Error tracking and crash reporting
//! - Extension marketplace analytics
//!
//! Reference:
//! vs/workbench/services/telemetry
//!
//! ## Thread Safety
//!
//! All metric updates are thread-safe using:
//! - Arc for shared ownership across threads
//! - Atomic operations where possible
//! - Mutex locks for complex aggregations
//! - Lock-free counters for high-frequency updates
//!
//! # FUTURE Enhancements
//!
//! - [DISTRIBUTED TRACING] Integrate with OpenTelemetry for distributed tracing
//! metrics
//! - [CUSTOM METRICS] Add custom metric types for business KPIs
//! - `ALERTING`: Implement metric-based alerting thresholds
//! - `AGGREGATION`: Add time-windowed aggregations (1m, 5m, 15m)
//! - `EXPORT`: Add support for external monitoring systems (Datadog, New Relic)
//! ## Sensitive Data Handling
//!
//! Metrics aggregation ensures sensitive data is excluded:
//! - No request payloads in metrics
//! - No authentication tokens in labels
//! - No user-identifiable information in error classifications
//! - IP addresses and PII are aggregated, not logged individually

pub mod AggregationValidator;

pub mod GetMetrics;

pub mod MetricGuard;

pub mod MetricsCollector;

pub mod MetricsData;

pub mod MinMaxUpdate;
