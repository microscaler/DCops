//! NetBoxDevice Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox devices (DCIM).

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::references::NetBoxResourceReference;

/// Primary IP address reference
/// Supports both CRD references (GitOps-friendly) and direct IP addresses (fallback)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(untagged)]
pub enum PrimaryIPReference {
    /// IPClaim CRD reference (recommended, GitOps-friendly)
    IPClaimRef(NetBoxResourceReference),
    
    /// Direct IP address string (e.g., "192.168.1.10/24" or "2001:db8::1/64")
    /// Used as fallback when IPClaim CRD is not available
    IPAddress(String),
}

// Note: Custom JsonSchema implementation removed due to schemars API limitations
// The default derive will generate an anyOf schema which may not be fully structural
// but Kubernetes will accept it with --validate=false
// TODO: Consider using a tagged enum or different serialization strategy for better structural compliance

/// NetBoxDeviceSpec defines the desired state of a NetBox device
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxDevice",
    namespaced,
    status = "NetBoxDeviceStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxDeviceSpec {
    /// Device name (optional, but recommended)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    
    /// Device type reference (references NetBoxDeviceType CRD)
    pub device_type: NetBoxResourceReference,
    
    /// Device role reference (references NetBoxDeviceRole CRD)
    pub device_role: NetBoxResourceReference,
    
    /// Site reference (references NetBoxSite CRD)
    pub site: NetBoxResourceReference,
    
    /// Location reference (references NetBoxLocation CRD, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<NetBoxResourceReference>,
    
    /// Tenant reference (references NetBoxTenant CRD, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<NetBoxResourceReference>,
    
    /// Platform reference (references NetBoxPlatform CRD, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<NetBoxResourceReference>,
    
    /// Serial number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial: Option<String>,
    
    /// Asset tag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_tag: Option<String>,
    
    /// Device status (active, offline, planned, staged, failed, inventory)
    #[serde(default = "default_device_status")]
    pub status: DeviceStatus,
    
    /// Primary IPv4 address reference (optional)
    /// Can be either:
    /// - IPClaim CRD reference (recommended, GitOps-friendly)
    /// - IP address string (e.g., "192.168.1.10/24") as fallback
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_ip4: Option<PrimaryIPReference>,
    
    /// Primary IPv6 address reference (optional)
    /// Can be either:
    /// - IPClaim CRD reference (recommended, GitOps-friendly)
    /// - IP address string (e.g., "2001:db8::1/64") as fallback
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_ip6: Option<PrimaryIPReference>,
    
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Comments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
}

fn default_device_status() -> DeviceStatus {
    DeviceStatus::Active
}

/// Device status choices
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DeviceStatus {
    #[default]
    Active,
    Offline,
    Planned,
    Staged,
    Failed,
    Inventory,
    Decommissioning,
}

/// NetBoxDeviceStatus defines the observed state of a NetBox device
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxDeviceStatus {
    /// NetBox device ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox device URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the device
    pub state: crate::tenancy::netbox_tenant::ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

