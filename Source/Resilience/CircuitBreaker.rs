//! Circuit breaker for fault isolation with state consistency
//! validation and event publishing.
//!
//! Tracks failures/successes, transitions between `Closed` → `Open` →
//! `HalfOpen` → `Closed`, and publishes `CircuitEvent` on every
//! transition for telemetry subscribers.

use std::{
	sync::Arc,
	time::{Duration, Instant},
};

use tokio::sync::{RwLock, broadcast};

use crate::dev_log;
use super::{
	CircuitBreakerConfig::CircuitBreakerConfig,
	CircuitEvent::CircuitEvent,
	CircuitState::CircuitState,
	CircuitStatistics::CircuitStatistics,
};

/// Circuit breaker for fault isolation with state consistency validation and
/// event publishing
#[derive(Debug)]
pub struct CircuitBreaker {
	Name:String,

	State:Arc<RwLock<CircuitState>>,

	Config:CircuitBreakerConfig,

	FailureCount:Arc<RwLock<u32>>,

	SuccessCount:Arc<RwLock<u32>>,

	LastFailureTime:Arc<RwLock<Option<Instant>>>,

	EventTx:Arc<broadcast::Sender<CircuitEvent>>,

	StateTransitionCounter:Arc<RwLock<u32>>,
}

impl CircuitBreaker {
	/// Create a new circuit breaker with event publishing
	pub fn new(name:String, Config:CircuitBreakerConfig) -> Self {
		let (EventTx, _) = broadcast::channel(1000);

		Self {
			Name:name.clone(),

			State:Arc::new(RwLock::new(CircuitState::Closed)),

			Config,

			FailureCount:Arc::new(RwLock::new(0)),

			SuccessCount:Arc::new(RwLock::new(0)),

			LastFailureTime:Arc::new(RwLock::new(None)),

			EventTx:Arc::new(EventTx),

			StateTransitionCounter:Arc::new(RwLock::new(0)),
		}
	}

	/// Get the circuit breaker event transmitter for subscription
	pub fn GetEventTransmitter(&self) -> broadcast::Sender<CircuitEvent> { (*self.EventTx).clone() }

	/// Get current state with panic recovery
	pub async fn GetState(&self) -> CircuitState { *self.State.read().await }

	/// Validate state consistency across all counters
	pub async fn ValidateState(&self) -> Result<(), String> {
		let state = *self.State.read().await;

		let failures = *self.FailureCount.read().await;

		let successes = *self.SuccessCount.read().await;

		match state {
			CircuitState::Closed => {
				if successes != 0 {
					return Err(format!("Inconsistent state: Closed but has {} successes", successes));
				}

				if failures >= self.Config.FailureThreshold {
					dev_log!(
						"resilience",
						"warn: [CircuitBreaker] State inconsistency: Closed but failure count ({}) >= threshold ({})",
						failures,
						self.Config.FailureThreshold
					);
				}
			},

			CircuitState::Open => {
				if failures < self.Config.FailureThreshold {
					dev_log!(
						"resilience",
						"warn: [CircuitBreaker] State inconsistency: Open but failure count ({}) < threshold ({})",
						failures,
						self.Config.FailureThreshold
					);
				}
			},

			CircuitState::HalfOpen => {
				if successes >= self.Config.SuccessThreshold {
					return Err(format!(
						"Inconsistent state: HalfOpen but has {} successes (should be Closed)",
						successes
					));
				}
			},
		}

		Ok(())
	}

	/// Transition state with validation and event publishing
	async fn TransitionState(&self, NewState:CircuitState, reason:&str) -> Result<(), String> {
		let CurrentState = self.GetState().await;

		if CurrentState == NewState {
			// No transition needed
			return Ok(());
		}

		// Validate the proposed transition
		match (CurrentState, NewState) {
			(CircuitState::Closed, CircuitState::Open) | (CircuitState::HalfOpen, CircuitState::Open) => {

				// Valid transitions
			},

			(CircuitState::Open, CircuitState::HalfOpen) => {

				// Valid transition through recovery
			},

			(CircuitState::HalfOpen, CircuitState::Closed) => {

				// Valid recovery transition
			},

			_ => {
				return Err(format!(
					"Invalid state transition from {:?} to {:?} for {}",
					CurrentState, NewState, self.Name
				));
			},
		}

		// Publish state transition event
		let event = CircuitEvent {
			name:self.Name.clone(),

			FromState:CurrentState,

			ToState:NewState,

			timestamp:crate::Utility::CurrentTimestamp(),

			reason:reason.to_string(),
		};

		let _ = self.EventTx.send(event);

		// Transition state
		*self.State.write().await = NewState;

		// Increment transition counter
		*self.StateTransitionCounter.write().await += 1;

		dev_log!(
			"resilience",
			"[CircuitBreaker] State transition for {}: {:?} -> {:?} (reason: {})",
			self.Name,
			CurrentState,
			NewState,
			reason
		);

		// Validate new state consistency
		self.ValidateState().await.map_err(|e| {
			dev_log!(
				"resilience",
				"error: [CircuitBreaker] State validation failed after transition: {}",
				e
			);

			e
		})?;

		Ok(())
	}

