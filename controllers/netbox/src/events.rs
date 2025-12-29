//! Kubernetes Events support for NetBox Controller
//!
//! This module provides event recording capabilities for the reconciler,
//! allowing SREs to inspect reconciliation events via `kubectl get events`.

use kube::runtime::events::{Event, EventType, Recorder};
use kube::Resource;
use tracing::warn;
use k8s_openapi::api::core::v1::ObjectReference;

/// Standard event reasons for NetBox Controller
pub mod reasons {
    /// Resource was successfully created in NetBox
    pub const CREATED: &str = "Created";
    
    /// Resource was successfully updated in NetBox
    pub const UPDATED: &str = "Updated";
    
    /// Resource was successfully deleted in NetBox
    pub const DELETED: &str = "Deleted";
    
    /// Reconciliation failed with an error
    pub const RECONCILIATION_FAILED: &str = "ReconciliationFailed";
    
    /// A required dependency (tenant, site, etc.) was not found
    pub const DEPENDENCY_NOT_FOUND: &str = "DependencyNotFound";
    
    /// Drift detected between CRD spec and NetBox state
    pub const DRIFT_DETECTED: &str = "DriftDetected";
    
    /// Token resolution failed for a tenant
    pub const TOKEN_RESOLUTION_FAILED: &str = "TokenResolutionFailed";
    
    /// A failed reconciliation is being retried
    pub const RETRY_ATTEMPT: &str = "RetryAttempt";
    
    /// Startup reconciliation mapped an existing NetBox resource
    pub const STARTUP_MAPPED: &str = "StartupMapped";
}

/// Trait for event recording (allows mocking in tests)
/// This trait abstracts event recording to enable testing with mocks
#[async_trait::async_trait]
pub trait EventRecorderTrait: Send + Sync {
    async fn publish(&self, event: &Event, obj_ref: &ObjectReference) -> Result<(), kube::Error>;
}

/// Wrapper for real Recorder to implement EventRecorderTrait
pub struct RecorderWrapper {
    recorder: Recorder,
}

impl RecorderWrapper {
    pub fn new(recorder: Recorder) -> Self {
        Self { recorder }
    }
}

#[async_trait::async_trait]
impl EventRecorderTrait for RecorderWrapper {
    async fn publish(&self, event: &Event, obj_ref: &ObjectReference) -> Result<(), kube::Error> {
        self.recorder.publish(event, obj_ref).await
    }
}

#[cfg(test)]
#[async_trait::async_trait]
impl EventRecorderTrait for crate::test_utils::mock_event_recorder::MockEventRecorder {
    async fn publish(&self, event: &Event, obj_ref: &ObjectReference) -> Result<(), kube::Error> {
        self.record(event, obj_ref).await
    }
}

/// Extension trait for EventRecorder to simplify event recording
#[async_trait::async_trait]
pub trait EventRecorderExt {
    /// Record a Normal event for a resource
    async fn record_normal<K: Resource + Send + Sync>(&self, reason: &str, message: &str, obj: &K)
    where
        K::DynamicType: Default;
    
    /// Record a Warning event for a resource
    async fn record_warning<K: Resource + Send + Sync>(&self, reason: &str, message: &str, obj: &K)
    where
        K::DynamicType: Default;
}

#[async_trait::async_trait]
impl EventRecorderExt for Recorder {
    /// Record a Normal event (successful operations)
    async fn record_normal<K: Resource + Send + Sync>(&self, reason: &str, message: &str, obj: &K) 
    where
        K::DynamicType: Default,
    {
        let event = Event {
            type_: EventType::Normal,
            reason: reason.to_string(),
            note: Some(message.to_string()),
            action: String::new(),
            secondary: None,
        };
        
        // Create ObjectReference from the resource
        let dynamic_type = K::DynamicType::default();
        let obj_ref = ObjectReference {
            kind: Some(K::kind(&dynamic_type).to_string()),
            namespace: obj.meta().namespace.clone(),
            name: obj.meta().name.clone(),
            uid: obj.meta().uid.clone(),
            api_version: Some(K::api_version(&dynamic_type).to_string()),
            resource_version: obj.meta().resource_version.clone(),
            field_path: None,
        };
        
        if let Err(e) = self.publish(&event, &obj_ref).await {
            warn!("Failed to record Normal event (reason: {}, message: {}): {}", reason, message, e);
            // Don't fail reconciliation on event recording failure
        }
    }
    
    /// Record a Warning event (errors, failures)
    async fn record_warning<K: Resource + Send + Sync>(&self, reason: &str, message: &str, obj: &K)
    where
        K::DynamicType: Default,
    {
        let event = Event {
            type_: EventType::Warning,
            reason: reason.to_string(),
            note: Some(message.to_string()),
            action: String::new(),
            secondary: None,
        };
        
        // Create ObjectReference from the resource
        let dynamic_type = K::DynamicType::default();
        let obj_ref = ObjectReference {
            kind: Some(K::kind(&dynamic_type).to_string()),
            namespace: obj.meta().namespace.clone(),
            name: obj.meta().name.clone(),
            uid: obj.meta().uid.clone(),
            api_version: Some(K::api_version(&dynamic_type).to_string()),
            resource_version: obj.meta().resource_version.clone(),
            field_path: None,
        };
        
        if let Err(e) = self.publish(&event, &obj_ref).await {
            warn!("Failed to record Warning event (reason: {}, message: {}): {}", reason, message, e);
            // Don't fail reconciliation on event recording failure
        }
    }
}

