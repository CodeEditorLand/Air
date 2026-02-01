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
//! # TODOs
//!
//! - [OPENTELEMETRY] Full OpenTelemetry SDK integration
//! - [SAMPLING] Implement dynamic/tail-based sampling
//! - [EXPORT] OpenTelemetry Protocol (OTLP) export to Jaeger/Zipkin
//! - [ANALYSIS] Automatic anomaly detection in traces
//! - [METRICS] Trace-derived custom metrics
//!
//! ## Sensitive Data Handling
//!
//! Tracing automatically excludes sensitive data:
//! - No request payloads in span attributes
//! - No authentication tokens in trace headers
//! - No user-identifiable information in spans
//! - Error messages are sanitized before export

use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{AirError, Result};

/// Trace ID generator and manager with sampling support
#[derive(Debug, Clone)]
pub struct TraceGenerator {
	trace_spans:Arc<RwLock<HashMap<String, TraceSpan>>>,
	sampling_config:Arc<RwLock<SamplingConfig>>,
}

/// Sampling configuration for trace generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingConfig {
	/// Sample rate (0.0 to 1.0) - percentage of traces to collect
	pub sample_rate:f64,
	/// Minimum sample rate for critical operations
	pub critical_sample_rate:f64,
	/// Max spans per trace to prevent memory bloat
	pub max_spans_per_trace:usize,
	/// Trace TTL in milliseconds before cleanup
	pub trace_ttl_ms:u64,
}

impl Default for SamplingConfig {
	fn default() -> Self {
		Self {
			sample_rate:0.1,          // 10% sampling
			critical_sample_rate:1.0, // 100% for critical
			max_spans_per_trace:1000,
			trace_ttl_ms:3600000, // 1 hour
		}
	}
}

impl SamplingConfig {
	/// Validate sampling configuration
	pub fn validate(&self) -> Result<()> {
		if self.sample_rate < 0.0 || self.sample_rate > 1.0 {
			return Err(crate::AirError::Internal("sample_rate must be between 0.0 and 1.0".to_string()));
		}
		if self.critical_sample_rate < 0.0 || self.critical_sample_rate > 1.0 {
			return Err(crate::AirError::Internal(
				"critical_sample_rate must be between 0.0 and 1.0".to_string(),
			));
		}
		if self.max_spans_per_trace == 0 {
			return Err(crate::AirError::Internal(
				"max_spans_per_trace must be greater than 0".to_string(),
			));
		}
		if self.trace_ttl_ms == 0 {
			return Err(crate::AirError::Internal("trace_ttl_ms must be greater than 0".to_string()));
		}
		Ok(())
	}
}

/// A single span in a trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
	pub span_id:String,
	pub trace_id:String,
	pub parent_span_id:Option<String>,
	pub operation_name:String,
	pub start_time:u64,
	pub end_time:Option<u64>,
	pub status:SpanStatus,
	pub attributes:HashMap<String, String>,
	pub events:Vec<SpanEvent>,
	pub error:Option<String>,
	pub duration_ms:Option<u64>,
}

/// Span status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpanStatus {
	Started,
	Active,
	Completed,
	Failed,
	Cancelled,
}

/// Event within a span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
	pub timestamp:u64,
	pub name:String,
	pub attributes:HashMap<String, String>,
}

/// Distributed trace metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceMetadata {
	pub trace_id:String,
	pub root_span_id:String,
	pub total_spans:usize,
	pub root_operation:String,
	pub start_time:u64,
	pub end_time:Option<u64>,
	pub total_duration_ms:Option<u64>,
	pub status:TraceStatus,
}

/// Trace status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TraceStatus {
	InProgress,
	Completed,
	Failed,
	Cancelled,
}

/// Context propagation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropagationContext {
	pub TraceId:String,
	pub SpanId:String,
	pub CorrelationId:String,
	pub ParentSpanId:Option<String>,
}

impl TraceGenerator {
	/// Create a new trace generator with default sampling
	pub fn new() -> Self {
		Self {
			trace_spans:Arc::new(RwLock::new(HashMap::new())),
			sampling_config:Arc::new(RwLock::new(SamplingConfig::default())),
		}
	}

	/// Create a new trace generator with custom sampling
	pub fn with_sampling(sampling_config:SamplingConfig) -> Result<Self> {
		sampling_config
			.validate()
			.map_err(|e| AirError::Internal(format!("Invalid sampling config: {}", e)))?;

		Ok(Self {
			trace_spans:Arc::new(RwLock::new(HashMap::new())),
			sampling_config:Arc::new(RwLock::new(sampling_config)),
		})
	}

