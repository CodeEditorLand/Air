//! # Resilience Patterns Module
//!
//! Provides robust resilience patterns for external service calls:
//! - Exponential backoff retry logic with jitter
//! - Circuit breaker pattern for fault isolation
//! - Bulkhead pattern for resource isolation
//! - Timeout management with cascading deadlines
//!
//! ## Responsibilities
//!
//! ### Retry Patterns
//! - Exponential backoff with jitter for distributed systems
//! - Adaptive retry policies based on error classification
//! - Retry budget management for service rate limiting
//! - Panic recovery for background retry tasks
//!
//! ### Circuit Breaker
//! - Automatic fault detection and isolation
//! - State consistency validation across transitions
//! - Event publishing for telemetry integration
//! - Half-open state monitoring for recovery testing
//!
//! ### Bulkhead Pattern
//! - Concurrent request limiting for resource protection
//! - Queue management with overflow protection
//! - Load monitoring and metrics collection
//! - Timeout validation for all operations
//!
//! ### Timeout Management
//! - Cascading deadline propagation
//! - Global deadline coordination
//! - Operation timeout enforcement
//! - Panic-safe timeout cancellation
//!
//! ## Integration with Mountain
//!
//! Resilience patterns directly support Mountain's stability by:
//! - preventing cascading failures through circuit breaker isolation
//! - managing load through bulkhead resource limits
//! - providing event publishing for Mountain's telemetry dashboard
//! - enabling adaptive retry behavior for improved service availability
//!
//! ## VSCode Stability References
//!
//! Similar patterns used in VSCode for:
//! - External service resilience (telemetry, updates, extensions)
//! - Editor process isolation and recovery
//! - Background task fault tolerance
//!
//! Reference:
//! vs/base/common/errors
//!
//! # FUTURE Enhancements
//!
//! - [DISTRIBUTED TRACING] Integrate with Tracing module for retry/circuit span
//! correlation
//! - [CUSTOM METRICS] Add detailed bulkhead load metrics to Metrics module
//! - [EVENT PUBLISHING] Extend circuit breaker events with OpenTelemetry
//! support
//! - [ADAPTIVE POLICIES] Enhance retry policies with machine learning-based
//! error prediction
//! - [METRICS INTEGRATION] Export resilience metrics to Mountain's telemetry UI
//! ## Sensitive Data Handling
//!
//! This module does not process sensitive data directly but should:
//! - Redact error messages before logging/event publishing
//! - Avoid including request payloads in resilience events
//! - Sanitize service names before publishing to telemetry

pub mod Retry;

pub mod Timeout;

pub mod CircuitState;

pub mod CircuitBreakerConfig;

pub mod CircuitEvent;

pub mod CircuitBreaker;

pub mod CircuitStatistics;

pub mod BulkheadConfig;

pub mod BulkheadStatistics;

pub mod BulkheadExecutor;

pub mod ResilienceOrchestrator;

#[cfg(test)]
mod ResilienceTests;
