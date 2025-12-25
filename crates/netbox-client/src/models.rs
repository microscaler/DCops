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

/// NetBox choice field (value/label structure)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ChoiceField<T> {
    pub value: T,
    pub label: String,
}

/// Prefix model matching NetBox PrefixSerializer
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Prefix {
    pub id: u64,
    pub url: String,
    pub display: String,
    #[serde(deserialize_with = "deserialize_family")]
    pub family: u8, // 4 or 6 (extracted from ChoiceField)
    pub prefix: String, // e.g., "192.168.1.0/24"
    pub vrf: Option<NestedVrf>,
    pub tenant: Option<NestedTenant>,
    pub vlan: Option<NestedVlan>,
    #[serde(deserialize_with = "deserialize_prefix_status")]
    pub status: PrefixStatus, // Extracted from ChoiceField
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

// Helper deserializers for NetBox choice fields
fn deserialize_family<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let choice: ChoiceField<u8> = ChoiceField::deserialize(deserializer)?;
    Ok(choice.value)
}

fn deserialize_prefix_status<'de, D>(deserializer: D) -> Result<PrefixStatus, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let choice: ChoiceField<String> = ChoiceField::deserialize(deserializer)?;
    match choice.value.as_str() {
        "active" => Ok(PrefixStatus::Active),
        "reserved" => Ok(PrefixStatus::Reserved),
        "deprecated" => Ok(PrefixStatus::Deprecated),
        "container" => Ok(PrefixStatus::Container),
        _ => Err(serde::de::Error::custom(format!("Unknown prefix status: {}", choice.value))),
    }
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
    pub tags: Option<Vec<serde_json::Value>>, // Tag references: numeric IDs or dictionaries with "name"/"slug"
}

/// Device model (from DCIM API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Device {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: Option<String>, // Devices can be unnamed
    pub device_type: NestedDeviceType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_role: Option<NestedDeviceRole>, // Optional in some API responses
    pub tenant: Option<NestedTenant>,
    pub platform: Option<NestedPlatform>,
    pub site: Option<NestedSite>,
    pub location: Option<NestedLocation>,
    #[serde(deserialize_with = "deserialize_device_status")]
    pub status: DeviceStatus,
    pub serial: Option<String>,
    pub asset_tag: Option<String>,
    pub primary_ip4: Option<NestedIPAddress>,
    pub primary_ip6: Option<NestedIPAddress>,
    pub description: Option<String>,
    pub comments: Option<String>,
    pub tags: Vec<NestedTag>,
    pub created: String,
    pub last_updated: String,
}

/// Device status choices
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DeviceStatus {
    Active,
    Offline,
    Planned,
    Staged,
    Failed,
    Inventory,
    Decommissioning,
}

fn deserialize_device_status<'de, D>(deserializer: D) -> Result<DeviceStatus, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let choice: ChoiceField<String> = ChoiceField::deserialize(deserializer)?;
    match choice.value.as_str() {
        "active" => Ok(DeviceStatus::Active),
        "offline" => Ok(DeviceStatus::Offline),
        "planned" => Ok(DeviceStatus::Planned),
        "staged" => Ok(DeviceStatus::Staged),
        "failed" => Ok(DeviceStatus::Failed),
        "inventory" => Ok(DeviceStatus::Inventory),
        "decommissioning" => Ok(DeviceStatus::Decommissioning),
        _ => Ok(DeviceStatus::Active), // Default to active
    }
}

/// Nested Location (for references)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NestedLocation {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
}

/// Interface model (from DCIM API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Interface {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub device: NestedDevice,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vdcs: Vec<serde_json::Value>, // VDCs (Virtual Device Contexts) - can be empty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<serde_json::Value>, // Module reference (optional)
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>, // Optional label
    #[serde(deserialize_with = "deserialize_interface_type")]
    pub r#type: String, // Interface type (e.g., "1000base-t", "virtual") - extracted from ChoiceField
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<serde_json::Value>, // Parent interface (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge: Option<serde_json::Value>, // Bridge interface (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lag: Option<serde_json::Value>, // LAG interface (optional)
    pub mac_address: Option<String>,
    pub mtu: Option<u16>,
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ip_addresses: Vec<NestedIPAddress>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<NestedTag>,
    pub created: String,
    pub last_updated: String,
}

// Helper deserializer for Interface type (ChoiceField)
fn deserialize_interface_type<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let choice: ChoiceField<String> = ChoiceField::deserialize(deserializer)?;
    Ok(choice.value)
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
    #[serde(deserialize_with = "deserialize_vlan_status")]
    pub status: VlanStatus,
    pub role: Option<NestedRole>,
    pub description: String,
    pub comments: String,
    pub tags: Vec<NestedTag>,
    pub custom_fields: serde_json::Value,
    pub created: String,
    pub last_updated: String,
}

fn deserialize_vlan_status<'de, D>(deserializer: D) -> Result<VlanStatus, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let choice: ChoiceField<String> = ChoiceField::deserialize(deserializer)?;
    match choice.value.as_str() {
        "active" => Ok(VlanStatus::Active),
        "reserved" => Ok(VlanStatus::Reserved),
        "deprecated" => Ok(VlanStatus::Deprecated),
        _ => Ok(VlanStatus::Active), // Default to active
    }
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

/// Tenant model (from Tenancy API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Tenant {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub comments: Option<String>,
    pub group: Option<NestedTenantGroup>,
    pub created: String,
    pub last_updated: String,
}

