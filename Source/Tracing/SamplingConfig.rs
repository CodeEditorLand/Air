use serde::{Deserialize, Serialize};

use crate::{AirError, Result};

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
			sample_rate:0.1, // 10% sampling

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
			return Err(AirError::Internal("sample_rate must be between 0.0 and 1.0".to_string()));
		}

		if self.critical_sample_rate < 0.0 || self.critical_sample_rate > 1.0 {
			return Err(AirError::Internal(
				"critical_sample_rate must be between 0.0 and 1.0".to_string(),
			));
		}

		if self.max_spans_per_trace == 0 {
			return Err(AirError::Internal("max_spans_per_trace must be greater than 0".to_string()));
		}

		if self.trace_ttl_ms == 0 {
			return Err(AirError::Internal("trace_ttl_ms must be greater than 0".to_string()));
		}

		Ok(())
	}
}
