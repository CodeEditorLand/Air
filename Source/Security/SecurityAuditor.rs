use std::sync::Arc;

use tokio::sync::RwLock;

use crate::dev_log;

use super::SecurityEvent::Struct as SecurityEvent;
use super::SecurityEventType::SecurityEventType;
use super::SecuritySeverity::SecuritySeverity;

/// Security auditor for logging security events
pub struct Struct {
	/// Event history
	events:Arc<RwLock<Vec<SecurityEvent>>>,

	/// Event retention count
	retention:usize,
}

impl Struct {
	/// Create a new security auditor
	pub fn new(retention:usize) -> Self { Self { events:Arc::new(RwLock::new(Vec::new())), retention } }

	/// Log a security event
	pub async fn LogEvent(&self, event:SecurityEvent) {
		let mut events = self.events.write().await;

		events.push(event.clone());

		// Trim to retention limit
		if events.len() > self.retention {
			events.remove(0);
		}

		// Log to system logger
		dev_log!(
			"security",
			"{:?}: {} - {}",
			event.EventType,
			event.Details,
			event.SourceIp.as_deref().unwrap_or("N/A")
		);

		// In production, forward to Mountain monitoring
	}

	/// Get event history
	pub async fn GetEvents(&self, event_type:Option<SecurityEventType>, limit:Option<usize>) -> Vec<SecurityEvent> {
		let events = self.events.read().await;

		let mut filtered:Vec<SecurityEvent> = if let Some(evt_type) = event_type {
			events.iter().filter(|e| e.EventType == evt_type).cloned().collect()
		} else {
			events.clone()
		};

		// Reverse to get most recent first
		filtered.reverse();

		// Apply limit
		if let Some(limit) = limit {
			filtered.truncate(limit);
		}

		filtered
	}

	/// Get recent critical events
	pub async fn GetCriticalEvents(&self, limit:usize) -> Vec<SecurityEvent> {
		self.GetEvents(None, Some(limit))
			.await
			.into_iter()
			.filter(|e| e.Severity == SecuritySeverity::Critical)
			.collect()
	}
}

impl Clone for Struct {
	fn clone(&self) -> Self { Self { events:self.events.clone(), retention:self.retention } }
}
