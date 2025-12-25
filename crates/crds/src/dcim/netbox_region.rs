//! NetBoxRegion Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox regions (hierarchical site organization).

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::references::NetBoxResourceReference;

/// NetBoxRegionSpec defines the desired state of a NetBox region
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxRegion",
    namespaced,
    status = "NetBoxRegionStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxRegionSpec {
    /// Region name
    pub name: String,
    
    /// Region slug (optional, auto-generated from name if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    
    /// Parent region reference (references NetBoxRegion CRD for hierarchical organization)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<NetBoxResourceReference>,
    
    /// Description of the region
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// NetBoxRegionStatus defines the observed state of a NetBox region
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxRegionStatus {
    /// NetBox region ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox region URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the region
    pub state: crate::tenancy::netbox_tenant::ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

