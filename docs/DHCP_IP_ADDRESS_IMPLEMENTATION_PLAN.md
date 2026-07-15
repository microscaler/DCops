# DHCP IP Address Implementation Plan

## Overview

This document outlines the implementation plan for completing the DHCP IP address support, including validation, example YAML files, and comprehensive tests.

## Current Status

✅ **Completed:**
- CRD fields (`macAddress`, `interface`, `status.address`)
- NetBox client support (`assigned_object_type`, `assigned_object_id`)
- Basic reconciler logic (MAC resolution, interface assignment)
- Basic validation (address/ipRange requirement)

⏳ **Remaining:**
- Enhanced validation for DHCP scenarios
- Complete example YAML files for both scenarios
- Comprehensive test coverage

## 1. Validation Enhancements

### 1.1 DHCP Scenario Validation Rules

Based on NetBox documentation and codebase patterns, we need to validate:

#### Scenario 1: Random Allocation from IP Range
**Valid Configuration:**
- `status: dhcp`
- `ipRange: <reference>` (required)
- `address: <not specified>` (will be allocated)
- `macAddress: <not specified>` (not needed)
- `interface: <not specified>` (not needed)

**Validation:**
- ✅ `ipRange` must be specified
- ✅ `address` must NOT be specified in spec (will be in status after allocation)
- ✅ `macAddress` and `interface` are optional (not needed for random allocation)

#### Scenario 2: Static DHCP Reservation
**Valid Configuration:**
- `status: dhcp`
- `address: <specific IP>` (required)
- `macAddress: <hex>` OR `interface: <reference>` (at least one required)
- `ipRange: <optional>` (if specified, address must be within range)

**Validation:**
- ✅ `address` must be specified
- ✅ Either `macAddress` OR `interface` must be specified (interface takes precedence)
- ✅ If `ipRange` is specified, `address` must be within the range
- ✅ MAC address format validation (hex with colons or dashes)

### 1.2 Implementation Location

**File:** `controllers/netbox/src/reconciler/ipam/ip_address.rs`

**Location:** After interface resolution, before IP creation/update

**Pattern to Follow:**
```rust
// Validate DHCP scenario requirements
if ip_address_crd.spec.status == crds::IPAddressStatus::Dhcp {
    if ip_address_crd.spec.address.is_some() {
        // Static reservation: require macAddress or interface
        if ip_address_crd.spec.mac_address.is_none() && ip_address_crd.spec.interface.is_none() {
            let error_msg = "For static DHCP reservations (status: dhcp with address specified), either 'macAddress' or 'interface' must be provided".to_string();
            error!("NetBoxIPAddress {}/{}: {}", namespace, name, error_msg);
            update_status_error(&*self.netbox_ip_address_api, name, namespace, error_msg.clone(), ip_address_crd.status.as_ref()).await;
            return Err(ControllerError::InvalidInput(error_msg));
        }
        
        // Validate MAC address format if provided
        if let Some(mac) = &ip_address_crd.spec.mac_address {
            if !is_valid_mac_address(mac) {
                let error_msg = format!("Invalid MAC address format '{}'. Expected format: 'aa:bb:cc:dd:ee:ff' or 'aa-bb-cc-dd-ee-ff'", mac);
                error!("NetBoxIPAddress {}/{}: {}", namespace, name, error_msg);
                update_status_error(&*self.netbox_ip_address_api, name, namespace, error_msg.clone(), ip_address_crd.status.as_ref()).await;
                return Err(ControllerError::InvalidInput(error_msg));
            }
        }
    } else {
        // Random allocation: require ipRange, no address
        if ip_address_crd.spec.ip_range.is_none() {
            let error_msg = "For random DHCP allocation (status: dhcp without address), 'ipRange' must be specified".to_string();
            error!("NetBoxIPAddress {}/{}: {}", namespace, name, error_msg);
            update_status_error(&*self.netbox_ip_address_api, name, namespace, error_msg.clone(), ip_address_crd.status.as_ref()).await;
            return Err(ControllerError::InvalidInput(error_msg));
        }
    }
}
```

