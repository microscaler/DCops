//! NetBoxMACAddress Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox MAC addresses (DCIM).
//! Critical for PXE boot - identifies devices by MAC address.

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// NetBoxMACAddressSpec defines the desired state of a NetBox MAC address
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxMACAddress",
    namespaced,
    status = "NetBoxMACAddressStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxMACAddressSpec {
    /// MAC address (format: "aa:bb:cc:dd:ee:ff" or "aa-bb-cc-dd-ee-ff")
    pub mac_address: String,
    
    /// Interface name (references NetBoxInterface CRD)
    /// Format: "<device-name>/<interface-name>"
    pub interface: String,
    
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Comments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
}

/// NetBoxMACAddressStatus defines the observed state of a NetBox MAC address
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxMACAddressStatus {
    /// NetBox MAC address ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox MAC address URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the MAC address
    pub state: crate::tenancy::netbox_tenant::ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

