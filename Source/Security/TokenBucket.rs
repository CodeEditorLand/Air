/// Rate limit bucket for token bucket algorithm
#[derive(Debug, Clone)]
pub(crate) struct TokenBucket {
	pub(crate) tokens:f64,

	pub(crate) capacity:f64,

	pub(crate) refill_rate:f64,

	pub(crate) last_refill:std::time::Instant,
}

impl TokenBucket {
	pub(crate) fn new(capacity:f64, refill_rate:f64) -> Self {
		Self { tokens:capacity, capacity, refill_rate, last_refill:std::time::Instant::now() }
	}

	pub(crate) fn refill(&mut self) {
		let now = std::time::Instant::now();

		let elapsed = now.duration_since(self.last_refill).as_secs_f64();

		self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);

		self.last_refill = now;
	}

	pub(crate) fn try_consume(&mut self, tokens:f64) -> bool {
		self.refill();

		if self.tokens >= tokens {
			self.tokens -= tokens;

			true
		} else {
			false
		}
	}
}