/// Tenant Group (nested)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NestedTenantGroup {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
}

/// Site model (from DCIM API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Site {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub physical_address: Option<String>,
    pub shipping_address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub tenant: Option<NestedTenant>,
    pub region: Option<NestedRegion>,
    pub site_group: Option<NestedSiteGroup>,
    #[serde(deserialize_with = "deserialize_site_status")]
    pub status: SiteStatus,
    pub facility: Option<String>,
    pub time_zone: Option<String>,
    pub comments: Option<String>,
    pub tags: Vec<NestedTag>,
    pub created: String,
    pub last_updated: String,
}

/// Site status choices
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SiteStatus {
    Active,
    Planned,
    Retired,
    Staging,
}

fn deserialize_site_status<'de, D>(deserializer: D) -> Result<SiteStatus, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let choice: ChoiceField<String> = ChoiceField::deserialize(deserializer)?;
    match choice.value.as_str() {
        "active" => Ok(SiteStatus::Active),
        "planned" => Ok(SiteStatus::Planned),
        "retired" => Ok(SiteStatus::Retired),
        "staging" => Ok(SiteStatus::Staging),
        _ => Err(serde::de::Error::custom(format!("Unknown site status: {}", choice.value))),
    }
}

/// Role model (from IPAM API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Role {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub weight: Option<u16>,
    pub comments: Option<String>,
    pub created: String,
    pub last_updated: String,
}

/// Tag model (from Extras API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Tag {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
    pub color: String,
    pub description: Option<String>,
    pub comments: Option<String>,
    pub created: String,
    pub last_updated: String,
}

/// Aggregate model (from IPAM API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Aggregate {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub prefix: String,
    pub rir: Option<NestedRir>,
    pub date_allocated: Option<String>,
    pub description: Option<String>,
    pub comments: Option<String>,
    pub tags: Vec<NestedTag>,
    pub created: String,
    pub last_updated: String,
}

/// RIR (Regional Internet Registry) nested model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NestedRir {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
}

/// RIR (Regional Internet Registry) full model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Rir {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub is_private: bool,
    pub created: String,
    pub last_updated: String,
}

/// Tenant Group model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TenantGroup {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub comments: Option<String>,
    pub parent: Option<NestedTenantGroup>,
    pub tenant_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _depth: Option<u32>, // MPTT depth field, optional
    pub created: String,
    pub last_updated: String,
}

// ============================================================================
// DCIM Models
// ============================================================================

/// Device Role model (from DCIM API)
/// Device Role model (from DCIM API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DeviceRole {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
    pub color: Option<String>,
    pub vm_role: bool,
    pub description: Option<String>,
    pub comments: Option<String>,
    pub device_count: u64,
    pub virtualmachine_count: u64,
    pub created: String,
    pub last_updated: String,
}

/// Manufacturer model (from DCIM API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Manufacturer {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub devicetype_count: u64,
    pub inventoryitem_count: u64,
    pub platform_count: u64,
    pub created: String,
    pub last_updated: String,
}

/// Platform model (from DCIM API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Platform {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
    pub manufacturer: Option<NestedManufacturer>,
    pub napalm_driver: Option<String>,
    pub napalm_args: Option<String>,
    pub description: Option<String>,
    pub comments: Option<String>,
    pub device_count: u64,
    pub virtualmachine_count: u64,
    pub created: String,
    pub last_updated: String,
}

/// Device Type model (from DCIM API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DeviceType {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub manufacturer: NestedManufacturer,
    pub model: String,
    pub slug: String,
    pub part_number: Option<String>,
    pub u_height: f64,
    pub is_full_depth: bool,
    pub description: Option<String>,
    pub comments: Option<String>,
    pub device_count: u64,
    pub created: String,
    pub last_updated: String,
}

/// MAC Address model (from DCIM API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MACAddress {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub mac_address: String,
    pub assigned_object_type: Option<String>,
    pub assigned_object_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_object: Option<serde_json::Value>, // Nested object (interface, etc.)
    pub description: Option<String>,
    pub comments: Option<String>,
    pub tags: Vec<NestedTag>,
    pub created: String,
    pub last_updated: String,
}

/// Nested Platform (for references)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NestedPlatform {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
}

/// Region model (from DCIM API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Region {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
    pub parent: Option<NestedRegion>,
    pub description: Option<String>,
    pub comments: Option<String>,
    pub site_count: u64,
    #[serde(default)]
    pub prefix_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _depth: Option<u32>,
    pub created: String,
    pub last_updated: String,
}

/// Nested Region (for references)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NestedRegion {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
}

/// Site Group model (from DCIM API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SiteGroup {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
    pub parent: Option<NestedSiteGroup>,
    pub description: Option<String>,
    pub comments: Option<String>,
    pub site_count: u64,
    #[serde(default)]
    pub prefix_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _depth: Option<u32>,
    pub created: String,
    pub last_updated: String,
}

/// Nested Site Group (for references)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NestedSiteGroup {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
}

/// Location model (from DCIM API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Location {
    pub id: u64,
    pub url: String,
    pub display: String,
    pub name: String,
    pub slug: String,
    pub site: NestedSite,
    pub parent: Option<NestedLocation>,
    pub description: Option<String>,
    pub comments: Option<String>,
    pub device_count: u64,
    pub rack_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _depth: Option<u32>,
    pub created: String,
    pub last_updated: String,
}