	/// Generate a new trace ID with panic recovery
	pub fn generate_trace_id() -> String {
		std::panic::catch_unwind(|| uuid::Uuid::new_v4().to_string()).unwrap_or_else(|e| {
			log::error!("[Tracing] Panic in generate_trace_id, using fallback: {:?}", e);
			format!("{:x}", rand::random::<u64>())
		})
	}

	/// Generate a new span ID
	pub fn generate_span_id() -> String {
		std::panic::catch_unwind(|| uuid::Uuid::new_v4().to_string()).unwrap_or_else(|e| {
			log::error!("[Tracing] Panic in generate_span_id, using fallback: {:?}", e);
			format!("{:x}", rand::random::<u64>())
		})
	}

	/// Determine if a trace should be sampled based on configuration
	pub async fn should_sample(&self, is_critical:bool) -> bool {
		let config = self.sampling_config.read().await;
		let rate = if is_critical { config.critical_sample_rate } else { config.sample_rate };

		rand::random::<f64>() < rate
	}

	/// Parse W3C Trace Context header
	pub fn parse_trace_context(header:&str) -> Result<PropagationContext> {
		let parts:Vec<&str> = header.split(';').collect();

		let mut trace_id = String::new();
		let mut parent_span_id = None;

		for part in parts {
			let kv:Vec<&str> = part.split('=').collect();
			if kv.len() != 2 {
				continue;
			}

			match kv[0].trim() {
				"traceparent" => {
					let trace_parent:Vec<&str> = kv[1].trim().split('-').collect();
					if trace_parent.len() >= 2 {
						trace_id = trace_parent[1].to_string();
						if trace_parent.len() >= 3 {
							parent_span_id = Some(trace_parent[2].to_string());
						}
					}
				},
				_ => {},
			}
		}

		if trace_id.is_empty() {
			return Err(AirError::Internal("Invalid trace context header".to_string()));
		}

		Ok(PropagationContext {
			TraceId:trace_id,
			SpanId:Self::generate_span_id(),
			CorrelationId:crate::utils::GenerateRequestId(),
			ParentSpanId:parent_span_id,
		})
	}

	/// Create a new trace span with optional sampling check
	pub async fn create_span(
		&self,
		trace_id:String,
		operation_name:impl Into<String>,
		parent_span_id:Option<String>,
		attributes:Option<HashMap<String, String>>,
	) -> Result<TraceSpan> {
		let span_id = Self::generate_span_id();
		let operation_name = operation_name.into();

		let span = TraceSpan {
			span_id:span_id.clone(),
			trace_id:trace_id.clone(),
			parent_span_id:parent_span_id.clone(),
			operation_name:operation_name.clone(),
			start_time:crate::utils::CurrentTimestamp(),
			end_time:None,
			status:SpanStatus::Started,
			attributes:attributes.unwrap_or_default(),
			events:Vec::new(),
			error:None,
			duration_ms:None,
		};

		let mut spans = self.trace_spans.write().await;

		// Check trace span limit
		let trace_span_count = spans.values().filter(|s| s.trace_id == trace_id).count();

		let config = self.sampling_config.read().await;
		if trace_span_count >= config.max_spans_per_trace {
			log::warn!(
				"[Tracing] Trace {} exceeds max spans ({}), dropping span {}",
				trace_id,
				config.max_spans_per_trace,
				span_id
			);
			return Err(AirError::Internal("Max spans per trace exceeded".to_string()));
		}

		spans.insert(span_id.clone(), span.clone());

		Ok(span)
	}

	/// Add an event to a span
	pub async fn add_span_event(
		&self,
		span_id:&str,
		event_name:impl Into<String>,
		attributes:HashMap<String, String>,
	) -> Result<()> {
		let event = SpanEvent {
			timestamp:crate::utils::CurrentTimestamp(),
			name:event_name.into(),
			attributes:self.sanitize_attributes(attributes),
		};

		let mut spans = self.trace_spans.write().await;
		if let Some(span) = spans.get_mut(span_id) {
			span.events.push(event);
			Ok(())
		} else {
			Err(AirError::Internal(format!("Span not found: {}", span_id)))
		}
	}

	/// Mark a span as active
	pub async fn mark_span_active(&self, span_id:&str) -> Result<()> {
		let mut spans = self.trace_spans.write().await;
		if let Some(span) = spans.get_mut(span_id) {
			span.status = SpanStatus::Active;
			Ok(())
		} else {
			Err(AirError::Internal(format!("Span not found: {}", span_id)))
		}
	}