### 1.3 MAC Address Format Validation

**Helper Function:** Add to `controllers/netbox/src/reconcile_helpers.rs`

```rust
/// Validate MAC address format
/// Accepts formats: "aa:bb:cc:dd:ee:ff" or "aa-bb-cc-dd-ee-ff"
pub fn is_valid_mac_address(mac: &str) -> bool {
    // Remove separators and check hex
    let cleaned = mac.replace(':', "").replace('-', "");
    cleaned.len() == 12 && cleaned.chars().all(|c| c.is_ascii_hexdigit())
}
```

## 2. Example YAML Files

### 2.1 Update Existing DHCP Example

**File:** `config/examples/tenant-datacenter-tenant/netbox-ip-address-dhcp-example.yaml`

**Current:** Shows random allocation scenario (ipRange without address)

**Action:** Update to show **static reservation** scenario with MAC address:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxIPAddress
metadata:
  name: dhcp-client-ip-static
  namespace: default
spec:
  # Static DHCP reservation: specific IP assigned to device with MAC address
  address: "192.168.1.100/24"
  
  # MAC address of the interface (required for static DHCP reservations)
  # The reconciler will query NetBox to find the interface with this MAC
  # and assign the IP to that interface
  macAddress: "aa:bb:cc:dd:ee:ff"
  
  # Alternative: You can directly reference the interface instead of MAC
  # interface:
  #   apiGroup: dcops.microscaler.io
  #   kind: NetBoxInterface
  #   name: eth0
  #   namespace: default
  
  # IP range reference (optional, for validation)
  # If specified, the address must be within this range
  ipRange:
    apiGroup: dcops.microscaler.io
    kind: NetBoxIPRange
    name: dhcp-pool-range
    namespace: default
  
  tenant:
    apiGroup: dcops.microscaler.io
    kind: NetBoxTenant
    name: datacenter-tenant
    namespace: default
  
  status: dhcp  # Indicates this is a DHCP-assigned IP
  
  description: "Static DHCP reservation for client device"
  tags:
    - apiGroup: dcops.microscaler.io
      kind: NetBoxTag
      name: dhcp-managed
      namespace: default
    - apiGroup: dcops.microscaler.io
      kind: NetBoxTag
      name: client-device
      namespace: default
```

### 2.2 Create New Example for Random Allocation

**File:** `config/examples/tenant-datacenter-tenant/netbox-ip-address-dhcp-random-example.yaml`

**New file** showing random allocation scenario:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxIPAddress
metadata:
  name: dhcp-client-ip-random
  namespace: default
spec:
  # Random DHCP allocation: IP will be allocated from the range
  # Do NOT specify address here - it will be stored in status.address after reconciliation
  
  # IP range reference (required for random allocation)
  ipRange:
    apiGroup: dcops.microscaler.io
    kind: NetBoxIPRange
    name: dhcp-pool-range
    namespace: default
  
  tenant:
    apiGroup: dcops.microscaler.io
    kind: NetBoxTenant
    name: datacenter-tenant
    namespace: default
  
  status: dhcp  # Indicates this is a DHCP-assigned IP
  
  description: "Random DHCP allocation - IP will be assigned from pool"
  tags:
    - apiGroup: dcops.microscaler.io
      kind: NetBoxTag
      name: dhcp-managed
      namespace: default
    - apiGroup: dcops.microscaler.io
      kind: NetBoxTag
      name: client-device
      namespace: default

# NOTE: After reconciliation, status.address will contain the allocated IP
# Example status after reconciliation:
# status:
#   address: "192.168.1.150/24"  # Allocated from the range
#   netboxId: 42
#   netboxUrl: "http://netbox/api/ipam/ip-addresses/42/"
#   state: Created
```

### 2.3 Create Example with Interface Reference

