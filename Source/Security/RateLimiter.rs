use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio::sync::RwLock;

use crate::Result;

use super::RateLimitConfig::Struct as RateLimitConfig;
use super::RateLimitStatus::Struct as RateLimitStatus;
use super::TokenBucket::TokenBucket;

/// Rate limiter with per-IP and per-client tracking
pub struct Struct {
	config:RateLimitConfig,

	ip_buckets:Arc<RwLock<HashMap<String, TokenBucket>>>,

	client_buckets:Arc<RwLock<HashMap<String, TokenBucket>>>,

	cleanup_interval:Duration,
}

impl Struct {
	/// Create a new rate limiter
	pub fn New(config:RateLimitConfig) -> Self {
		let cleanup_interval = Duration::from_secs(300); // 5 minutes

		Self {
			config,

			ip_buckets:Arc::new(RwLock::new(HashMap::new())),

			client_buckets:Arc::new(RwLock::new(HashMap::new())),

			cleanup_interval,
		}
	}

	/// Check if request from IP is allowed
	pub async fn CheckIpRateLimit(&self, ip:&str) -> Result<bool> {
		let mut buckets = self.ip_buckets.write().await;

		let refill_rate = self.config.requests_per_second_ip as f64;

		let bucket = buckets
			.entry(ip.to_string())
			.or_insert_with(|| TokenBucket::new(self.config.burst_capacity as f64, refill_rate));

		Ok(bucket.try_consume(1.0))
	}

	/// Check if request from client is allowed
	pub async fn CheckClientRateLimit(&self, client_id:&str) -> Result<bool> {
		let mut buckets = self.client_buckets.write().await;

		let refill_rate = self.config.requests_per_second_client as f64;

		let bucket = buckets
			.entry(client_id.to_string())
			.or_insert_with(|| TokenBucket::new(self.config.burst_capacity as f64, refill_rate));

		Ok(bucket.try_consume(1.0))
	}

	/// Check both IP and client rate limits
	pub async fn CheckRateLimit(&self, ip:&str, client_id:&str) -> Result<bool> {
		let ip_allowed = self.CheckIpRateLimit(ip).await?;

		let client_allowed = self.CheckClientRateLimit(client_id).await?;

		Ok(ip_allowed && client_allowed)
	}

	/// Get current rate limit status for IP
	pub async fn GetIpStatus(&self, ip:&str) -> RateLimitStatus {
		let buckets = self.ip_buckets.read().await;

		if let Some(bucket) = buckets.get(ip) {
			RateLimitStatus {
				remaining_tokens:bucket.tokens as u32,

				capacity:bucket.capacity as u32,

				refill_rate:bucket.refill_rate as u32,
			}
		} else {
			RateLimitStatus {
				remaining_tokens:self.config.burst_capacity,

				capacity:self.config.burst_capacity,

				refill_rate:self.config.requests_per_second_ip,
			}
		}
	}

	/// Get current rate limit status for client
	pub async fn GetClientStatus(&self, client_id:&str) -> RateLimitStatus {
		let buckets = self.client_buckets.read().await;

		if let Some(bucket) = buckets.get(client_id) {
			RateLimitStatus {
				remaining_tokens:bucket.tokens as u32,

				capacity:bucket.capacity as u32,

				refill_rate:bucket.refill_rate as u32,
			}
		} else {
			RateLimitStatus {
				remaining_tokens:self.config.burst_capacity,

				capacity:self.config.burst_capacity,

				refill_rate:self.config.requests_per_second_client,
			}
		}
	}

	/// Clean up old buckets
	pub async fn CleanupStaleBuckets(&self) {
		let now = std::time::Instant::now();

		let mut ip_buckets = self.ip_buckets.write().await;

		ip_buckets.retain(|_, bucket| now.duration_since(bucket.last_refill) < self.cleanup_interval);

		let mut client_buckets = self.client_buckets.write().await;

		client_buckets.retain(|_, bucket| now.duration_since(bucket.last_refill) < self.cleanup_interval);

		// Cleanup completed - stale buckets removed
	}

	/// Start background cleanup task
	pub fn StartCleanupTask(&self) -> tokio::task::JoinHandle<()> {
		let ip_buckets = self.ip_buckets.clone();

		let client_buckets = self.client_buckets.clone();

		let cleanup_interval = self.cleanup_interval;

		tokio::spawn(async move {
			let mut interval = tokio::time::interval(cleanup_interval);

			loop {
				interval.tick().await;

				let now = std::time::Instant::now();

				let mut buckets = ip_buckets.write().await;

				buckets.retain(|_, bucket| now.duration_since(bucket.last_refill) < cleanup_interval);

				let mut buckets = client_buckets.write().await;

				buckets.retain(|_, bucket| now.duration_since(bucket.last_refill) < cleanup_interval);
			}
		})
	}
}

impl Clone for Struct {
	fn clone(&self) -> Self {
		Self {
			config:self.config.clone(),

			ip_buckets:self.ip_buckets.clone(),

			client_buckets:self.client_buckets.clone(),

			cleanup_interval:self.cleanup_interval,
		}
	}
}
