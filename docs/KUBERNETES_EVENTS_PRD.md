# Kubernetes Events PRD

**Product Requirements Document: Kubernetes Event Emission for NetBox Controller**

**Status:** Planned  
**Priority:** Medium  
**Target:** Post-80% Test Coverage  
**Created:** 2025-12-29  
**Owner:** NetBox Controller Team

---

## 1. Executive Summary

The NetBox Controller currently provides observability through CRD status fields and controller logs. However, it does not emit Kubernetes Events, which are a standard mechanism for controllers to communicate reconciliation state to SREs and operators. This PRD outlines the requirements for adding Kubernetes Event emission to improve observability and debugging capabilities.

---

## 2. Problem Statement

### Current State
- **Status Updates Only**: The controller updates CRD status fields (`status.state`, `status.error`) but does not emit Kubernetes Events
- **Limited Observability**: SREs must check CRD status or controller pod logs to understand reconciliation state
- **No Event Timeline**: `kubectl describe <resource>` does not show a timeline of reconciliation events
- **Harder Debugging**: Difficult to correlate issues across resources without event history

### Impact
- SREs cannot use standard Kubernetes tooling (`kubectl get events`) to monitor controller behavior
- No event timeline visible in `kubectl describe` output
- Reduced visibility into reconciliation lifecycle (create, update, error, retry cycles)
- Harder to troubleshoot issues without diving into controller logs

---

## 3. Goals and Objectives

### Primary Goals
1. **Improve Observability**: Enable SREs to inspect reconciliation events using standard Kubernetes tooling
2. **Standard Compliance**: Follow Kubernetes controller best practices by emitting Events
3. **Better Debugging**: Provide event timeline for troubleshooting reconciliation issues
4. **Non-Breaking**: Implementation must not change existing behavior or break current functionality

### Success Criteria
- ✅ Events are visible via `kubectl get events` in the CR's namespace
- ✅ Events appear in `kubectl describe <crd>` output
- ✅ Events are properly categorized (Normal, Warning)
- ✅ Event messages are clear and actionable
- ✅ No performance degradation from event emission
- ✅ All existing tests continue to pass
- ✅ New tests cover event emission scenarios

---

## 4. Requirements

### 4.1 Functional Requirements

#### FR1: Event Emission on Reconciliation Actions
The controller MUST emit Kubernetes Events for the following reconciliation actions:

| Action | Event Type | When Emitted |
|--------|------------|--------------|
| Resource Created | Normal | When a NetBox resource is successfully created |
| Resource Updated | Normal | When a NetBox resource is successfully updated |
| Resource Deleted | Normal | When a NetBox resource is successfully deleted |
| Reconciliation Error | Warning | When reconciliation fails (with retry information) |
| Dependency Not Found | Warning | When a required dependency (tenant, site, etc.) is missing |
| Drift Detected | Warning | When drift is detected between CRD spec and NetBox state |
| Token Resolution Failed | Warning | When token resolution fails for a tenant |
| Retry Attempt | Normal | When a failed reconciliation is retried (with backoff info) |
| Startup Reconciliation | Normal | When startup reconciliation maps existing NetBox resources |

#### FR2: Event Message Format
Event messages MUST follow this format:
```
<Action>: <Resource Type> <namespace/name> - <Details>
```

Examples:
- `Created: NetBoxPrefix default/test-prefix - Created prefix 192.168.1.0/24 in NetBox (ID: 100)`
- `Updated: NetBoxDevice default/web-server - Updated description from "Old" to "New"`
- `Warning: NetBoxTenant default/datacenter-tenant - Token resolution failed: Secret not found`
- `Retry: IPClaim default/app-ip - Retrying after error (attempt 2, backoff: 60s)`

#### FR3: Event Metadata
Events MUST include:
- **Reason**: Short, machine-readable reason code (e.g., `Created`, `Updated`, `ReconciliationFailed`, `DependencyNotFound`)
- **Type**: `Normal` for successful operations, `Warning` for errors
- **Message**: Human-readable description (see FR2)
- **Source**: Controller name (`netbox-controller`)
- **FirstTimestamp**: When the event first occurred
- **LastTimestamp**: When the event last occurred (for repeated events)
- **Count**: Number of times the event occurred (for repeated events)

#### FR4: Event Deduplication
The controller MUST leverage Kubernetes' built-in event deduplication:
- Events with the same `reason`, `type`, `source`, and `involvedObject` are automatically deduplicated
- Kubernetes increments `count` and updates `lastTimestamp` for duplicate events
- This prevents event spam while maintaining visibility

#### FR5: Namespace Scoping
Events MUST be emitted in the same namespace as the Custom Resource:
- Events are namespaced resources
- Events are automatically associated with the CR via `involvedObject`
- SREs can filter events: `kubectl get events -n <namespace>`

### 4.2 Non-Functional Requirements

#### NFR1: Performance
- Event emission MUST NOT add more than 50ms latency to reconciliation operations
- Event API calls MUST be non-blocking (fire-and-forget or async)
- Failed event emission MUST NOT block reconciliation (fail gracefully)

