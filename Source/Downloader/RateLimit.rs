//! Token-bucket rate limiter for per-download bandwidth throttling.
//!
//! Tokens represent bytes that can be consumed. They refill at `refill_rate`
//! bytes/second up to `capacity`. Downloads call `consume(bytes).await` which
//! parks the task until enough tokens are available, keeping the observed
//! throughput at or below the configured limit while still allowing short
//! bursts up to `capacity_factor` seconds' worth of data.

use std::time::{Duration, Instant};

use crate::Result;

/// Token-bucket rate limiter. Stores the bucket state; wrap in `Arc<RwLock<_>>`
/// to share across concurrent download tasks.
#[derive(Debug)]
pub struct TokenBucket {
	/// Available tokens (bytes).
	tokens:f64,

	/// Burst capacity (bytes).
	capacity:f64,

	/// Bytes-per-second refill rate.
	refill_rate:f64,

	/// Monotonic timestamp of the last refill.
	last_refill:Instant,
}

impl TokenBucket {
	/// Create a bucket with `bytes_per_sec` sustained throughput and
	/// a burst buffer of `capacity_factor` seconds' worth of tokens.
	pub fn new(bytes_per_sec:u64, capacity_factor:f64) -> Self {
		let refill_rate = bytes_per_sec as f64;

		let capacity = refill_rate * capacity_factor;

		Self { tokens:capacity, capacity, refill_rate, last_refill:Instant::now() }
	}

	/// Replenish tokens based on elapsed wall time. Call before every consume.
	pub fn refill(&mut self) {
		let elapsed = self.last_refill.elapsed().as_secs_f64();

		if elapsed > 0.0 {
			self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);

			self.last_refill = Instant::now();
		}
	}

	/// Consume up to `bytes` tokens immediately. Returns how many were
	/// consumed. Does not block - the caller decides what to do with remaining
	/// deficit.
	pub fn try_consume(&mut self, bytes:u64) -> u64 {
		self.refill();

		let bytes = bytes as f64;

		if self.tokens >= bytes {
			self.tokens -= bytes;

			bytes as u64
		} else {
			let available = self.tokens;

			self.tokens = 0.0;

			available as u64
		}
	}

	/// Async-wait until `bytes` tokens are available, then consume them.
	/// Polls at most every 100 ms so Tokio's timer wheel stays responsive.
	pub async fn consume(&mut self, bytes:u64) -> Result<()> {
		let bytes_needed = bytes as f64;

		loop {
			self.refill();

			if self.tokens >= bytes_needed {
				self.tokens -= bytes_needed;

				return Ok(());
			}

			let tokens_needed = bytes_needed - self.tokens;

			let wait_secs = (tokens_needed / self.refill_rate).min(0.1);

			tokio::time::sleep(Duration::from_secs_f64(wait_secs)).await;
		}
	}

	/// Adjust the sustained rate. Burst capacity is reset to 5× the new rate.
	pub fn set_rate(&mut self, bytes_per_sec:u64) {
		self.refill_rate = bytes_per_sec as f64;

		self.capacity = self.refill_rate * 5.0;
	}
}
