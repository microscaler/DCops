//! NetBoxSiteGroup Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox site groups (alternative to regions for site organization).

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::references::NetBoxResourceReference;

/// NetBoxSiteGroupSpec defines the desired state of a NetBox site group
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxSiteGroup",
    namespaced,
    status = "NetBoxSiteGroupStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxSiteGroupSpec {
    /// Site group name
    pub name: String,
    
    /// Site group slug (optional, auto-generated from name if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    
    /// Parent site group reference (references NetBoxSiteGroup CRD for hierarchical organization)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<NetBoxResourceReference>,
    
    /// Description of the site group
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// NetBoxSiteGroupStatus defines the observed state of a NetBox site group
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxSiteGroupStatus {
    /// NetBox site group ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox site group URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the site group
    pub state: crate::tenancy::netbox_tenant::ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

