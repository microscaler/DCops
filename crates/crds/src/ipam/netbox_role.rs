//! NetBoxRole Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox IPAM roles.

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// NetBoxRoleSpec defines the desired state of a NetBox role
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxRole",
    namespaced,
    status = "NetBoxRoleStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxRoleSpec {
    /// Role name
    pub name: String,
    
    /// Role slug (optional, auto-generated from name if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    
    /// Description of the role
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Weight (for ordering)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<u16>,
    
    /// Comments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
}

/// NetBoxRoleStatus defines the observed state of a NetBox role
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxRoleStatus {
    /// NetBox role ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox role URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the role
    pub state: crate::tenancy::netbox_tenant::ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

