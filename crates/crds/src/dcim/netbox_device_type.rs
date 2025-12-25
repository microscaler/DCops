//! NetBoxDeviceType Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox device types (DCIM).

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::references::NetBoxResourceReference;

/// NetBoxDeviceTypeSpec defines the desired state of a NetBox device type
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxDeviceType",
    namespaced,
    status = "NetBoxDeviceTypeStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxDeviceTypeSpec {
    /// Manufacturer reference (references NetBoxManufacturer CRD)
    pub manufacturer: NetBoxResourceReference,
    
    /// Device model (e.g., "Raspberry Pi 4 Model B")
    pub model: String,
    
    /// Device type slug (optional, auto-generated from model if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    
    /// Part number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_number: Option<String>,
    
    /// U height (rack units, default: 1.0)
    #[serde(default = "default_u_height")]
    pub u_height: f64,
    
    /// Is full depth (default: false)
    #[serde(default = "default_is_full_depth")]
    pub is_full_depth: bool,
    
    /// Description of the device type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Comments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
}

fn default_u_height() -> f64 {
    1.0
}

fn default_is_full_depth() -> bool {
    false
}

/// NetBoxDeviceTypeStatus defines the observed state of a NetBox device type
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxDeviceTypeStatus {
    /// NetBox device type ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox device type URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the device type
    pub state: crate::tenancy::netbox_tenant::ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

