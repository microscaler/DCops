//! NetBoxRIR Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox IPAM RIRs (Regional Internet Registries).

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::references::NetBoxResourceReference;

/// NetBoxRIRSpec defines the desired state of a NetBox RIR
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxRIR",
    namespaced,
    status = "NetBoxRIRStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxRIRSpec {
    /// RIR name (e.g., "ARIN", "RIPE", "APNIC")
    pub name: String,
    
    /// RIR slug (optional, auto-generated from name if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    
    /// Description of the RIR
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Whether this is a private RIR (default: false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_private: Option<bool>,
    
    /// Comments (optional)
    /// Additional notes or documentation about this RIR
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
    
    /// Tag references (references NetBoxTag CRDs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<NetBoxResourceReference>>,
}

/// NetBoxRIRStatus defines the observed state of a NetBox RIR
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxRIRStatus {
    /// NetBox RIR ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox RIR URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the RIR
    pub state: crate::tenancy::netbox_tenant::ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