	/// Record a successful call with panic recovery
	pub async fn RecordSuccess(&self) {
		let state = self.GetState().await;

		match state {
			CircuitState::Closed => {
				// Reset counters
				*self.FailureCount.write().await = 0;
			},

			CircuitState::HalfOpen => {
				// Increment success count
				let mut SuccessCount = self.SuccessCount.write().await;

				*SuccessCount += 1;

				if *SuccessCount >= self.Config.SuccessThreshold {
					// Close the circuit
					let _ = self.TransitionState(CircuitState::Closed, "Success threshold reached").await;

					*self.FailureCount.write().await = 0;

					*self.SuccessCount.write().await = 0;
				}
			},

			_ => {},
		}
	}

	/// Record a failed call with panic recovery
	pub async fn RecordFailure(&self) {
		let State = self.GetState().await;

		*self.LastFailureTime.write().await = Some(Instant::now());

		match State {
			CircuitState::Closed => {
				// Increment failure count
				let mut FailureCount = self.FailureCount.write().await;

				*FailureCount += 1;

				if *FailureCount >= self.Config.FailureThreshold {
					// Open the circuit
					let _ = self.TransitionState(CircuitState::Open, "Failure threshold reached").await;

					*self.SuccessCount.write().await = 0;
				}
			},

			CircuitState::HalfOpen => {
				// Return to open state
				let _ = self.TransitionState(CircuitState::Open, "Failure in half-open state").await;

				*self.SuccessCount.write().await = 0;
			},

			_ => {},
		}
	}

	/// Attempt to transition to half-open if timeout has elapsed with panic
	/// recovery
	pub async fn AttemptRecovery(&self) -> bool {
		let state = self.GetState().await;

		if state != CircuitState::Open {
			return state == CircuitState::HalfOpen;
		}

		if let Some(last_failure) = *self.LastFailureTime.read().await {
			if last_failure.elapsed() >= Duration::from_secs(self.Config.TimeoutSecs) {
				let _ = self.TransitionState(CircuitState::HalfOpen, "Recovery timeout elapsed").await;

				*self.SuccessCount.write().await = 0;

				return true;
			}
		}

		false
	}

	/// Get circuit breaker statistics for metrics
	pub async fn GetStatistics(&self) -> CircuitStatistics {
		CircuitStatistics {
			Name:self.Name.clone(),

			State:self.GetState().await,

			Failures:*self.FailureCount.read().await,

			Successes:*self.SuccessCount.read().await,

			StateTransitions:*self.StateTransitionCounter.read().await,

			LastFailureTime:*self.LastFailureTime.read().await,
		}
	}

	/// Validate circuit breaker configuration
	pub fn ValidateConfig(config:&CircuitBreakerConfig) -> Result<(), String> {
		if config.FailureThreshold == 0 {
			return Err("FailureThreshold must be greater than 0".to_string());
		}

		if config.SuccessThreshold == 0 {
			return Err("SuccessThreshold must be greater than 0".to_string());
		}

		if config.TimeoutSecs == 0 {
			return Err("TimeoutSecs must be greater than 0".to_string());
		}

		Ok(())
	}
}

impl Clone for CircuitBreaker {
	fn clone(&self) -> Self {
		Self {
			Name:self.Name.clone(),

			State:self.State.clone(),

			Config:self.Config.clone(),

			FailureCount:self.FailureCount.clone(),

			SuccessCount:self.SuccessCount.clone(),

			LastFailureTime:self.LastFailureTime.clone(),

			EventTx:self.EventTx.clone(),

			StateTransitionCounter:self.StateTransitionCounter.clone(),
		}
	}
}
