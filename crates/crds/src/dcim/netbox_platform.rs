//! NetBoxPlatform Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox platforms (DCIM).

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::references::NetBoxResourceReference;

/// NetBoxPlatformSpec defines the desired state of a NetBox platform
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxPlatform",
    namespaced,
    status = "NetBoxPlatformStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxPlatformSpec {
    /// Platform name (e.g., "Talos Linux", "Ubuntu")
    pub name: String,
    
    /// Platform slug (optional, auto-generated from name if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    
    /// Manufacturer reference (references NetBoxManufacturer CRD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<NetBoxResourceReference>,
    
    /// NAPALM driver (for network automation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub napalm_driver: Option<String>,
    
    /// NAPALM arguments (JSON string)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub napalm_args: Option<String>,
    
    /// Description of the platform
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Comments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
}

/// NetBoxPlatformStatus defines the observed state of a NetBox platform
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxPlatformStatus {
    /// NetBox platform ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox platform URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the platform
    pub state: crate::tenancy::netbox_tenant::ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

