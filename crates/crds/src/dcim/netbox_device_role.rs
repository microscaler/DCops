//! NetBoxDeviceRole Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox device roles (DCIM).

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// NetBoxDeviceRoleSpec defines the desired state of a NetBox device role
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxDeviceRole",
    namespaced,
    status = "NetBoxDeviceRoleStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxDeviceRoleSpec {
    /// Device role name
    pub name: String,
    
    /// Device role slug (optional, auto-generated from name if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    
    /// Color (hex code, e.g., "9e9e9e")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    
    /// Virtual machine role (if true, can be assigned to VMs)
    #[serde(default = "default_vm_role")]
    pub vm_role: bool,
    
    /// Description of the device role
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Comments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
}

fn default_vm_role() -> bool {
    false
}

/// NetBoxDeviceRoleStatus defines the observed state of a NetBox device role
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxDeviceRoleStatus {
    /// NetBox device role ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox device role URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the device role
    pub state: crate::tenancy::netbox_tenant::ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