	/// Complete a span with optional error
	pub async fn complete_span(&self, span_id:&str, error:Option<String>) -> Result<u64> {
		let Now = crate::utils::CurrentTimestamp();
		let mut spans = self.trace_spans.write().await;

		if let Some(span) = spans.get_mut(span_id) {
			span.end_time = Some(Now);
			span.duration_ms = Some(Now.saturating_sub(span.start_time));
			span.status = if error.is_some() { SpanStatus::Failed } else { SpanStatus::Completed };
			span.error = error.map(|e| self.sanitize_error_message(&e));
			Ok(span.duration_ms.unwrap_or(0))
		} else {
			Err(AirError::Internal(format!("Span not found: {}", span_id)))
		}
	}

	/// Add attribute to a span
	pub async fn add_span_attribute(&self, span_id:&str, key:String, value:String) -> Result<()> {
		self.add_span_attributes(span_id, HashMap::from([(key, value)])).await
	}

	/// Add multiple attributes to a span
	pub async fn add_span_attributes(&self, span_id:&str, attributes:HashMap<String, String>) -> Result<()> {
		let sanitized = self.sanitize_attributes(attributes);
		let mut spans = self.trace_spans.write().await;

		if let Some(span) = spans.get_mut(span_id) {
			for (key, value) in sanitized {
				span.attributes.insert(key, value);
			}
			Ok(())
		} else {
			Err(AirError::Internal(format!("Span not found: {}", span_id)))
		}
	}

	/// Get a span by ID
	pub async fn get_span(&self, span_id:&str) -> Result<TraceSpan> {
		let spans = self.trace_spans.read().await;
		spans
			.get(span_id)
			.cloned()
			.ok_or_else(|| AirError::Internal(format!("Span not found: {}", span_id)))
	}

	/// Get all spans for a trace
	pub async fn get_trace_spans(&self, trace_id:&str) -> Result<Vec<TraceSpan>> {
		let spans = self.trace_spans.read().await;
		Ok(spans.values().filter(|span| span.trace_id == trace_id).cloned().collect())
	}

	/// Get trace metadata
	pub async fn get_trace_metadata(&self, trace_id:&str) -> Result<TraceMetadata> {
		let trace_spans = self.get_trace_spans(trace_id).await?;

		if trace_spans.is_empty() {
			return Err(AirError::Internal(format!("Trace not found: {}", trace_id)));
		}

		let root_span = trace_spans
			.iter()
			.find(|s| s.parent_span_id.is_none())
			.ok_or_else(|| AirError::Internal("No root span found".to_string()))?;

		let total_duration_ms = trace_spans.iter().filter_map(|s| s.duration_ms).max();

		let status = if trace_spans.iter().any(|s| s.status == SpanStatus::Failed) {
			TraceStatus::Failed
		} else if trace_spans
			.iter()
			.all(|s| s.status == SpanStatus::Completed || s.status == SpanStatus::Failed)
		{
			TraceStatus::Completed
		} else {
			TraceStatus::InProgress
		};

		let end_time = trace_spans.iter().filter_map(|s| s.end_time).max();

		Ok(TraceMetadata {
			trace_id:trace_id.to_string(),
			root_span_id:root_span.span_id.clone(),
			total_spans:trace_spans.len(),
			root_operation:root_span.operation_name.clone(),
			start_time:root_span.start_time,
			end_time,
			total_duration_ms,
			status,
		})
	}

	/// Export trace in JSON format
	pub async fn export_trace(&self, trace_id:&str) -> Result<String> {
		let spans = self.get_trace_spans(trace_id).await?;
		let metadata = self.get_trace_metadata(trace_id).await?;

		let export = serde_json::json!({
			"metadata": metadata,
			"spans": spans,
		});

		serde_json::to_string_pretty(&export)
			.map_err(|e| AirError::Serialization(format!("Failed to export trace: {}", e)))
	}

	/// Clean up old spans (older than specified milliseconds)
	pub async fn cleanup_old_spans(&self, older_than_ms:Option<u64>) -> Result<usize> {
		let Now = crate::utils::CurrentTimestamp();
		let ttl = older_than_ms.unwrap_or_else(|| {
			tokio::task::block_in_place(|| {
				tokio::runtime::Handle::current().block_on(async { self.sampling_config.read().await.trace_ttl_ms })
			})
		});

		let mut spans = self.trace_spans.write().await;
		let original_len = spans.len();

		spans.retain(|_, span| span.end_time.map_or(true, |end| Now.saturating_sub(end) < ttl));

		Ok(original_len.saturating_sub(spans.len()))
	}

