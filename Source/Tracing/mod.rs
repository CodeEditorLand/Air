//! # Distributed Tracing Module
//!
//! OpenTelemetry-compatible tracing with sampling, span management,
//! propagation contexts, and per-operation statistics.

pub mod PropagationContext;
pub mod SamplingConfig;
pub mod SpanEvent;
pub mod SpanStatus;
pub mod TraceGenerator;
pub mod TraceMetadata;
pub mod TraceSpan;
pub mod TraceStatistics;
pub mod TraceStatus;
