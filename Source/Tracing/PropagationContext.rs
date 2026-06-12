use serde::{Deserialize, Serialize};

use crate::{Result, Tracing::TraceGenerator::TraceGenerator};

/// Context propagation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropagationContext {
	pub TraceId:String,

	pub SpanId:String,

	pub CorrelationId:String,

	pub ParentSpanId:Option<String>,
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

	let CorrelationId = crate::Utility::GenerateRequestId();

	PropagationContext { TraceId, SpanId, CorrelationId, ParentSpanId }
}

/// Create a W3C trace context header from propagation context
pub fn create_trace_context_header(context:&PropagationContext) -> String {
	format!("traceparent=00-{}-{}-01", context.TraceId, context.SpanId)
}
