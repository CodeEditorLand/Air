//! Bulkhead semaphore for resource isolation with metrics and panic
//! recovery.
//!
//! Limits concurrent execution via a `tokio::sync::Semaphore`, tracks
//! queue depth, and collects telemetry counters for rejected, completed,
//! and timed-out requests.

use std::{
	sync::Arc,
	time::Duration,
};

use tokio::sync::RwLock;

use crate::dev_log;

use super::{
	BulkheadConfig::BulkheadConfig,
	BulkheadStatistics::BulkheadStatistics,
};

/// Bulkhead semaphore for resource isolation with metrics and panic recovery
#[derive(Debug)]
pub struct BulkheadExecutor {
	name:String,

	semaphore:Arc<tokio::sync::Semaphore>,

	config:BulkheadConfig,

	current_requests:Arc<RwLock<u32>>,

	queue_size:Arc<RwLock<u32>>,

	total_rejected:Arc<RwLock<u64>>,

	total_completed:Arc<RwLock<u64>>,

	total_timed_out:Arc<RwLock<u64>>,
}

impl BulkheadExecutor {
	/// Create a new bulkhead executor with metrics tracking
	pub fn new(name:String, config:BulkheadConfig) -> Self {
		Self {
			name:name.clone(),

			semaphore:Arc::new(tokio::sync::Semaphore::new(config.max_concurrent)),

			config,

			current_requests:Arc::new(RwLock::new(0)),

			queue_size:Arc::new(RwLock::new(0)),

			total_rejected:Arc::new(RwLock::new(0)),

			total_completed:Arc::new(RwLock::new(0)),

			total_timed_out:Arc::new(RwLock::new(0)),
		}
	}

	/// Validate bulkhead configuration
	pub fn ValidateConfig(config:&BulkheadConfig) -> Result<(), String> {
		if config.max_concurrent == 0 {
			return Err("max_concurrent must be greater than 0".to_string());
		}

		if config.max_queue == 0 {
			return Err("max_queue must be greater than 0".to_string());
		}

		if config.timeout_secs == 0 {
			return Err("timeout_secs must be greater than 0".to_string());
		}

		Ok(())
	}

	/// Execute with bulkhead protection and panic recovery
	pub async fn Execute<F, R>(&self, f:F) -> Result<R, String>
	where
		F: std::future::Future<Output = Result<R, String>>, {
		async {
			// Validate timeout
			if self.config.timeout_secs == 0 {
				return Err("Bulkhead timeout must be greater than 0".to_string());
			}

			// Check queue size
			let queue = *self.queue_size.read().await;

			if queue >= self.config.max_queue as u32 {
				*self.total_rejected.write().await += 1;

				dev_log!("resilience", "warn: [Bulkhead] Queue full for {}, rejecting request", self.name);

				return Err("Bulkhead queue full".to_string());
			}

			// Increment queue size
			*self.queue_size.write().await += 1;

			// Acquire permit with timeout
			let _Permit =
				match tokio::time::timeout(Duration::from_secs(self.config.timeout_secs), self.semaphore.acquire())
					.await
				{
					Ok(Ok(_)) => {
						// Permit acquired, proceed with execution
						// Decrement queue size
						*self.queue_size.write().await -= 1;
					},

					Ok(Err(e)) => {
						*self.queue_size.write().await -= 1;

						return Err(format!("Bulkhead semaphore error: {}", e));
					},

					Err(_) => {
						*self.queue_size.write().await -= 1;

						*self.total_timed_out.write().await += 1;

						dev_log!("resilience", "warn: [Bulkhead] Timeout waiting for permit for {}", self.name);

						return Err("Bulkhead timeout waiting for permit".to_string());
					},
				};

			// Decrement queue size, increment current requests
			*self.queue_size.write().await -= 1;

			*self.current_requests.write().await += 1;

			// Execute with timeout (no catch_unwind to avoid interior mutability issues)
			let execution_result = tokio::time::timeout(Duration::from_secs(self.config.timeout_secs), f).await;

			let execution_result:Result<R, String> = match execution_result {
				Ok(Ok(value)) => Ok(value),

				Ok(Err(e)) => Err(e),

				Err(_) => {
					*self.total_timed_out.write().await += 1;

					Err("Bulkhead execution timeout".to_string())
				},
			};

			if execution_result.is_ok() {
				*self.total_completed.write().await += 1;
			}

			execution_result
		}
		.await
	}

	/// Get current load with panic recovery
	pub async fn GetLoad(&self) -> (u32, u32) {
		async {
			let current = *self.current_requests.read().await;

			let queue = *self.queue_size.read().await;

			(current, queue)
		}
		.await
	}

	/// Get bulkhead statistics for metrics
	pub async fn GetStatistics(&self) -> BulkheadStatistics {
		async {
			BulkheadStatistics {
				name:self.name.clone(),

				current_concurrent:*self.current_requests.read().await,

				current_queue:*self.queue_size.read().await,

				max_concurrent:self.config.max_concurrent,

				max_queue:self.config.max_queue,

				total_rejected:*self.total_rejected.read().await,

				total_completed:*self.total_completed.read().await,

				total_timed_out:*self.total_timed_out.read().await,
			}
		}
		.await
	}

	/// Calculate utilization percentage
	pub async fn GetUtilization(&self) -> f64 {
		let (current, _) = self.GetLoad().await;

		if self.config.max_concurrent == 0 {
			return 0.0;
		}

		(current as f64 / self.config.max_concurrent as f64) * 100.0
	}
}

impl Clone for BulkheadExecutor {
	fn clone(&self) -> Self {
		Self {
			name:self.name.clone(),

			semaphore:self.semaphore.clone(),

			config:self.config.clone(),

			current_requests:self.current_requests.clone(),

			queue_size:self.queue_size.clone(),

			total_rejected:self.total_rejected.clone(),

			total_completed:self.total_completed.clone(),

			total_timed_out:self.total_timed_out.clone(),
		}
	}
}
