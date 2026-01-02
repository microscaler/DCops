//! Type definitions for NetBox client
//!
//! This module provides newtype wrappers for NetBox IDs to prevent mixing
//! different ID types and improve type safety.

use serde::{Deserialize, Serialize};
use std::fmt;

// ============================================================================
// Display implementations for formatting
// ============================================================================

// ============================================================================
// ID Types - Newtype wrappers to prevent mixing
// ============================================================================

/// Tenant ID - prevents mixing with other ID types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TenantId(pub u64);

/// Site ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SiteId(pub u64);

/// Device ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeviceId(pub u64);

/// Interface ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InterfaceId(pub u64);

/// Prefix ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PrefixId(pub u64);

/// IP Address ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IpAddressId(pub u64);

/// VLAN ID - Note: VLAN IDs are u32 in NetBox, not u64
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VlanId(pub u32);

/// Region ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RegionId(pub u64);

/// Site Group ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SiteGroupId(pub u64);

/// Location ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LocationId(pub u64);

/// Device Role ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeviceRoleId(pub u64);

/// Device Type ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeviceTypeId(pub u64);

/// Manufacturer ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ManufacturerId(pub u64);

/// Platform ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlatformId(pub u64);

/// Role ID (IPAM Role)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RoleId(pub u64);

/// Aggregate ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AggregateId(pub u64);

/// RIR ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RirId(pub u64);

/// VLAN Group ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VlanGroupId(pub u64);

/// Tenant Group ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TenantGroupId(pub u64);

/// IP Range ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IPRangeId(pub u64);

/// VRF ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VrfId(pub u64);

/// Route Target ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RouteTargetId(pub u64);

// ============================================================================
// Conversion traits for convenience
// ============================================================================

// TenantId conversions
impl From<u64> for TenantId {
    fn from(id: u64) -> Self {
        TenantId(id)
    }
}

impl From<TenantId> for u64 {
    fn from(id: TenantId) -> Self {
        id.0
    }
}

// SiteId conversions
impl From<u64> for SiteId {
    fn from(id: u64) -> Self {
        SiteId(id)
    }
}

impl From<SiteId> for u64 {
    fn from(id: SiteId) -> Self {
        id.0
    }
}

// DeviceId conversions
impl From<u64> for DeviceId {
    fn from(id: u64) -> Self {
        DeviceId(id)
    }
}

impl From<DeviceId> for u64 {
    fn from(id: DeviceId) -> Self {
        id.0
    }
}

// InterfaceId conversions
impl From<u64> for InterfaceId {
    fn from(id: u64) -> Self {
        InterfaceId(id)
    }
}

impl From<InterfaceId> for u64 {
    fn from(id: InterfaceId) -> Self {
        id.0
    }
}

// PrefixId conversions
impl From<u64> for PrefixId {
    fn from(id: u64) -> Self {
        PrefixId(id)
    }
}

impl From<PrefixId> for u64 {
    fn from(id: PrefixId) -> Self {
        id.0
    }
}

// IpAddressId conversions
impl From<u64> for IpAddressId {
    fn from(id: u64) -> Self {
        IpAddressId(id)
    }
}

impl From<IpAddressId> for u64 {
    fn from(id: IpAddressId) -> Self {
        id.0
    }
}

// VlanId conversions (u32)
impl From<u32> for VlanId {
    fn from(id: u32) -> Self {
        VlanId(id)
    }
}

impl From<VlanId> for u32 {
    fn from(id: VlanId) -> Self {
        id.0
    }
}

// RegionId conversions
impl From<u64> for RegionId {
    fn from(id: u64) -> Self {
        RegionId(id)
    }
}

impl From<RegionId> for u64 {
    fn from(id: RegionId) -> Self {
        id.0
    }
}

// SiteGroupId conversions
impl From<u64> for SiteGroupId {
    fn from(id: u64) -> Self {
        SiteGroupId(id)
    }
}

impl From<SiteGroupId> for u64 {
    fn from(id: SiteGroupId) -> Self {
        id.0
    }
}

