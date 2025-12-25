//! NetBoxVLAN Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox VLANs (IPAM).

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::references::NetBoxResourceReference;

/// NetBoxVLANSpec defines the desired state of a NetBox VLAN
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxVLAN",
    namespaced,
    status = "NetBoxVLANStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxVLANSpec {
    /// VLAN ID (1-4094)
    pub vid: u16,
    
    /// VLAN name
    pub name: String,
    
    /// Site reference (references NetBoxSite CRD, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site: Option<NetBoxResourceReference>,
    
    /// VLAN group reference (references NetBoxVLANGroup CRD, optional - not yet implemented)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<NetBoxResourceReference>,
    
    /// Tenant reference (references NetBoxTenant CRD, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<NetBoxResourceReference>,
    
    /// Role reference (references NetBoxRole CRD, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<NetBoxResourceReference>,
    
    /// VLAN status (active, reserved, deprecated)
    #[serde(default = "default_vlan_status")]
    pub status: VlanStatus,
    
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Comments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
}

fn default_vlan_status() -> VlanStatus {
    VlanStatus::Active
}

/// VLAN status choices
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum VlanStatus {
    #[default]
    Active,
    Reserved,
    Deprecated,
}

/// NetBoxVLANStatus defines the observed state of a NetBox VLAN
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxVLANStatus {
    /// NetBox VLAN ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox VLAN URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the VLAN
    pub state: crate::tenancy::netbox_tenant::ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

