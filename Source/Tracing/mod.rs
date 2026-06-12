//! # Distributed Tracing Module
//!
//! Provides distributed tracing support with trace ID generation, span
//! collection, correlation ID propagation, trace export capabilities, and
//! sampled tracing for performance.
//!
//! ## Responsibilities
//!
//! ### Trace Generation
//! - Unique trace ID generation using UUID v4
//! - Span ID generation for hierarchical tracing
//! - Distributed trace parent-child relationships
//! - Trace context propagation across service boundaries
//!
//! ### Span Management
//! - Span lifecycle management (started, active, completed, failed)
//! - Span attribute and event tracking
//! - Duration measurement with microsecond precision
//! - Automatic span cleanup with TTL
//!
//! ### Distributed Tracing Integration
//! - W3C Trace Context format compatibility
//! - Correlation ID propagation for request tracking
//! - OpenTelemetry integration support
//! - B3 header format support for Zipkin/Jaeger
//!
//! ### Sampled Tracing
//! - Trace sampling to avoid performance degradation
//! - Configurable sampling rates by endpoint
//! - Head-based sampling for high-volume requests
//! - Tail-based sampling candidate tracking
//!
//! ## Integration with Mountain
//!
//! Tracing coordinates with Wind services:
//! - Distributed traces across all Element daemons
//! - Wind services consume trace data for analysis
//! Real-time trace visualization in Mountain UI
//! - Cross-service latency and dependency mapping
//!
//! ## VSCode Debugging References
//!
//! Similar tracing patterns used in VSCode for:
//! - Cross-process communication tracing
//! - Extension host performance profiling
//! - Language server protocol debugging
//! - IPC message flow tracking
//!
//! Reference:
//! vs/base/common/errors
//!
//! ## Performance Considerations
//!
//! - Trace sampling based on request volume and importance
//! - Async span storage to avoid blocking service paths
//! - Bounded span cache with automatic cleanup
//! - Lock-free where possible for high-frequency operations
//!
//! # FUTURE Enhancements
//!
//! - `OPENTELEMETRY`: Full OpenTelemetry SDK integration
//! - `SAMPLING`: Implement dynamic/tail-based sampling
//! - `EXPORT`: OpenTelemetry Protocol (OTLP) export to Jaeger/Zipkin
//! - `ANALYSIS`: Automatic anomaly detection in traces
//! - `METRICS`: Trace-derived custom metrics
//! ## Sensitive Data Handling
//!
//! Tracing automatically excludes sensitive data:
//! - No request payloads in span attributes
//! - No authentication tokens in trace headers
//! - No user-identifiable information in spans
//! - Error messages are sanitized before export

// ── Sub-modules ──────────────────────────────────────────────────────────────
//
// Types are accessed via three-segment paths per project convention:
//   crate::Tracing::TraceGenerator::TraceGenerator
//   crate::Tracing::PropagationContext::PropagationContext
// Free functions are re-exported below for ergonomic access.

pub mod PropagationContext;
pub mod SamplingConfig;
pub mod SpanEvent;
pub mod SpanStatus;
pub mod TraceGenerator;
pub mod TraceMetadata;
pub mod TraceSpan;
pub mod TraceStatistics;
pub mod TraceStatus;

// ── Re-exports: Free functions ───────────────────────────────────────────────

pub use PropagationContext::create_propagation_context;
pub use PropagationContext::create_trace_context_header;
pub use PropagationContext::get_propagation_context;
pub use PropagationContext::set_propagation_context;

pub use TraceGenerator::get_trace_generator;
pub use TraceGenerator::initialize;
pub use TraceGenerator::initialize_tracing;