// LocationId conversions
impl From<u64> for LocationId {
    fn from(id: u64) -> Self {
        LocationId(id)
    }
}

impl From<LocationId> for u64 {
    fn from(id: LocationId) -> Self {
        id.0
    }
}

// DeviceRoleId conversions
impl From<u64> for DeviceRoleId {
    fn from(id: u64) -> Self {
        DeviceRoleId(id)
    }
}

impl From<DeviceRoleId> for u64 {
    fn from(id: DeviceRoleId) -> Self {
        id.0
    }
}

// DeviceTypeId conversions
impl From<u64> for DeviceTypeId {
    fn from(id: u64) -> Self {
        DeviceTypeId(id)
    }
}

impl From<DeviceTypeId> for u64 {
    fn from(id: DeviceTypeId) -> Self {
        id.0
    }
}

// ManufacturerId conversions
impl From<u64> for ManufacturerId {
    fn from(id: u64) -> Self {
        ManufacturerId(id)
    }
}

impl From<ManufacturerId> for u64 {
    fn from(id: ManufacturerId) -> Self {
        id.0
    }
}

// PlatformId conversions
impl From<u64> for PlatformId {
    fn from(id: u64) -> Self {
        PlatformId(id)
    }
}

impl From<PlatformId> for u64 {
    fn from(id: PlatformId) -> Self {
        id.0
    }
}

// RoleId conversions
impl From<u64> for RoleId {
    fn from(id: u64) -> Self {
        RoleId(id)
    }
}

impl From<RoleId> for u64 {
    fn from(id: RoleId) -> Self {
        id.0
    }
}

// AggregateId conversions
impl From<u64> for AggregateId {
    fn from(id: u64) -> Self {
        AggregateId(id)
    }
}

impl From<AggregateId> for u64 {
    fn from(id: AggregateId) -> Self {
        id.0
    }
}

// RirId conversions
impl From<u64> for RirId {
    fn from(id: u64) -> Self {
        RirId(id)
    }
}

impl From<RirId> for u64 {
    fn from(id: RirId) -> Self {
        id.0
    }
}

// VlanGroupId conversions
impl From<u64> for VlanGroupId {
    fn from(id: u64) -> Self {
        VlanGroupId(id)
    }
}

impl From<VlanGroupId> for u64 {
    fn from(id: VlanGroupId) -> Self {
        id.0
    }
}

// TenantGroupId conversions
impl From<u64> for TenantGroupId {
    fn from(id: u64) -> Self {
        TenantGroupId(id)
    }
}

impl From<TenantGroupId> for u64 {
    fn from(id: TenantGroupId) -> Self {
        id.0
    }
}

// IPRangeId conversions
impl From<u64> for IPRangeId {
    fn from(id: u64) -> Self {
        IPRangeId(id)
    }
}

impl From<IPRangeId> for u64 {
    fn from(id: IPRangeId) -> Self {
        id.0
    }
}

// VrfId conversions
impl From<u64> for VrfId {
    fn from(id: u64) -> Self {
        VrfId(id)
    }
}

impl From<VrfId> for u64 {
    fn from(id: VrfId) -> Self {
        id.0
    }
}

// RouteTargetId conversions
impl From<u64> for RouteTargetId {
    fn from(id: u64) -> Self {
        RouteTargetId(id)
    }
}

impl From<RouteTargetId> for u64 {
    fn from(id: RouteTargetId) -> Self {
        id.0
    }
}

// ============================================================================
// Display implementations for formatting
// ============================================================================

impl fmt::Display for SiteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for PrefixId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for IPRangeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// Type Aliases for non-ID types
// ============================================================================

/// NetBox API URL (e.g., "http://netbox:80/api/dcim/sites/1/")
pub type NetBoxUrl = String;

/// NetBox slug (lowercase, hyphenated identifier)
pub type NetBoxSlug = String;

/// NetBox name (human-readable name)
pub type NetBoxName = String;