**File:** `config/examples/tenant-datacenter-tenant/netbox-ip-address-dhcp-interface-example.yaml`

**New file** showing static reservation with interface reference:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxIPAddress
metadata:
  name: dhcp-server-ip
  namespace: default
spec:
  # Static DHCP reservation using interface reference
  address: "192.168.1.1/24"
  
  # Direct interface reference (alternative to MAC address)
  interface:
    apiGroup: dcops.microscaler.io
    kind: NetBoxInterface
    name: eth0
    namespace: default
  
  tenant:
    apiGroup: dcops.microscaler.io
    kind: NetBoxTenant
    name: datacenter-tenant
    namespace: default
  
  status: dhcp
  
  description: "DHCP server IP assigned to interface"
  tags:
    - apiGroup: dcops.microscaler.io
      kind: NetBoxTag
      name: dhcp-managed
      namespace: default
    - apiGroup: dcops.microscaler.io
      kind: NetBoxTag
      name: server
      namespace: default
```

## 3. Test Coverage

### 3.1 Test File Location

**File:** `controllers/netbox/src/reconciler/ipam/ip_address_test.rs`

### 3.2 Test Cases to Add

#### Test 1: Static DHCP Reservation with MAC Address
```rust
#[tokio::test]
async fn test_reconcile_dhcp_static_with_mac() {
    // Setup: Create IP address CRD with:
    // - status: dhcp
    // - address: "192.168.1.100/24"
    // - macAddress: "aa:bb:cc:dd:ee:ff"
    // - ipRange reference
    
    // Setup: Mock interface with matching MAC
    // Setup: Mock NetBox client to return interface on query
    
    // Execute: Reconcile
    
    // Assert:
    // - IP created with assigned_object_type: "dcim.interface"
    // - IP created with assigned_object_id: <interface_id>
    // - Status updated with address
}
```

#### Test 2: Static DHCP Reservation with Interface Reference
```rust
#[tokio::test]
async fn test_reconcile_dhcp_static_with_interface() {
    // Setup: Create IP address CRD with:
    // - status: dhcp
    // - address: "192.168.1.100/24"
    // - interface reference
    
    // Setup: Mock NetBoxInterface CRD
    // Setup: Mock NetBox client
    
    // Execute: Reconcile
    
    // Assert:
    // - IP created with assigned_object_type: "dcim.interface"
    // - IP created with assigned_object_id: <interface_id>
}
```

#### Test 3: Random DHCP Allocation from IP Range
```rust
#[tokio::test]
async fn test_reconcile_dhcp_random_allocation() {
    // Setup: Create IP address CRD with:
    // - status: dhcp
    // - ipRange reference
    // - NO address specified
    
    // Setup: Mock IP range
    // Setup: Mock NetBox to allocate IP from range
    
    // Execute: Reconcile
    
    // Assert:
    // - IP allocated from range
    // - Status.address contains allocated IP
    // - Status.netboxId set
}
```

#### Test 4: Validation - Missing MAC/Interface for Static Reservation
```rust
#[tokio::test]
async fn test_validation_dhcp_static_missing_mac_and_interface() {
    // Setup: Create IP address CRD with:
    // - status: dhcp
    // - address: "192.168.1.100/24"
    // - NO macAddress or interface
    
    // Execute: Reconcile
    
    // Assert:
    // - Returns InvalidInput error
    // - Error message mentions macAddress or interface required
    // - Status updated with error
}
```

#### Test 5: Validation - Missing IP Range for Random Allocation
```rust
#[tokio::test]
async fn test_validation_dhcp_random_missing_ip_range() {
    // Setup: Create IP address CRD with:
    // - status: dhcp
    // - NO address
    // - NO ipRange
    
    // Execute: Reconcile
    
    // Assert:
    // - Returns InvalidInput error
    // - Error message mentions ipRange required
}
```

#### Test 6: Validation - Invalid MAC Address Format
```rust
#[tokio::test]
async fn test_validation_invalid_mac_format() {
    // Setup: Create IP address CRD with:
    // - status: dhcp
    // - address: "192.168.1.100/24"
    // - macAddress: "invalid-format"
    
    // Execute: Reconcile
    
    // Assert:
    // - Returns InvalidInput error
    // - Error message mentions valid MAC format
}
```

#### Test 7: MAC Address Resolution Failure
```rust
#[tokio::test]
async fn test_mac_resolution_failure() {
    // Setup: Create IP address CRD with:
    // - status: dhcp
    // - address: "192.168.1.100/24"
    // - macAddress: "aa:bb:cc:dd:ee:ff"
    
    // Setup: Mock NetBox to return no interfaces for MAC
    
    // Execute: Reconcile
    
    // Assert:
    // - Warning logged about MAC not found
    // - IP created without interface assignment (graceful degradation)
}
```

#### Test 8: Interface Assignment Update
```rust
#[tokio::test]
async fn test_update_dhcp_ip_with_interface_assignment() {
    // Setup: Existing IP address in NetBox without interface
    // Setup: Update CRD to add interface reference
    
    // Execute: Reconcile
    
    // Assert:
    // - IP updated with assigned_object_type and assigned_object_id
}
```

## 4. Implementation Order

1. **Add MAC address validation helper** (`reconcile_helpers.rs`)
2. **Add DHCP scenario validation** (`ip_address.rs`)
3. **Update existing DHCP example** (static reservation with MAC)
4. **Create random allocation example**
5. **Create interface reference example**
6. **Add test cases** (start with validation tests, then functional tests)

## 5. NetBox API Considerations

### 5.1 IP Range Allocation

**Finding:** NetBox does NOT have an `available-ips` endpoint for IP ranges (only for prefixes).

**Implication:** For Scenario 1 (random allocation), we have two options:

**Option A:** Require prefix reference instead of IP range for random allocation
- Use existing `allocate_ip` function with prefix
- Simpler implementation
- More restrictive (requires prefix, not just range)

**Option B:** Implement manual IP selection from range
- Query all IPs in the range
- Find first available IP
- Create IP address manually
- More complex but supports IP ranges

**Recommendation:** Start with Option A (prefix-based), document Option B as future enhancement.

### 5.2 Interface Assignment

**Confirmed:** NetBox supports `assigned_object_type: "dcim.interface"` and `assigned_object_id: <interface_id>` ✅

**Confirmed:** MAC address query via `query_interfaces(&[("mac_address", mac)])` ✅

## 6. Documentation Updates

Update `docs/DHCP_IP_ADDRESS_INVESTIGATION.md` with:
- Implementation status for validation
- Implementation status for examples
- Implementation status for tests
- Notes about IP range allocation limitations

## 7. Testing Strategy

### 7.1 Unit Tests
- Validation logic (isolated)
- MAC address format validation
- DHCP scenario detection

### 7.2 Integration Tests
- Full reconciliation flow with mock NetBox
- Interface resolution
- IP creation with assignment

### 7.3 Manual Testing Checklist
- [ ] Apply static reservation example (MAC address)
- [ ] Apply static reservation example (interface reference)
- [ ] Apply random allocation example
- [ ] Verify IP assigned to interface in NetBox
- [ ] Verify status.address populated for random allocation
- [ ] Test validation errors (missing fields, invalid formats)

## References

- NetBox IP Address API: `/api/ipam/ip-addresses/`
- NetBox Interface API: `/api/dcim/interfaces/`
- NetBox IP Range API: `/api/ipam/ip-ranges/`
- NetBox Prefix Available IPs: `/api/ipam/prefixes/{id}/available-ips/`
- Existing validation patterns: `controllers/netbox/src/reconciler/ipam/ip_claim.rs`
- Existing test patterns: `controllers/netbox/src/reconciler/ipam/ip_address_test.rs`