#### NFR2: Reliability
- Event emission failures MUST be logged but MUST NOT cause reconciliation to fail
- Event recorder MUST handle API rate limiting gracefully
- Event recorder MUST retry transient failures (with backoff)

#### NFR3: Testability
- Event emission MUST be mockable for unit tests
- Event recorder MUST be injectable via dependency injection
- Tests MUST verify event emission without requiring a real Kubernetes cluster

#### NFR4: Backward Compatibility
- Implementation MUST NOT change existing CRD status update behavior
- Implementation MUST NOT break existing tests
- Implementation MUST be opt-in (can be disabled via feature flag if needed)

---

## 5. Design

### 5.1 Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Reconciler                           │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Reconciliation Logic                            │  │
│  │  - reconcile_netbox_prefix()                    │  │
│  │  - reconcile_netbox_device()                    │  │
│  │  - reconcile_ip_claim()                          │  │
│  └──────────────┬──────────────────────────────────┘  │
│                 │                                       │
│                 ▼                                       │
│  ┌──────────────────────────────────────────────────┐  │
│  │  EventRecorder (kube-runtime)                    │  │
│  │  - record()                                       │  │
│  │  - publish_event()                                │  │
│  └──────────────┬──────────────────────────────────┘  │
│                 │                                       │
│                 ▼                                       │
│         Kubernetes API Server                           │
│         (Events API)                                    │
└─────────────────────────────────────────────────────────┘
```

### 5.2 Component Design

#### EventRecorder Integration
- Use `kube_runtime::events::Recorder` from `kube-runtime` crate
- Create `EventRecorder` in `Controller::new()` and pass to `Reconciler`
- Store as `Option<EventRecorder>` to allow disabling for tests

#### Reconciler Changes
- Add `event_recorder: Option<EventRecorder>` field to `Reconciler` struct
- Create helper methods:
  - `record_event_normal(&self, reason: &str, message: &str)`
  - `record_event_warning(&self, reason: &str, message: &str)`
- Emit events at key reconciliation points:
  - After successful create/update/delete
  - On reconciliation errors
  - On dependency resolution failures
  - On drift detection

#### Event Reasons (Standardized)
Define a set of standard event reasons:

```rust
pub mod event_reasons {
    pub const CREATED: &str = "Created";
    pub const UPDATED: &str = "Updated";
    pub const DELETED: &str = "Deleted";
    pub const RECONCILIATION_FAILED: &str = "ReconciliationFailed";
    pub const DEPENDENCY_NOT_FOUND: &str = "DependencyNotFound";
    pub const DRIFT_DETECTED: &str = "DriftDetected";
    pub const TOKEN_RESOLUTION_FAILED: &str = "TokenResolutionFailed";
    pub const RETRY_ATTEMPT: &str = "RetryAttempt";
    pub const STARTUP_MAPPED: &str = "StartupMapped";
}
```

### 5.3 Implementation Approach

#### Phase 1: Infrastructure Setup
1. Add `EventRecorder` to `Reconciler` struct
2. Create event recording helper methods
3. Add `EventRecorder` to `Controller::new()` initialization
4. Make event recorder optional (for tests)

#### Phase 2: Core Events
1. Emit events for successful operations (create, update, delete)
2. Emit events for reconciliation errors
3. Add tests for event emission

#### Phase 3: Advanced Events
1. Emit events for dependency resolution failures
2. Emit events for drift detection
3. Emit events for retry attempts
4. Emit events for startup reconciliation

#### Phase 4: Testing & Documentation
1. Add comprehensive tests for event emission
2. Mock `EventRecorder` in unit tests
3. Document event reasons and when they're emitted
4. Add examples to documentation

---

## 6. Technical Details

### 6.1 Dependencies

**Required:**
- `kube-runtime = "2.0"` (already in `Cargo.toml`)
- `kube::runtime::events::Recorder` (from `kube-runtime`)

**No new dependencies required** - `kube-runtime` already provides event recording capabilities.

### 6.2 Code Changes

#### New Files
- `controllers/netbox/src/events.rs` - Event recording helpers and reason constants
- `controllers/netbox/src/events_test.rs` - Tests for event emission

#### Modified Files
- `controllers/netbox/src/reconciler/mod.rs` - Add `EventRecorder` field and helper methods
- `controllers/netbox/src/controller.rs` - Initialize `EventRecorder` and pass to `Reconciler`
- `controllers/netbox/src/reconciler/*.rs` - Add event emission calls at key points
- `controllers/netbox/src/test_utils.rs` - Add mock `EventRecorder` for tests

### 6.3 Example Implementation

```rust
// controllers/netbox/src/events.rs
use kube::runtime::events::{Event, EventType, Recorder};
use kube::core::ObjectMeta;

pub mod reasons {
    pub const CREATED: &str = "Created";
    pub const UPDATED: &str = "Updated";
    pub const RECONCILIATION_FAILED: &str = "ReconciliationFailed";
    // ... more reasons
}

pub trait EventRecorderExt {
    async fn record_normal(&self, reason: &str, message: &str, obj: &dyn kube::Resource);
    async fn record_warning(&self, reason: &str, message: &str, obj: &dyn kube::Resource);
}

impl EventRecorderExt for Recorder {
    async fn record_normal(&self, reason: &str, message: &str, obj: &dyn kube::Resource) {
        self.publish(Event {
            type_: EventType::Normal,
            reason: reason.to_string(),
            note: Some(message.to_string()),
            action: None,
            secondary: None,
        }).await;
    }
    
    async fn record_warning(&self, reason: &str, message: &str, obj: &dyn kube::Resource) {
        // Similar implementation for Warning events
    }
}
```

```rust
// controllers/netbox/src/reconciler/mod.rs
impl Reconciler {
    async fn record_event_normal(&self, reason: &str, message: &str, obj: &dyn kube::Resource) {
        if let Some(recorder) = &self.event_recorder {
            if let Err(e) = recorder.record_normal(reason, message, obj).await {
                warn!("Failed to record event: {}", e);
                // Don't fail reconciliation on event recording failure
            }
        }
    }
    
    // Similar for record_event_warning
}
```

```rust
// Example usage in reconciler
pub async fn reconcile_netbox_prefix(&self, prefix_crd: &NetBoxPrefix) -> Result<(), ControllerError> {
    // ... reconciliation logic ...
    
    if created {
        self.record_event_normal(
            event_reasons::CREATED,
            &format!("Created prefix {} in NetBox (ID: {})", prefix_cidr, netbox_id),
            prefix_crd,
        ).await;
    } else if updated {
        self.record_event_normal(
            event_reasons::UPDATED,
            &format!("Updated prefix {} in NetBox", prefix_cidr),
            prefix_crd,
        ).await;
    }
    
    // ... rest of reconciliation ...
}
```

---

## 7. Testing Strategy

### 7.1 Unit Tests
- Mock `EventRecorder` to verify events are emitted with correct reason, type, and message
- Test that event emission failures don't block reconciliation
- Test event deduplication behavior

### 7.2 Integration Tests
- Verify events appear in Kubernetes cluster (requires test cluster)
- Verify events are associated with correct CR via `involvedObject`
- Verify events appear in `kubectl get events` output

### 7.3 Test Coverage Goals
- 100% coverage of event emission code paths
- All event reasons tested
- Error handling for event emission failures tested

---

## 8. Migration Plan

### 8.1 Rollout Strategy
1. **Phase 1**: Implement infrastructure (EventRecorder setup) - no events emitted yet
2. **Phase 2**: Emit events for successful operations only (low risk)
3. **Phase 3**: Emit events for errors and warnings
4. **Phase 4**: Emit events for advanced scenarios (drift, retries)

### 8.2 Rollback Plan
- Event emission can be disabled by setting `event_recorder: None` in `Reconciler::new()`
- No data migration required
- No breaking changes to CRDs or APIs

---

## 9. Success Metrics

### 9.1 Quantitative Metrics
- Event emission latency: < 50ms per reconciliation
- Event API call success rate: > 99.9%
- Test coverage: 100% of event emission code paths

### 9.2 Qualitative Metrics
- SRE feedback: Events are useful for debugging
- Reduced time to diagnose reconciliation issues
- Improved observability without requiring controller log access

---

## 10. Future Enhancements

### 10.1 Potential Improvements
- **Event Aggregation**: Aggregate similar events to reduce noise
- **Event Filtering**: Allow filtering events by type/reason
- **Custom Event Types**: Add custom event types for specific scenarios
- **Event Metrics**: Track event emission rates and types

### 10.2 Integration Opportunities
- **Prometheus Metrics**: Expose event emission metrics
- **Alerting**: Create alerts based on Warning events
- **Dashboards**: Visualize event timeline in monitoring dashboards

---

## 11. Dependencies and Prerequisites

### 11.1 Prerequisites
- ✅ `kube-runtime = "2.0"` already in dependencies
- ✅ Test coverage at confident level (80%+)
- ✅ All existing tests passing
- ✅ Stable reconciliation logic (no major refactoring in progress)

### 11.2 Blockers
- None identified - implementation can proceed once test coverage is sufficient

---

## 12. Open Questions

1. **Event Volume**: Should we limit event emission frequency to prevent API spam?
   - **Answer**: Kubernetes handles deduplication, but we should monitor event volume

2. **Event Retention**: Should we configure event retention policies?
   - **Answer**: Use default Kubernetes event retention (1 hour), can be configured at cluster level

3. **Event Filtering**: Should we allow filtering which events are emitted?
   - **Answer**: Start with all events, add filtering later if needed

---

## 13. References

- [Kubernetes Events Documentation](https://kubernetes.io/docs/reference/kubernetes-api/cluster-resources/event-v1/)
- [kube-runtime EventRecorder](https://docs.rs/kube-runtime/latest/kube_runtime/events/struct.Recorder.html)
- [Kubernetes Controller Best Practices](https://kubernetes.io/docs/concepts/architecture/controller/)

---

## 14. Approval and Sign-off

**Status**: Draft - Awaiting test coverage milestone  
**Next Review**: After 80% test coverage achieved  
**Owner**: NetBox Controller Team

---

**Document Version**: 1.0  
**Last Updated**: 2025-12-29

