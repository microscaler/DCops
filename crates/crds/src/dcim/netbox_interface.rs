//! NetBoxInterface Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox interfaces (DCIM).

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// NetBoxInterfaceSpec defines the desired state of a NetBox interface
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxInterface",
    namespaced,
    status = "NetBoxInterfaceStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxInterfaceSpec {
    /// Device name (references NetBoxDevice CRD)
    pub device: String,
    
    /// Interface name (e.g., "eth0", "wlan0")
    pub name: String,
    
    /// Interface type (e.g., "1000base-t", "virtual", "other")
    #[serde(default = "default_interface_type")]
    pub r#type: String,
    
    /// Enabled (default: true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    
    /// MAC address (optional, can be managed via NetBoxMACAddress CRD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<String>,
    
    /// MTU (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtu: Option<u16>,
    
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_interface_type() -> String {
    "other".to_string()
}

fn default_enabled() -> bool {
    true
}

/// NetBoxInterfaceStatus defines the observed state of a NetBox interface
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxInterfaceStatus {
    /// NetBox interface ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox interface URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the interface
    pub state: crate::tenancy::netbox_tenant::ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

