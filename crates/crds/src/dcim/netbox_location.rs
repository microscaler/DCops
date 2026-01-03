//! NetBoxLocation Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox locations (nested locations within sites).

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::references::NetBoxResourceReference;

/// NetBoxLocationSpec defines the desired state of a NetBox location
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxLocation",
    namespaced,
    status = "NetBoxLocationStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxLocationSpec {
    /// Location name
    pub name: String,
    
    /// Location slug (optional, auto-generated from name if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    
    /// Site reference (references NetBoxSite CRD, required)
    pub site: NetBoxResourceReference,
    
    /// Tenant reference (references NetBoxTenant CRD, required)
    /// Tenant is required in NetBox for proper resource organization and access control
    pub tenant: NetBoxResourceReference,
    
    /// Parent location reference (references NetBoxLocation CRD for nested locations)
    /// If not provided, this is a top-level location (parent will be null in NetBox)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<NetBoxResourceReference>,
    
    /// Facility identifier (optional but recommended for data center locations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facility: Option<String>,
    
    /// Description of the location
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Comments (optional)
    /// Additional notes or documentation about this location
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
    
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

/// NetBoxLocationStatus defines the observed state of a NetBox location
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxLocationStatus {
    /// NetBox location ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox location URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the location
    pub state: crate::tenancy::netbox_tenant::ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

