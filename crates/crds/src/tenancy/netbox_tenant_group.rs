//! NetBoxTenantGroup Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox tenant groups.

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::references::NetBoxResourceReference;

/// NetBoxTenantGroupSpec defines the desired state of a NetBox tenant group
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxTenantGroup",
    namespaced,
    status = "NetBoxTenantGroupStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxTenantGroupSpec {
    /// Tenant group name
    pub name: String,
    
    /// Tenant group slug (optional, auto-generated from name if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    
    /// Description of the tenant group
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Comments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
    
    /// Parent tenant group reference (references NetBoxTenantGroup CRD for hierarchical organization)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<NetBoxResourceReference>,
    
    /// Tag references (references NetBoxTag CRDs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<NetBoxResourceReference>>,
    
    /// Enable drift detection (default: true)
    /// 
    /// When enabled, the reconciler will detect and correct any changes made to the resource
    /// in the NetBox UI that differ from the CRD spec. Git is the source of truth.
    /// Set to false to disable drift detection (not recommended for GitOps workflows).
    #[serde(skip_serializing_if = "Option::is_none", default = "default_drift_detection")]
    pub drift_detection: Option<bool>,
}

fn default_drift_detection() -> Option<bool> {
    Some(true)
}

/// NetBoxTenantGroupStatus defines the observed state of a NetBox tenant group
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxTenantGroupStatus {
    /// NetBox ID of the tenant group
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox URL of the tenant group
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the tenant group
    pub state: crate::ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<String>,
}

