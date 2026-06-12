//! Circuit breaker event type for telemetry and metrics integration.
//!
//! Published on every state transition so subscribers (e.g. Mountain
//! telemetry) can react to circuit changes.

use serde::{Deserialize, Serialize};

use super::CircuitState::CircuitState;

/// Circuit breaker events for metrics and telemetry integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitEvent {
	pub name:String,

	pub FromState:CircuitState,

	pub ToState:CircuitState,

	pub timestamp:u64,

	pub reason:String,
}
