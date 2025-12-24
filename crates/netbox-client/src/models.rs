//! NetBox API models
//!
//! These models match the NetBox REST API serializers.
//! See: netbox/netbox/ipam/api/serializers_/ip.py

use serde::{Deserialize, Serialize};

/// NetBox API response wrapper (for paginated responses)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub count: u64,
    pub next: Option<String>,
    pub previous: Option<String>,
    pub results: Vec<T>,
}

/// Prefix model matching NetBox PrefixSerializer
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Prefix {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub family: u8, // 4 or 6
    pub prefix: String, // e.g., "192.168.1.0/24"
    pub vrf: Option<NestedVrf>,
    pub tenant: Option<NestedTenant>,
    pub vlan: Option<NestedVlan>,
    pub status: PrefixStatus,
    pub role: Option<NestedRole>,
    pub is_pool: bool,
    pub mark_utilized: bool,
    pub description: String,
    pub comments: String,
    pub tags: Vec<NestedTag>,
    pub custom_fields: serde_json::Value,
    pub created: String, // ISO 8601 datetime
    pub last_updated: String, // ISO 8601 datetime
    pub children: u64,
    pub _depth: u64,
}

/// IP Address model matching NetBox IPAddressSerializer
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct IPAddress {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub family: u8, // 4 or 6
    pub address: String, // e.g., "192.168.1.1/24"
    pub vrf: Option<NestedVrf>,
    pub tenant: Option<NestedTenant>,
    pub status: IPAddressStatus,
    pub role: Option<String>, // IPAddressRoleChoices
    pub assigned_object_type: Option<String>,
    pub assigned_object_id: Option<u64>,
    pub assigned_object: Option<serde_json::Value>,
    pub nat_inside: Option<NestedIPAddress>,
    pub nat_outside: Vec<NestedIPAddress>,
    pub dns_name: String,
    pub description: String,
    pub comments: String,
    pub tags: Vec<NestedTag>,
    pub custom_fields: serde_json::Value,
    pub created: String, // ISO 8601 datetime
    pub last_updated: String, // ISO 8601 datetime
}

/// Available IP Address (from prefix available-ips endpoint)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AvailableIP {
    pub family: u8,
    pub address: String, // e.g., "192.168.1.1/24"
    pub vrf: Option<NestedVrf>,
    pub description: Option<String>,
}

/// Request body for allocating an IP address
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AllocateIPRequest {
    pub address: Option<String>, // Optional: specific IP to allocate
    pub description: Option<String>,
    pub status: Option<IPAddressStatus>,
    pub role: Option<String>,
    pub dns_name: Option<String>,
    pub tags: Option<Vec<String>>, // Tag slugs
}

/// Device model (from DCIM API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Device {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub device_type: NestedDeviceType,
    pub device_role: NestedDeviceRole,
    pub site: Option<NestedSite>,
    pub interfaces: Vec<Interface>,
    pub tags: Vec<NestedTag>,
    pub created: String,
    pub last_updated: String,
}

/// Interface model (from DCIM API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Interface {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub mac_address: Option<String>,
    pub device: NestedDevice,
    pub ip_addresses: Vec<NestedIPAddress>,
    pub tags: Vec<NestedTag>,
}

// Nested serializers (simplified versions for references)

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NestedVrf {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NestedTenant {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NestedVlan {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub vid: u16,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NestedRole {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NestedTag {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NestedIPAddress {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NestedDevice {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NestedDeviceType {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub model: String,
    pub manufacturer: NestedManufacturer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NestedDeviceRole {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NestedManufacturer {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NestedSite {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
}

/// Prefix status choices
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PrefixStatus {
    Container,
    Active,
    Reserved,
    Deprecated,
}

/// IP Address status choices
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum IPAddressStatus {
    Active,
    Reserved,
    Deprecated,
    Dhcp,
    #[serde(rename = "slaac")]
    Slaac,
}

/// VLAN model (from IPAM API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Vlan {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub site: Option<NestedSite>,
    pub group: Option<NestedVlanGroup>,
    pub vid: u16,
    pub name: String,
    pub tenant: Option<NestedTenant>,
    pub status: VlanStatus,
    pub role: Option<NestedRole>,
    pub description: String,
    pub comments: String,
    pub tags: Vec<NestedTag>,
    pub custom_fields: serde_json::Value,
    pub created: String,
    pub last_updated: String,
}

/// VLAN status choices
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum VlanStatus {
    Active,
    Reserved,
    Deprecated,
}

/// VLAN Group (nested)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NestedVlanGroup {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
}
