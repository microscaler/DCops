# NetBox IP Address DHCP Scenarios Investigation

## Overview

This document tracks the investigation and implementation of DHCP IP address management scenarios in NetBox.

## Two DHCP Scenarios

### Scenario 1: Random IP Allocation from Pool
- **Description**: IP is allocated randomly from a DHCP pool when a device requests it
- **Requirements**:
  - `spec.ipRange`: Reference to NetBoxIPRange CRD (DHCP pool)
  - `spec.status: dhcp`
  - `spec.address`: NOT specified (will be allocated and stored in `status.address`)
- **NetBox API**: Uses IP range allocation or prefix available-ips endpoint

### Scenario 2: Static DHCP Reservation
- **Description**: IP is statically set in the CR and handed out to a device, usually associated with the interface's MAC address
- **Requirements**:
  - `spec.status: dhcp`
  - `spec.address`: Static IP address (e.g., "192.168.1.100/24")
  - `spec.macAddress`: MAC address in hex format (e.g., "aa:bb:cc:dd:ee:ff")
  - `spec.interface`: Reference to NetBoxInterface CRD (optional, can be resolved from MAC)
- **NetBox API**: Uses `assigned_object_type` and `assigned_object_id` to assign IP to interface

## NetBox API Investigation

### IP Address Model Fields

From `crates/netbox-client/src/models.rs`:

```rust
pub struct IPAddress {
    // ... other fields ...
    pub assigned_object_type: Option<String>,  // e.g., "dcim.interface"
    pub assigned_object_id: Option<u64>,    // ID of the assigned interface
    pub assigned_object: Option<serde_json::Value>,  // Nested object data
    // ... other fields ...
}
```

### Key Findings

1. **Interface Assignment**: IP addresses can be assigned to interfaces using:
   - `assigned_object_type`: "dcim.interface"
   - `assigned_object_id`: Interface ID

2. **MAC Address Resolution**: 
   - Interfaces have `mac_address: Option<String>`
   - MACAddress objects can be created and assigned to interfaces
   - We can query interfaces by MAC address to find the correct interface

3. **Current Client Limitations**:
   - `create_ip_address` does NOT support `assigned_object_type`/`assigned_object_id`
   - `update_ip_address` does NOT support `assigned_object_type`/`assigned_object_id`
   - Need to add these fields to the API client

## Required Changes

### 1. CRD Updates (`crates/crds/src/ipam/netbox_ip_address.rs`)

Add to `NetBoxIPAddressSpec`:

```rust
/// MAC address in hex format (e.g., "aa:bb:cc:dd:ee:ff")
/// Required for static DHCP reservations (status: dhcp)
/// Used to resolve the interface to which this IP should be assigned
#[serde(skip_serializing_if = "Option::is_none")]
pub mac_address: Option<String>,

/// Interface reference (references NetBoxInterface CRD, optional)
/// For static DHCP reservations, this can be resolved from mac_address
/// If both mac_address and interface are specified, interface takes precedence
#[serde(skip_serializing_if = "Option::is_none")]
pub interface: Option<NetBoxResourceReference>,
```

### 2. NetBox Client Updates

#### Update `AllocateIPRequest` (`crates/netbox-client/src/ipam/ip_address.rs`)

Add fields:
```rust
pub struct AllocateIPRequest {
    // ... existing fields ...
    pub assigned_object_type: Option<String>,  // e.g., "dcim.interface"
    pub assigned_object_id: Option<u64>,       // Interface ID
}
```

#### Update `create_ip_address` function

Add support for `assigned_object_type` and `assigned_object_id` in request body:
```rust
if let Some(obj_type) = req.assigned_object_type {
    body["assigned_object_type"] = serde_json::Value::String(obj_type);
}
if let Some(obj_id) = req.assigned_object_id {
    body["assigned_object_id"] = serde_json::json!(obj_id);
}
```

#### Update `update_ip_address` function