#[cfg(test)]
#[async_trait::async_trait]
impl EventRecorderExt for crate::test_utils::mock_event_recorder::MockEventRecorder {
    /// Record a Normal event (successful operations)
    async fn record_normal<K: Resource + Send + Sync>(&self, reason: &str, message: &str, obj: &K) 
    where
        K::DynamicType: Default,
    {
        let event = Event {
            type_: EventType::Normal,
            reason: reason.to_string(),
            note: Some(message.to_string()),
            action: String::new(),
            secondary: None,
        };
        
        // Create ObjectReference from the resource
        let dynamic_type = K::DynamicType::default();
        let obj_ref = ObjectReference {
            kind: Some(K::kind(&dynamic_type).to_string()),
            namespace: obj.meta().namespace.clone(),
            name: obj.meta().name.clone(),
            uid: obj.meta().uid.clone(),
            api_version: Some(K::api_version(&dynamic_type).to_string()),
            resource_version: obj.meta().resource_version.clone(),
            field_path: None,
        };
        
        // Use the trait's publish method
        if let Err(e) = <Self as EventRecorderTrait>::publish(self, &event, &obj_ref).await {
            warn!("Failed to record Normal event (reason: {}, message: {}): {}", reason, message, e);
        }
    }
    
    /// Record a Warning event (errors, failures)
    async fn record_warning<K: Resource + Send + Sync>(&self, reason: &str, message: &str, obj: &K)
    where
        K::DynamicType: Default,
    {
        let event = Event {
            type_: EventType::Warning,
            reason: reason.to_string(),
            note: Some(message.to_string()),
            action: String::new(),
            secondary: None,
        };
        
        // Create ObjectReference from the resource
        let dynamic_type = K::DynamicType::default();
        let obj_ref = ObjectReference {
            kind: Some(K::kind(&dynamic_type).to_string()),
            namespace: obj.meta().namespace.clone(),
            name: obj.meta().name.clone(),
            uid: obj.meta().uid.clone(),
            api_version: Some(K::api_version(&dynamic_type).to_string()),
            resource_version: obj.meta().resource_version.clone(),
            field_path: None,
        };
        
        // Use the trait's publish method
        if let Err(e) = <Self as EventRecorderTrait>::publish(self, &event, &obj_ref).await {
            warn!("Failed to record Warning event (reason: {}, message: {}): {}", reason, message, e);
        }
    }
}

/// Helper function to record normal events via trait object
pub(crate) async fn record_event_normal_helper<K: Resource>(
    recorder: &dyn EventRecorderTrait,
    reason: &str,
    message: &str,
    obj: &K,
) where
    K::DynamicType: Default,
{
    let event = Event {
        type_: EventType::Normal,
        reason: reason.to_string(),
        note: Some(message.to_string()),
        action: String::new(),
        secondary: None,
    };
    
    let dynamic_type = K::DynamicType::default();
    let obj_ref = ObjectReference {
        kind: Some(K::kind(&dynamic_type).to_string()),
        namespace: obj.meta().namespace.clone(),
        name: obj.meta().name.clone(),
        uid: obj.meta().uid.clone(),
        api_version: Some(K::api_version(&dynamic_type).to_string()),
        resource_version: obj.meta().resource_version.clone(),
        field_path: None,
    };
    
    if let Err(e) = recorder.publish(&event, &obj_ref).await {
        warn!("Failed to record Normal event (reason: {}, message: {}): {}", reason, message, e);
    }
}

/// Helper function to record warning events via trait object
pub(crate) async fn record_event_warning_helper<K: Resource>(
    recorder: &dyn EventRecorderTrait,
    reason: &str,
    message: &str,
    obj: &K,
) where
    K::DynamicType: Default,
{
    let event = Event {
        type_: EventType::Warning,
        reason: reason.to_string(),
        note: Some(message.to_string()),
        action: String::new(),
        secondary: None,
    };
    
    let dynamic_type = K::DynamicType::default();
    let obj_ref = ObjectReference {
        kind: Some(K::kind(&dynamic_type).to_string()),
        namespace: obj.meta().namespace.clone(),
        name: obj.meta().name.clone(),
        uid: obj.meta().uid.clone(),
        api_version: Some(K::api_version(&dynamic_type).to_string()),
        resource_version: obj.meta().resource_version.clone(),
        field_path: None,
    };
    
    if let Err(e) = recorder.publish(&event, &obj_ref).await {
        warn!("Failed to record Warning event (reason: {}, message: {}): {}", reason, message, e);
    }
}

