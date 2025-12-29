//! Test helper functions for event emission testing
//!
//! This module provides helper functions to simplify writing event tests
//! and asserting that events are emitted with correct content.

#[cfg(test)]
use crate::test_utils::mock_event_recorder::{CapturedEvent, MockEventRecorder};
#[cfg(test)]
use kube::runtime::events::EventType;
#[cfg(test)]
use kube::Resource;

/// Assert that an event was emitted with the given reason
#[cfg(test)]
pub fn assert_event_emitted(
    recorder: &MockEventRecorder,
    reason: &str,
) -> Result<CapturedEvent, String> {
    let events = recorder.find_by_reason(reason);
    if events.is_empty() {
        let all_events: Vec<String> = recorder
            .get_events()
            .iter()
            .map(|e| format!("{:?}", e))
            .collect();
        return Err(format!(
            "Expected event with reason '{}' but none found. All events: {:?}",
            reason, all_events
        ));
    }
    Ok(events[0].clone())
}

/// Assert that an event was emitted with the given reason and type
#[cfg(test)]
pub fn assert_event_emitted_with_type(
    recorder: &MockEventRecorder,
    reason: &str,
    event_type: EventType,
) -> Result<CapturedEvent, String> {
    let event = assert_event_emitted(recorder, reason)?;
    if event.event_type != event_type {
        return Err(format!(
            "Expected event type {:?} but got {:?} for reason '{}'",
            event_type, event.event_type, reason
        ));
    }
    Ok(event)
}

/// Assert that a Normal event was emitted
#[cfg(test)]
pub fn assert_normal_event_emitted(
    recorder: &MockEventRecorder,
    reason: &str,
) -> Result<CapturedEvent, String> {
    assert_event_emitted_with_type(recorder, reason, EventType::Normal)
}

/// Assert that a Warning event was emitted
#[cfg(test)]
pub fn assert_warning_event_emitted(
    recorder: &MockEventRecorder,
    reason: &str,
) -> Result<CapturedEvent, String> {
    assert_event_emitted_with_type(recorder, reason, EventType::Warning)
}

/// Assert that an event message contains the given text
#[cfg(test)]
pub fn assert_event_message_contains(
    event: &CapturedEvent,
    text: &str,
) -> Result<(), String> {
    match &event.message {
        Some(msg) if msg.contains(text) => Ok(()),
        Some(msg) => Err(format!(
            "Expected event message to contain '{}' but got '{}'",
            text, msg
        )),
        None => Err(format!(
            "Expected event message to contain '{}' but message was None",
            text
        )),
    }
}

/// Assert that an event was emitted for the given resource
#[cfg(test)]
pub fn assert_event_for_resource<K: Resource>(
    event: &CapturedEvent,
    resource: &K,
) -> Result<(), String>
where
    K::DynamicType: Default,
{
    let dynamic_type = K::DynamicType::default();
    let expected_kind = K::kind(&dynamic_type).to_string();
    let expected_name = resource.meta().name.as_deref().unwrap_or("<unnamed>");
    let expected_namespace = resource.meta().namespace.as_deref().unwrap_or("default");

    if let Some(ref kind) = event.object_ref.kind {
        if kind != &expected_kind {
            return Err(format!(
                "Expected event kind '{}' but got '{}'",
                expected_kind, kind
            ));
        }
    } else {
        return Err("Event object_ref.kind is None".to_string());
    }

    if let Some(ref name) = event.object_ref.name {
        if name != expected_name {
            return Err(format!(
                "Expected event name '{}' but got '{}'",
                expected_name, name
            ));
        }
    } else {
        return Err("Event object_ref.name is None".to_string());
    }

    if event.object_ref.namespace.as_deref() != Some(expected_namespace) {
        return Err(format!(
            "Expected event namespace '{}' but got '{:?}'",
            expected_namespace,
            event.object_ref.namespace
        ));
    }

    Ok(())
}

/// Assert that multiple events were emitted
#[cfg(test)]
pub fn assert_event_count(
    recorder: &MockEventRecorder,
    reason: &str,
    expected_count: usize,
) -> Result<(), String> {
    let count = recorder.count_by_reason(reason);
    if count != expected_count {
        return Err(format!(
            "Expected {} events with reason '{}' but found {}",
            expected_count, reason, count
        ));
    }
    Ok(())
}

/// Assert that at least one event was emitted
#[cfg(test)]
pub fn assert_at_least_one_event(recorder: &MockEventRecorder) -> Result<(), String> {
    let events = recorder.get_events();
    if events.is_empty() {
        return Err("Expected at least one event but none were emitted".to_string());
    }
    Ok(())
}

/// Assert that no events were emitted
#[cfg(test)]
pub fn assert_no_events(recorder: &MockEventRecorder) -> Result<(), String> {
    let events = recorder.get_events();
    if !events.is_empty() {
        let event_reasons: Vec<String> = events.iter().map(|e| e.reason.clone()).collect();
        return Err(format!(
            "Expected no events but found {}: {:?}",
            events.len(),
            event_reasons
        ));
    }
    Ok(())
}

