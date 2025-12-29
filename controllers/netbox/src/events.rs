//! Kubernetes Events support for NetBox Controller
//!
//! This module provides event recording capabilities for the reconciler,
//! allowing SREs to inspect reconciliation events via `kubectl get events`.

use kube::runtime::events::{Event, EventType, Recorder};
use kube::Resource;
use tracing::warn;

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

/// Extension trait for EventRecorder to simplify event recording
pub trait EventRecorderExt {
    /// Record a Normal event for a resource
    async fn record_normal<K: Resource>(&self, reason: &str, message: &str, obj: &K)
    where
        K::DynamicType: Default;
    
    /// Record a Warning event for a resource
    async fn record_warning<K: Resource>(&self, reason: &str, message: &str, obj: &K)
    where
        K::DynamicType: Default;
}

impl EventRecorderExt for Recorder {
    /// Record a Normal event (successful operations)
    async fn record_normal<K: Resource>(&self, reason: &str, message: &str, obj: &K) 
    where
        K::DynamicType: Default,
    {
        use k8s_openapi::api::core::v1::ObjectReference;
        
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
    async fn record_warning<K: Resource>(&self, reason: &str, message: &str, obj: &K)
    where
        K::DynamicType: Default,
    {
        use k8s_openapi::api::core::v1::ObjectReference;
        
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

