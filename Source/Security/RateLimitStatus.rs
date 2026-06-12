use serde::{Deserialize, Serialize};

/// Rate limit status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Struct {
	pub remaining_tokens:u32,

	pub capacity:u32,

	pub refill_rate:u32,
}
