//! # Distributed Tracing Module
//!
//! Provides distributed tracing support with trace ID generation, span collection,
//! correlation ID propagation, and trace export capabilities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::{Result, AirError};

/// Trace ID generator and manager
#[derive(Debug, Clone)]
pub struct TraceGenerator {
    trace_spans: Arc<RwLock<HashMap<String, TraceSpan>>>,
}

/// A single span in a trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
    pub span_id: String,
    pub trace_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub status: SpanStatus,
    pub attributes: HashMap<String, String>,
    pub events: Vec<SpanEvent>,
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
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
    pub timestamp: u64,
    pub name: String,
    pub attributes: HashMap<String, String>,
}

/// Distributed trace metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceMetadata {
    pub trace_id: String,
    pub root_span_id: String,
    pub total_spans: usize,
    pub root_operation: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub total_duration_ms: Option<u64>,
    pub status: TraceStatus,
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
    pub trace_id: String,
    pub span_id: String,
    pub correlation_id: String,
    pub parent_span_id: Option<String>,
}

impl TraceGenerator {
    /// Create a new trace generator
    pub fn new() -> Self {
        Self {
            trace_spans: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Generate a new trace ID
    pub fn generate_trace_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
    
    /// Generate a new span ID
    pub fn generate_span_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
    
    /// Create a new trace span
    pub async fn create_span(
        &self,
        trace_id: String,
        operation_name: impl Into<String>,
        parent_span_id: Option<String>,
    ) -> Result<TraceSpan> {
        let span_id = Self::generate_span_id();
        let operation_name = operation_name.into();
        
        let span = TraceSpan {
            span_id: span_id.clone(),
            trace_id: trace_id.clone(),
            parent_span_id: parent_span_id.clone(),
            operation_name: operation_name.clone(),
            start_time: crate::utils::CurrentTimestamp(),
            end_time: None,
            status: SpanStatus::Started,
            attributes: HashMap::new(),
            events: Vec::new(),
            error: None,
            duration_ms: None,
        };
        
        let mut spans = self.trace_spans.write().await;
        spans.insert(span_id.clone(), span.clone());
        
        Ok(span)
    }
    
    /// Add an event to a span
    pub async fn add_span_event(
        &self,
        span_id: &str,
        event_name: impl Into<String>,
        attributes: HashMap<String, String>,
    ) -> Result<()> {
        let event = SpanEvent {
            timestamp: crate::utils::CurrentTimestamp(),
            name: event_name.into(),
            attributes,
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
    pub async fn mark_span_active(&self, span_id: &str) -> Result<()> {
        let mut spans = self.trace_spans.write().await;
        if let Some(span) = spans.get_mut(span_id) {
            span.status = SpanStatus::Active;
            Ok(())
        } else {
            Err(AirError::Internal(format!("Span not found: {}", span_id)))
        }
    }
    
    /// Complete a span
    pub async fn complete_span(&self, span_id: &str, error: Option<String>) -> Result<u64> {
        let now = crate::utils::current_timestamp();
        let mut spans = self.trace_spans.write().await;
        
        if let Some(span) = spans.get_mut(span_id) {
            span.end_time = Some(now);
            span.duration_ms = Some(now - span.start_time);
            span.status = if error.is_some() {
                SpanStatus::Failed
            } else {
                SpanStatus::Completed
            };
            span.error = error;
            Ok(span.duration_ms.unwrap())
        } else {
            Err(AirError::Internal(format!("Span not found: {}", span_id)))
        }
    }
    
    /// Add attribute to a span
    pub async fn add_span_attribute(
        &self,
        span_id: &str,
        key: String,
        value: String,
    ) -> Result<()> {
        let mut spans = self.trace_spans.write().await;
        if let Some(span) = spans.get_mut(span_id) {
            span.attributes.insert(key, value);
            Ok(())
        } else {
            Err(AirError::Internal(format!("Span not found: {}", span_id)))
        }
    }
    
    /// Get a span
    pub async fn get_span(&self, span_id: &str) -> Result<TraceSpan> {
        let spans = self.trace_spans.read().await;
        spans.get(span_id)
            .cloned()
            .ok_or_else(|| AirError::Internal(format!("Span not found: {}", span_id)))
    }
    
    /// Get all spans for a trace
    pub async fn get_trace_spans(&self, trace_id: &str) -> Result<Vec<TraceSpan>> {
        let spans = self.trace_spans.read().await;
        Ok(spans.values()
            .filter(|span| span.trace_id == trace_id)
            .cloned()
            .collect())
    }
    
    /// Get trace metadata
    pub async fn get_trace_metadata(&self, trace_id: &str) -> Result<TraceMetadata> {
        let trace_spans = self.get_trace_spans(trace_id).await?;
        
        if trace_spans.is_empty() {
            return Err(AirError::Internal(format!("Trace not found: {}", trace_id)));
        }
        
        let root_span = trace_spans.iter()
            .find(|s| s.parent_span_id.is_none())
            .ok_or_else(|| AirError::Internal("No root span found".to_string()))?;
        
        let total_duration_ms = trace_spans.iter()
            .filter_map(|s| s.duration_ms)
            .max();
        
        let status = if trace_spans.iter().any(|s| s.status == SpanStatus::Failed) {
            TraceStatus::Failed
        } else if trace_spans.iter().all(|s| s.status == SpanStatus::Completed || s.status == SpanStatus::Failed) {
            TraceStatus::Completed
        } else {
            TraceStatus::InProgress
        };
        
        let end_time = trace_spans.iter()
            .filter_map(|s| s.end_time)
            .max();
        
        Ok(TraceMetadata {
            trace_id: trace_id.to_string(),
            root_span_id: root_span.span_id.clone(),
            total_spans: trace_spans.len(),
            root_operation: root_span.operation_name.clone(),
            start_time: root_span.start_time,
            end_time,
            total_duration_ms,
            status,
        })
    }
    
    /// Export trace in JSON format
    pub async fn export_trace(&self, trace_id: &str) -> Result<String> {
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
    pub async fn cleanup_old_spans(&self, older_than_ms: u64) -> Result<usize> {
        let now = crate::utils::current_timestamp();
        let mut spans = self.trace_spans.write().await;
        let original_len = spans.len();
        
        spans.retain(|_, span| {
            span.end_time.map_or(true, |end| now - end < older_than_ms)
        });
        
        Ok(original_len - spans.len())
    }
}

impl Default for TraceGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Global trace generator instance
static TRACE_GENERATOR: std::sync::OnceLock<TraceGenerator> = std::sync::OnceLock::new();

/// Get or initialize the global trace generator
pub fn get_trace_generator() -> &'static TraceGenerator {
    TRACE_GENERATOR.get_or_init(TraceGenerator::new)
}

/// Initialize the global trace generator
pub fn initialize_tracing() -> Result<()> {
    let _generator = get_trace_generator();
    log::info!("[Tracing] Trace generator initialized");
    Ok(())
}

thread_local! {
    static PROPAGATION_CONTEXT: std::cell::RefCell<Option<PropagationContext>> = std::cell::RefCell::new(None);
}

/// Set the propagation context for the current thread
pub fn set_propagation_context(context: PropagationContext) {
    PROPAGATION_CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = Some(context);
    });
}

/// Get the current propagation context
pub fn get_propagation_context() -> Option<PropagationContext> {
    PROPAGATION_CONTEXT.with(|ctx| ctx.borrow().clone())
}

/// Create a propagation context from a trace span
pub async fn create_propagation_context(trace_id: String, parent_span_id: Option<String>) -> PropagationContext {
    let span_id = TraceGenerator::generate_span_id();
    let correlation_id = crate::utils::GenerateRequestId();
    
    PropagationContext {
        trace_id,
        span_id,
        correlation_id,
        parent_span_id,
    }
}
