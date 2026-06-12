//! Circuit breaker statistics for metrics export.
//!
//! Snapshot of `CircuitBreaker` state and counters. `LastFailureTime` is
//! skipped during serialization since `Instant` is not serializable.

use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::CircuitState::CircuitState;

/// Circuit breaker statistics for metrics export
#[derive(Debug, Clone, Serialize)]
pub struct CircuitStatistics {
	pub Name:String,

	pub State:CircuitState,

	pub Failures:u32,

	pub Successes:u32,

	pub StateTransitions:u32,

	#[serde(skip_serializing)]
	pub LastFailureTime:Option<Instant>,
}

impl<'de> Deserialize<'de> for CircuitStatistics {
	fn deserialize<D>(Deserializer:D) -> std::result::Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>, {
		use serde::de::{self, Visitor};

		struct CircuitStatisticsVisitor;

		impl<'de> Visitor<'de> for CircuitStatisticsVisitor {
			type Value = CircuitStatistics;

			fn expecting(&self, formatter:&mut std::fmt::Formatter) -> std::fmt::Result {
				formatter.write_str("struct CircuitStatistics")
			}

			fn visit_map<A>(self, mut map:A) -> std::result::Result<CircuitStatistics, A::Error>
			where
				A: de::MapAccess<'de>, {
				let mut Name = None;

				let mut State = None;

				let mut Failures = None;

				let mut Successes = None;

				let mut StateTransitions = None;

				while let Some(key) = map.next_key::<String>()? {
					match key.as_str() {
						"name" => Name = Some(map.next_value()?),

						"state" => State = Some(map.next_value()?),

						"failures" => Failures = Some(map.next_value()?),

						"successes" => Successes = Some(map.next_value()?),

						"state_transitions" => StateTransitions = Some(map.next_value()?),

						_ => {
							map.next_value::<de::IgnoredAny>()?;
						},
					}
				}

				Ok(CircuitStatistics {
					Name:Name.ok_or_else(|| de::Error::missing_field("name"))?,

					State:State.ok_or_else(|| de::Error::missing_field("state"))?,

					Failures:Failures.ok_or_else(|| de::Error::missing_field("failures"))?,

					Successes:Successes.ok_or_else(|| de::Error::missing_field("successes"))?,

					StateTransitions:StateTransitions.ok_or_else(|| de::Error::missing_field("state_transitions"))?,

					LastFailureTime:None,
				})
			}
		}

		const FIELDS:&[&str] = &["name", "state", "failures", "successes", "state_transitions"];

		Deserializer.deserialize_struct("CircuitStatistics", FIELDS, CircuitStatisticsVisitor)
	}
}
