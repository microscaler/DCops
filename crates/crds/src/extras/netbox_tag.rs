//! NetBoxTag Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox tags.

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::references::NetBoxResourceReference;

/// NetBoxTagSpec defines the desired state of a NetBox tag
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxTag",
    namespaced,
    status = "NetBoxTagStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxTagSpec {
    /// Tag name
    pub name: String,
    
    /// Tag slug (optional, auto-generated from name if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    
    /// Tag color (hex color code, e.g., "9e9e9e")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    
    /// Description of the tag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Comments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
    
    /// Tenant reference (optional)
    /// 
    /// If specified, the tag will be managed using this tenant's NetBox API token.
    /// If not specified, the controller will attempt to find a tenant from resources
    /// that reference this tag, or fall back to the system tenant.
    /// 
    /// Note: Tags in NetBox are global resources and don't have a tenant field.
    /// This field only determines which API token to use for tag operations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<NetBoxResourceReference>,
}

/// NetBoxTagStatus defines the observed state of a NetBox tag
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxTagStatus {
    /// NetBox tag ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox tag URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the tag
    pub state: crate::tenancy::netbox_tenant::ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

