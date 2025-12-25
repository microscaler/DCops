//! NetBoxLocation Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox locations (nested locations within sites).

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::references::NetBoxResourceReference;

/// NetBoxLocationSpec defines the desired state of a NetBox location
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxLocation",
    namespaced,
    status = "NetBoxLocationStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxLocationSpec {
    /// Location name
    pub name: String,
    
    /// Location slug (optional, auto-generated from name if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    
    /// Site reference (references NetBoxSite CRD)
    pub site: NetBoxResourceReference,
    
    /// Parent location reference (references NetBoxLocation CRD for nested locations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<NetBoxResourceReference>,
    
    /// Description of the location
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// NetBoxLocationStatus defines the observed state of a NetBox location
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxLocationStatus {
    /// NetBox location ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox location URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the location
    pub state: crate::tenancy::netbox_tenant::ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

