use serde::{Deserialize, Serialize};

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
	/// Default check interval in seconds
	pub DefaultCheckInterval:u64,

	/// Health history retention (number of records)
	pub HistoryRetention:usize,

	/// Consecutive failures threshold
	pub ConsecutiveFailuresThreshold:u32,

	/// Response time threshold in milliseconds
	pub ResponseTimeThresholdMs:u64,

	/// Enable automatic recovery
	pub EnableAutoRecovery:bool,

	/// Recovery timeout in seconds
	pub RecoveryTimeoutSec:u64,
}

impl Default for HealthCheckConfig {
	fn default() -> Self {
		Self {
			DefaultCheckInterval:30,

			HistoryRetention:100,

			ConsecutiveFailuresThreshold:3,

			ResponseTimeThresholdMs:5000,

			EnableAutoRecovery:true,

			RecoveryTimeoutSec:60,
		}
	}
}
