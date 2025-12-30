//! NetBoxIPAddress Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox IP addresses.
//! This allows GitOps-style management of NetBox IP addresses.

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::references::NetBoxResourceReference;

/// NetBoxIPAddressSpec defines the desired state of a NetBox IP address
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxIPAddress",
    namespaced,
    status = "NetBoxIPAddressStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxIPAddressSpec {
    /// IP address with CIDR (e.g., "192.168.1.10/24" or "2001:db8::1/64")
    /// Must be a valid CIDR notation (IP address with prefix length)
    /// 
    /// For DHCP-assigned IPs:
    /// - If tracking an already-assigned IP, specify the address here
    /// - If using an IP range reference, address is optional (will be allocated from range)
    /// - At least one of `address` or `ip_range` must be specified
    #[schemars(pattern(r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)/(?:[0-9]|[12][0-9]|3[0-2])$|^(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}/(?:[0-9]|[1-9][0-9]|1[0-2][0-8])$"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    
    /// IP range reference (references NetBoxIPRange CRD, optional)
    /// 
    /// For DHCP-assigned IPs, reference the IP range pool from which the IP was assigned.
    /// This links the IP address to its DHCP pool for proper tracking and management.
    /// 
    /// If `ip_range` is specified and `address` is not, the reconciler will allocate
    /// an IP from the range. If both are specified, the address must be within the range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_range: Option<NetBoxResourceReference>,
    
    /// Tenant reference (references NetBoxTenant CRD, required)
    /// Tenant is required in NetBox for proper resource organization and access control
    pub tenant: NetBoxResourceReference,
    
    /// VRF reference (references NetBoxVRF CRD, optional - not yet implemented)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vrf: Option<NetBoxResourceReference>,
    
    /// VLAN reference (references NetBoxVLAN CRD, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vlan: Option<NetBoxResourceReference>,
    
    /// IP address status (active, reserved, deprecated, dhcp, slaac)
    #[serde(default = "default_ip_address_status")]
    pub status: IPAddressStatus,
    
    /// IP address role (loopback, secondary, anycast, vip, vrrp, hsrp, glbp, carp)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    
    /// DNS name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_name: Option<String>,
    
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Tag references (references NetBoxTag CRDs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<NetBoxResourceReference>>,
    
    /// Comments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
}

fn default_ip_address_status() -> IPAddressStatus {
    IPAddressStatus::Active
}

/// IP address status in NetBox
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum IPAddressStatus {
    #[default]
    Active,
    Reserved,
    Deprecated,
    Dhcp,
    Slaac,
}

/// NetBoxIPAddressStatus defines the observed state of a NetBox IP address
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxIPAddressStatus {
    /// NetBox IP address ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox IP address URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the IP address
    pub state: crate::tenancy::netbox_tenant::ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}