Add support for `assigned_object_type` and `assigned_object_id` in request body.

### 3. Reconciler Updates (`controllers/netbox/src/reconciler/ipam/ip_address.rs`)

#### For Static DHCP Reservations (Scenario 2):

1. **MAC Address Resolution**:
   - If `spec.macAddress` is provided, query NetBox for interface with that MAC
   - If found, use interface ID for `assigned_object_id`
   - If `spec.interface` is also provided, use that instead (takes precedence)

2. **IP Creation/Update**:
   - Include `assigned_object_type: "dcim.interface"` and `assigned_object_id` in API request
   - This assigns the IP to the interface in NetBox

#### For Random Allocation (Scenario 1):

1. **IP Range Allocation**:
   - If `spec.ipRange` is provided and `spec.address` is NOT provided:
     - Allocate IP from range (need to check if NetBox supports this for IP ranges)
     - Store allocated IP in `status.address`
   - If both are provided, validate address is within range

### 4. Validation Logic

Add validation:
- If `status: dhcp` and `macAddress` is provided, this is a static reservation
- If `status: dhcp` and `ipRange` is provided (no address), this is random allocation
- If `status: dhcp` and both `address` and `macAddress` are provided, this is static reservation
- If `status: dhcp` and only `ipRange` is provided, this is random allocation

## Findings

### MAC Address Query ✅

**Found**: `query_interfaces` supports filtering by MAC address!

From `crates/netbox-client/src/dcim/interface.rs`:
```rust
pub async fn query_interfaces(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<Interface>, NetBoxError>
```

**Usage**: Filter interfaces by MAC address:
```rust
let interfaces = query_interfaces(core, &[("mac_address", mac)], false).await?;
```

**Example**: `get_device_by_mac` in `crates/netbox-client/src/dcim/device.rs` already uses this pattern!

### Interface Resolution Strategy

1. If `spec.interface` is provided → Use that directly (highest priority)
2. If `spec.macAddress` is provided → Query interfaces by MAC, use first match
3. If neither provided → No interface assignment

## Open Questions

1. **IP Range Allocation**: Does NetBox support allocating IPs from IP ranges (not just prefixes)?
   - Current code only supports prefix allocation via `/api/ipam/prefixes/{id}/available-ips/`
   - Need to check if IP ranges have a similar endpoint
   - **Note**: For Scenario 1 (random allocation), we may need to:
     - Query available IPs from the range manually
     - Or use a different NetBox API endpoint
     - Or require prefix reference instead of IP range for random allocation

## Implementation Status

- [x] Add `macAddress` and `interface` fields to CRD
- [x] Update NetBox client `AllocateIPRequest` struct
- [x] Update `create_ip_address` to support `assigned_object_type`/`assigned_object_id`
- [x] Update `update_ip_address` to support `assigned_object_type`/`assigned_object_id`
- [x] Update `allocate_ip` to support `assigned_object_type`/`assigned_object_id`
- [x] Add MAC address resolution logic in reconciler
- [x] Add interface assignment logic in reconciler
- [x] Basic validation (address/ipRange requirement)
- [ ] Enhanced validation for DHCP scenarios (see implementation plan)
- [ ] Update example YAML files for both scenarios (see implementation plan)
- [ ] Add comprehensive tests for both DHCP scenarios (see implementation plan)

## Next Steps

See `docs/DHCP_IP_ADDRESS_IMPLEMENTATION_PLAN.md` for detailed implementation plan covering:
- Enhanced validation rules for DHCP scenarios
- MAC address format validation
- Example YAML files for all scenarios
- Comprehensive test cases
- NetBox API considerations (IP range allocation limitations)

## References

- NetBox IP Address API: `/api/ipam/ip-addresses/`
- NetBox Interface API: `/api/dcim/interfaces/`
- NetBox MAC Address API: `/api/dcim/mac-addresses/`
- NetBox IP Range API: `/api/ipam/ip-ranges/`

