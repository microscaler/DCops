//! NetBoxRouteTarget Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox Route Targets.
//! Route targets are extended BGP communities used to manage route redistribution
//! among VRF tables, particularly in L3VPN scenarios.

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::references::NetBoxResourceReference;

/// NetBoxRouteTargetSpec defines the desired state of a NetBox Route Target
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxRouteTarget",
    namespaced,
    status = "NetBoxRouteTargetStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxRouteTargetSpec {
    /// Route Target name (required)
    /// Format: RFC 4360 format (e.g., "65000:100" or "65000:100:200")
    /// This is a unique identifier for the route target
    pub name: String,

    /// Tenant reference (optional)
    /// Route targets can be scoped to a tenant for multi-tenancy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<NetBoxResourceReference>,

    /// Description (optional)
    /// Human-readable description of the route target's purpose
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Comments (optional)
    /// Additional notes or documentation about this route target
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,

    /// Tag references (references NetBoxTag CRDs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<NetBoxResourceReference>>,
    
    /// Enable drift detection (default: true)
    /// 
    /// When enabled, the reconciler will detect and correct any changes made to the resource
    /// in the NetBox UI that differ from the CRD spec. Git is the source of truth.
    /// Set to false to disable drift detection (not recommended for GitOps workflows).
    #[serde(skip_serializing_if = "Option::is_none", default = "default_drift_detection")]
    pub drift_detection: Option<bool>,
}

fn default_drift_detection() -> Option<bool> {
    Some(true)
}

/// NetBoxRouteTargetStatus defines the observed state of a NetBox Route Target
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxRouteTargetStatus {
    /// NetBox Route Target ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,

    /// NetBox Route Target URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,

    /// Current state of the Route Target
    pub state: crate::tenancy::netbox_tenant::ResourceState,

    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

