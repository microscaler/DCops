//! Mock Event Recorder for testing event emission
//!
//! This module provides a mock implementation of event recording that captures
//! events in memory for test assertions.

#[cfg(test)]
use kube::runtime::events::{Event, EventType};
#[cfg(test)]
use k8s_openapi::api::core::v1::ObjectReference;
#[cfg(test)]
use std::sync::{Arc, Mutex};
#[cfg(test)]
use std::collections::VecDeque;

/// A captured event for testing
#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub struct CapturedEvent {
    pub event_type: EventType,
    pub reason: String,
    pub message: Option<String>,
    pub object_ref: ObjectReference,
}

/// Mock event recorder that captures events in memory
#[cfg(test)]
#[derive(Clone)]
pub struct MockEventRecorder {
    events: Arc<Mutex<VecDeque<CapturedEvent>>>,
}

#[cfg(test)]
impl MockEventRecorder {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Get all captured events
    pub fn get_events(&self) -> Vec<CapturedEvent> {
        let events = self.events.lock().unwrap();
        events.iter().cloned().collect()
    }

    /// Get the last captured event
    pub fn get_last_event(&self) -> Option<CapturedEvent> {
        let events = self.events.lock().unwrap();
        events.back().cloned()
    }

    /// Clear all captured events
    pub fn clear(&self) {
        let mut events = self.events.lock().unwrap();
        events.clear();
    }

    /// Count events by reason
    pub fn count_by_reason(&self, reason: &str) -> usize {
        let events = self.events.lock().unwrap();
        events.iter().filter(|e| e.reason == reason).count()
    }

    /// Count events by type
    pub fn count_by_type(&self, event_type: EventType) -> usize {
        let events = self.events.lock().unwrap();
        events.iter().filter(|e| e.event_type == event_type).count()
    }

    /// Find events by reason
    pub fn find_by_reason(&self, reason: &str) -> Vec<CapturedEvent> {
        let events = self.events.lock().unwrap();
        events.iter()
            .filter(|e| e.reason == reason)
            .cloned()
            .collect()
    }

    /// Record an event (internal method)
    pub(crate) async fn record(&self, event: &Event, obj_ref: &ObjectReference) -> Result<(), kube::Error> {
        let captured = CapturedEvent {
            event_type: event.type_.clone(),
            reason: event.reason.clone(),
            message: event.note.clone(),
            object_ref: obj_ref.clone(),
        };
        let mut events = self.events.lock().unwrap();
        events.push_back(captured);
        Ok(())
    }
}

#[cfg(test)]
impl Default for MockEventRecorder {
    fn default() -> Self {
        Self::new()
    }
}

