//! NetBoxAggregate Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox IPAM aggregates.

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// NetBoxAggregateSpec defines the desired state of a NetBox aggregate
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxAggregate",
    namespaced,
    status = "NetBoxAggregateStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxAggregateSpec {
    /// Aggregate prefix (e.g., "192.168.0.0/16" or "2001:db8::/32")
    /// Must be a valid CIDR notation (IP address with prefix length)
    #[schemars(pattern(r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)/(?:[0-9]|[12][0-9]|3[0-2])$|^(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}/(?:[0-9]|[1-9][0-9]|1[0-2][0-8])$"))]
    pub prefix: String,
    
    /// RIR (Regional Internet Registry) - ARIN, RIPE, APNIC, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rir: Option<String>,
    
    /// Date allocated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_allocated: Option<String>, // ISO 8601 date
    
    /// Description of the aggregate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Comments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
}

/// NetBoxAggregateStatus defines the observed state of a NetBox aggregate
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxAggregateStatus {
    /// NetBox aggregate ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox aggregate URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the aggregate
    pub state: crate::tenancy::netbox_tenant::ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

