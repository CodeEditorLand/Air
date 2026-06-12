use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{
	AirError,
	Result,
	Tracing::{
		PropagationContext::PropagationContext,
		SamplingConfig::SamplingConfig,
		SpanEvent::SpanEvent,
		SpanStatus::SpanStatus,
		TraceMetadata::TraceMetadata,
		TraceSpan::TraceSpan,
		TraceStatistics::TraceStatistics,
		TraceStatus::TraceStatus,
	},
	dev_log,
};

/// Trace ID generator and manager with sampling support
#[derive(Debug, Clone)]
pub struct TraceGenerator {
	trace_spans:Arc<RwLock<HashMap<String, TraceSpan>>>,

	sampling_config:Arc<RwLock<SamplingConfig>>,
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
			dev_log!("air", "error: [Tracing] Panic in generate_trace_id, using fallback: {:?}", e);

			format!("{:x}", rand::random::<u64>())
		})
	}

	/// Generate a new span ID
	pub fn generate_span_id() -> String {
		std::panic::catch_unwind(|| uuid::Uuid::new_v4().to_string()).unwrap_or_else(|e| {
			dev_log!("air", "error: [Tracing] Panic in generate_span_id, using fallback: {:?}", e);

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
			CorrelationId:crate::Utility::GenerateRequestId(),
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

			start_time:crate::Utility::CurrentTimestamp(),

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
			dev_log!(
				"air",
				"warn: [Tracing] Trace {} exceeds max spans ({}), dropping span {}",
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
			timestamp:crate::Utility::CurrentTimestamp(),

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
		let Now = crate::Utility::CurrentTimestamp();

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
		let Now = crate::Utility::CurrentTimestamp();

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

		let patterns:Vec<(&str, &str)> = vec![
			(r"(?i)password[=:]\S+", "password=[REDACTED]"),
			(r"(?i)token[=:]\S+", "token=[REDACTED]"),
			(r"(?i)secret[=:]\S+", "secret=[REDACTED]"),
			(r"(?i)(api|private)[_-]?key[=:]\S+", "api_key=[REDACTED]"),
			(
				r"(?i)authorization[=[:space:]]+Bearer[[:space:]]+\S+",
				"Authorization: Bearer ***",
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

impl Default for TraceGenerator {
	fn default() -> Self { Self::new() }
}

/// Global trace generator instance
static TRACE_GENERATOR:std::sync::OnceLock<TraceGenerator> = std::sync::OnceLock::new();

/// Get or initialize the global trace generator
pub fn get_trace_generator() -> &'static TraceGenerator { TRACE_GENERATOR.get_or_init(TraceGenerator::new) }

/// Initialize the global trace generator with custom sampling
pub fn initialize_tracing(sampling_config:Option<SamplingConfig>) -> crate::Result<()> {
	let generator = if let Some(config) = sampling_config {
		TraceGenerator::with_sampling(config)?
	} else {
		TraceGenerator::new()
	};

	let _old = TRACE_GENERATOR.set(generator);

	crate::dev_log!("air", "[Tracing] Trace generator initialized with tracing");

	Ok(())
}

/// Initialize tracing (alias for initialize_tracing with default config)
pub fn initialize() -> crate::Result<()> { initialize_tracing(None) }
