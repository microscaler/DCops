//! NetBoxIPRange Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox IP ranges.
//! IP ranges represent contiguous sequences of IP addresses, commonly used for DHCP pools.

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::references::NetBoxResourceReference;

/// NetBoxIPRangeSpec defines the desired state of a NetBox IP range
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxIPRange",
    namespaced,
    status = "NetBoxIPRangeStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxIPRangeSpec {
    /// Start IP address with CIDR (e.g., "192.168.1.100/24" or "2001:db8::100/64")
    /// Must be a valid CIDR notation (IP address with prefix length)
    #[schemars(pattern(r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)/(?:[0-9]|[12][0-9]|3[0-2])$|^(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}/(?:[0-9]|[1-9][0-9]|1[0-2][0-8])$"))]
    pub start_address: String,

    /// End IP address with CIDR (e.g., "192.168.1.200/24" or "2001:db8::200/64")
    /// Must be a valid CIDR notation (IP address with prefix length)
    /// Must be in the same family (IPv4 or IPv6) as start_address
    #[schemars(pattern(r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)/(?:[0-9]|[12][0-9]|3[0-2])$|^(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}/(?:[0-9]|[1-9][0-9]|1[0-2][0-8])$"))]
    pub end_address: String,

    /// Tenant reference (references NetBoxTenant CRD, required)
    /// Tenant is required in NetBox for proper resource organization and access control
    pub tenant: NetBoxResourceReference,

    /// VRF reference (references NetBoxVRF CRD, optional - not yet implemented)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vrf: Option<NetBoxResourceReference>,

    /// IP range status (active, reserved, deprecated)
    #[serde(default = "default_ip_range_status")]
    pub status: IPRangeStatus,

    /// IP range role reference (references NetBoxRole CRD, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<NetBoxResourceReference>,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Mark as utilized (for DHCP ranges, set to true)
    /// When true, all IPs in the range are considered utilized
    #[serde(default = "default_false")]
    pub mark_utilized: bool,

    /// Mark as populated (for DHCP ranges, set to true)
    /// When true, all IPs in the range are considered populated
    #[serde(default = "default_false")]
    pub mark_populated: bool,

    /// Tag references (references NetBoxTag CRDs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<NetBoxResourceReference>>,
}

fn default_ip_range_status() -> IPRangeStatus {
    IPRangeStatus::Active
}

fn default_false() -> bool {
    false
}

/// IP range status in NetBox
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum IPRangeStatus {
    #[default]
    Active,
    Reserved,
    Deprecated,
}

/// NetBoxIPRangeStatus defines the observed state of a NetBox IP range
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxIPRangeStatus {
    /// NetBox IP range ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,

    /// NetBox IP range URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,

    /// Current state of the IP range
    pub state: crate::tenancy::netbox_tenant::ResourceState,

    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

