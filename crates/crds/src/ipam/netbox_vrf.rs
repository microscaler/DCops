//! NetBoxVRF Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox VRFs (Virtual Routing and Forwarding).
//! VRFs represent independent routing tables, allowing for the isolation of network traffic
//! and the use of overlapping IP address spaces.

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::references::NetBoxResourceReference;

/// NetBoxVRFSpec defines the desired state of a NetBox VRF
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxVRF",
    namespaced,
    status = "NetBoxVRFStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxVRFSpec {
    /// VRF name (required)
    /// Administrative name for the VRF
    pub name: String,

    /// Route Distinguisher (optional)
    /// RFC 4364 format (e.g., "65000:1" or "192.168.1.1:1")
    /// Used to distinguish routes within the VRF
    /// If not specified, NetBox will auto-generate one
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rd: Option<String>,

    /// Enforce unique IP space (optional, default: false)
    /// When true, prevents duplicate prefixes/IPs within this VRF
    /// Useful for ensuring IP address uniqueness across overlapping address spaces
    #[serde(default = "default_false")]
    pub enforce_unique: bool,

    /// Tenant reference (optional)
    /// VRFs can be scoped to a tenant for multi-tenancy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<NetBoxResourceReference>,

    /// Description (optional)
    /// Human-readable description of the VRF's purpose
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Comments (optional)
    /// Additional notes or documentation about this VRF
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,

    /// Import Route Targets (optional)
    /// List of Route Target references for importing routes into this VRF
    /// Routes with these route targets will be imported into this VRF
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import_targets: Option<Vec<NetBoxResourceReference>>,

    /// Export Route Targets (optional)
    /// List of Route Target references for exporting routes from this VRF
    /// Routes from this VRF will be tagged with these route targets for export
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_targets: Option<Vec<NetBoxResourceReference>>,

    /// Tag references (references NetBoxTag CRDs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<NetBoxResourceReference>>,
}

fn default_false() -> bool {
    false
}

/// NetBoxVRFStatus defines the observed state of a NetBox VRF
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxVRFStatus {
    /// NetBox VRF ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,

    /// NetBox VRF URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,

    /// Current state of the VRF
    pub state: crate::tenancy::netbox_tenant::ResourceState,

    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

