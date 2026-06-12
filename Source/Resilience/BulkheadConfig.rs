//! Bulkhead configuration.
//!
//! Controls how many concurrent requests a bulkhead allows, its queue
//! depth, and the timeout for acquiring a permit.

use serde::{Deserialize, Serialize};

/// Bulkhead configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkheadConfig {
	/// Maximum concurrent requests
	pub max_concurrent:usize,

	/// Maximum queue size
	pub max_queue:usize,

	/// Request timeout (in seconds)
	pub timeout_secs:u64,
}

impl Default for BulkheadConfig {
	fn default() -> Self { Self { max_concurrent:10, max_queue:100, timeout_secs:30 } }
}