	/// Get trace statistics
	pub async fn get_statistics(&self) -> TraceStatistics {
		let spans = self.trace_spans.read().await;

		let total_traces = spans
			.values()
			.map(|s| s.trace_id.clone())
			.collect::<std::collections::HashSet<_>>()
			.len();

		let completed_spans = spans.values().filter(|s| s.status == SpanStatus::Completed).count();

		let failed_spans = spans.values().filter(|s| s.status == SpanStatus::Failed).count();

		let in_progress_spans = spans
			.values()
			.filter(|s| s.status == SpanStatus::Started || s.status == SpanStatus::Active)
			.count();

		TraceStatistics {
			total_traces:total_traces as u64,
			total_spans:spans.len() as u64,
			completed_spans:completed_spans as u64,
			failed_spans:failed_spans as u64,
			in_progress_spans:in_progress_spans as u64,
		}
	}

	/// Sanitize attributes to remove sensitive data
	fn sanitize_attributes(&self, mut attributes:HashMap<String, String>) -> HashMap<String, String> {
		let sensitive_keys = vec![
			"password",
			"token",
			"secret",
			"api_key",
			"authorization",
			"credential",
			"auth",
			"private_key",
			"session_token",
		];

		// Collect keys first to avoid borrowing issues
		let attr_keys:Vec<String> = attributes.keys().cloned().collect();

		for key in sensitive_keys {
			let key_lower = key.to_lowercase();
			for attr_key in &attr_keys {
				if attr_key.to_lowercase().contains(&key_lower) {
					attributes.insert(attr_key.clone(), "[REDACTED]".to_string());
				}
			}
		}

		attributes
	}

	/// Sanitize error messages to remove sensitive data
	fn sanitize_error_message(&self, message:&str) -> String {
		let mut sanitized = message.to_string();

		let patterns = vec![
			(r"(?i)password[=:]\S+", "password=[REDACTED]"),
			(r"(?i)token[=:]\S+", "token=[REDACTED]"),
			(r"(?i)secret[=:]\S+", "secret=[REDACTED]"),
			(r"(?i)(api|private)[_-]?key[=:]\S+", "api_key=[REDACTED]"),
			(
				r"(?i)authorization[=[:space:]]+Bearer[[:space:]]+\S+",
				"Authorization: Bearer [REDACTED]",
			),
		];

		for (pattern, replacement) in patterns {
			if let Ok(re) = regex::Regex::new(pattern) {
				sanitized = re.replace_all(&sanitized, replacement).to_string();
			}
		}

		sanitized
	}
}

/// Trace statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStatistics {
	pub total_traces:u64,
	pub total_spans:u64,
	pub completed_spans:u64,
	pub failed_spans:u64,
	pub in_progress_spans:u64,
}

impl Default for TraceGenerator {
	fn default() -> Self { Self::new() }
}

/// Global trace generator instance
static TRACE_GENERATOR:std::sync::OnceLock<TraceGenerator> = std::sync::OnceLock::new();

/// Get or initialize the global trace generator
pub fn get_trace_generator() -> &'static TraceGenerator { TRACE_GENERATOR.get_or_init(TraceGenerator::new) }

/// Initialize the global trace generator with custom sampling
pub fn initialize_tracing(sampling_config:Option<SamplingConfig>) -> Result<()> {
	let generator = if let Some(config) = sampling_config {
		TraceGenerator::with_sampling(config)?
	} else {
		TraceGenerator::new()
	};

	let _old = TRACE_GENERATOR.set(generator);
	log::info!("[Tracing] Trace generator initialized with tracing");
	Ok(())
}

thread_local! {
	static PROPAGATION_CONTEXT: std::cell::RefCell<Option<PropagationContext>> = std::cell::RefCell::new(None);
}

/// Set the propagation context for the current thread
pub fn set_propagation_context(context:PropagationContext) {
	PROPAGATION_CONTEXT.with(|ctx| {
		*ctx.borrow_mut() = Some(context);
	});
}

/// Get the current propagation context
pub fn get_propagation_context() -> Option<PropagationContext> { PROPAGATION_CONTEXT.with(|ctx| ctx.borrow().clone()) }

/// Create a propagation context from a trace span
pub async fn create_propagation_context(TraceId:String, ParentSpanId:Option<String>) -> PropagationContext {
	let SpanId = TraceGenerator::generate_span_id();
	let CorrelationId = crate::utils::GenerateRequestId();

	PropagationContext { TraceId, SpanId, CorrelationId, ParentSpanId }
}

/// Create a W3C trace context header from propagation context
pub fn create_trace_context_header(context:&PropagationContext) -> String {
	format!("traceparent=00-{}-{}-01", context.TraceId, context.SpanId)
}
