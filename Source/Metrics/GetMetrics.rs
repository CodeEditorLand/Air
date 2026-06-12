use crate::{Metrics::MetricsCollector::MetricsCollector, Result, dev_log};

/// Global metrics collector instance
static METRICS_INSTANCE:std::sync::OnceLock<MetricsCollector> = std::sync::OnceLock::new();

/// Get or initialize the global metrics collector
pub fn GetMetrics() -> &'static MetricsCollector { METRICS_INSTANCE.get_or_init(|| MetricsCollector::default()) }

/// Initialize the global metrics collector
pub fn InitializeMetrics() -> Result<()> {
	let _collector = GetMetrics();

	dev_log!("metrics", "[Metrics] Global metrics collector initialized");

	Ok(())
}
