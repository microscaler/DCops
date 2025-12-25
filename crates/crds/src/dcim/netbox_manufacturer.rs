//! NetBoxManufacturer Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox manufacturers (DCIM).

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// NetBoxManufacturerSpec defines the desired state of a NetBox manufacturer
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxManufacturer",
    namespaced,
    status = "NetBoxManufacturerStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxManufacturerSpec {
    /// Manufacturer name
    pub name: String,
    
    /// Manufacturer slug (optional, auto-generated from name if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    
    /// Description of the manufacturer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// NetBoxManufacturerStatus defines the observed state of a NetBox manufacturer
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxManufacturerStatus {
    /// NetBox manufacturer ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox manufacturer URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the manufacturer
    pub state: crate::tenancy::netbox_tenant::ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

