# Reconciliation Scenarios and Status Update Logic

**Date:** 2025-12-25  
**Status:** ✅ **DOCUMENTED**

## Overview

This document clarifies:
1. **Which values trigger status updates** for each CRD type
2. **Reconciliation scenarios** (CR changes vs NetBox drift)
3. **When each CRD needs status updates** and what fields are checked

## Reconciliation Scenarios

### Scenario 1: CR Document Updated (Spec Changed)

**Trigger:** Kubernetes watch event when CR `spec` changes (generation increments)

**Process:**
1. Controller detects spec change (generation changed)
2. Reconciler reads CR spec
3. Reconciler checks if resource exists in NetBox (by `netbox_id` from status)
4. If exists: Compare spec with NetBox resource → Update NetBox if different
5. If not exists: Create resource in NetBox
6. Update CR status with NetBox ID, URL, state, error (if any)

**Status Update:** ✅ **YES** - Always update status after NetBox operation completes

**Values Checked:**
- `netbox_id`: NetBox resource ID (set after creation)
- `netbox_url`: NetBox resource URL
- `state`: Reconciliation state (Pending → Created/Updated/Failed)
- `error`: Error message if operation failed

### Scenario 2: NetBox Changed (Drift Detection)

**Trigger:** Periodic reconciliation or explicit drift check

**Process:**
1. Reconciler reads CR status to get `netbox_id`
2. Reconciler queries NetBox API for resource by ID
3. If **NotFound**: Drift detected → Clear status → Recreate resource
4. If **Found**: Compare spec with NetBox resource → Update NetBox if different
5. Update CR status to reflect current state

**Status Update:** ✅ **YES** - Update status to reflect drift or current state

**Values Checked:**
- Same as Scenario 1, plus:
- Drift detection: Clear `netbox_id` and set `state` to `Pending` if resource deleted

### Scenario 3: No Changes (Idempotent Check)

**Trigger:** Controller reconciliation loop (after debounce period)

**Process:**
1. Reconciler reads CR spec and status
2. Reconciler checks if resource exists in NetBox (by `netbox_id`)
3. If exists: Compare spec with NetBox resource
4. If **no changes needed**: Skip NetBox update
5. Check if status needs updating (compare current status with desired status)
6. If **status unchanged**: Skip status update → Return early

**Status Update:** ❌ **NO** - Skip status update if values haven't changed

**Values Checked:**
- `netbox_id`: Must match NetBox resource ID
- `netbox_url`: Must match NetBox resource URL
- `state`: Must match expected state (Created/Updated)
- `error`: Must be `None` (no errors)

## Status Update Values by CRD Type

### Standard NetBox CRDs (16 types)

**Status Fields Checked by `status_needs_update()`:**
- `netbox_id: Option<u64>` - NetBox resource ID (set after creation)
- `netbox_url: Option<String>` - NetBox resource URL (set after creation)
- `state: ResourceState` - Pending, Created, Updated, Failed
- `error: Option<String>` - Error message if reconciliation failed

**When Status Updates:**
- ✅ **Scenario 1 (CR Spec Changed):** After creating/updating resource in NetBox
  - `netbox_id`: Set to NetBox resource ID
  - `netbox_url`: Set to NetBox resource URL
  - `state`: Pending → Created (on create) or Created → Updated (on update)
  - `error`: Cleared (set to None)
  
- ✅ **Scenario 2 (Drift Detected):** After detecting resource deleted in NetBox
  - `netbox_id`: Cleared (set to None or 0)
  - `netbox_url`: Cleared (set to empty string)
  - `state`: Created → Pending
  - `error`: Set to "Resource was deleted in NetBox, will recreate"
  
- ✅ **Scenario 3 (Error Occurred):** After reconciliation error
  - `netbox_id`: May be cleared or kept (depending on error)
  - `netbox_url`: May be cleared or kept (depending on error)
  - `state`: Created → Failed
  - `error`: Set to error message
  
- ❌ **Scenario 4 (No Changes):** Skip status update if all values match
  - All four fields (`netbox_id`, `netbox_url`, `state`, `error`) match desired values
  - Prevents reconciliation loops from unnecessary status updates

**CRDs Using This Pattern:**
- NetBoxDevice
- NetBoxSite
- NetBoxTenant
- NetBoxInterface
- NetBoxPlatform
- NetBoxRegion
- NetBoxSiteGroup
- NetBoxLocation
- NetBoxDeviceRole
- NetBoxManufacturer
- NetBoxDeviceType
- NetBoxVLAN
- NetBoxRole
- NetBoxTag
- NetBoxAggregate
- NetBoxPrefix (uses `PrefixState` instead of `ResourceState`)

### IPClaim (Special Case)

**Status Fields Checked by `ipclaim_status_needs_update()`:**
- `ip: Option<String>` - **Allocated IP address** (critical field - primary indicator)
- `state: AllocationState` - Pending, Allocated, Failed
- `netbox_ip_ref: Option<String>` - NetBox IPAddress object URL
- `error: Option<String>` - Error message if allocation failed

**When Status Updates:**
- ✅ **Scenario 1 (IP Allocated):** After successfully allocating IP from NetBox
  - `ip`: Set to allocated IP address (e.g., "192.168.1.10/24")
  - `state`: Pending → Allocated
  - `netbox_ip_ref`: Set to NetBox IPAddress URL
  - `error`: Cleared (set to None)
  
- ✅ **Scenario 2 (Existing IP Found):** After finding existing IP in NetBox (idempotency)
  - `ip`: Set to existing IP address
  - `state`: Pending → Allocated
  - `netbox_ip_ref`: Set to existing IP's URL
  - `error`: Cleared (set to None)
  
- ✅ **Scenario 3 (Allocation Failed):** After allocation failure
  - `ip`: Cleared (set to None)
  - `state`: Pending → Failed
  - `netbox_ip_ref`: Cleared (set to None)
  - `error`: Set to error message
  
- ✅ **Scenario 4 (Drift Detected):** After detecting IP was deallocated in NetBox
  - `ip`: Cleared (set to None)
  - `state`: Allocated → Pending
  - `netbox_ip_ref`: Cleared (set to None)
  - `error`: Set to "IP address was deallocated in NetBox"
  
- ❌ **Scenario 5 (No Changes):** Skip status update if all values match
  - All four fields (`ip`, `state`, `netbox_ip_ref`, `error`) match desired values
  - Prevents reconciliation loops from unnecessary status updates

**Special Considerations:**
- IPClaim status **must** show the allocated IP address - this is the primary value users need
- The `ip` field is the critical indicator of successful allocation
- Status update is essential for users to see which IP was allocated
- Unlike other CRDs, IPClaim tracks the actual IP address, not just a NetBox ID

### IPPool (Special Case)

**Status Fields:**
- `total_ips: u32` - Total IPs in pool
- `allocated_ips: u32` - Number of allocated IPs
- `available_ips: u32` - Number of available IPs

**When Status Updates:**
- ✅ After querying NetBox prefix to count IPs
- ✅ When IP allocation/deallocation changes counts
- Status reflects current pool utilization

**Note:** IPPool doesn't use `NetBoxStatusCheck` trait (different status structure)

### NetBoxMACAddress (Special Case)

**Status Fields:** Same as standard CRDs, but MAC addresses are managed via interfaces

**When Status Updates:**
- ✅ After setting MAC address on interface
- ✅ After detecting MAC address changed on interface
- Uses interface ID/URL as proxy for MAC address status

## Implementation Details

### Status Comparison Logic

The `status_needs_update()` helper function checks:

```rust
status.netbox_id() != Some(desired_netbox_id)
    || status.netbox_url().as_deref() != Some(desired_netbox_url)
    || status.state_str() != desired_state
    || status.error() != desired_error
```

**Returns `true` if ANY field changed, `false` if all fields match.**

### IPClaim Status Comparison Logic

The `ipclaim_status_needs_update()` helper function checks:

```rust
status.allocated_ip() != desired_ip
    || status.state_str() != desired_state
    || status.netbox_url() != desired_netbox_ip_ref
    || status.error() != desired_error
```

**Returns `true` if ANY field changed (including IP address), `false` if all fields match.**

## Reconciliation Flow

```
┌─────────────────────────────────────────────────────────────┐
│ Controller detects change (generation or periodic)          │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│ Reconciler reads CR spec and status                         │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│ Check if resource exists in NetBox (by netbox_id)           │
└───────────────────────┬─────────────────────────────────────┘
                        │
        ┌───────────────┴───────────────┐
        │                               │
        ▼                               ▼
┌───────────────┐              ┌───────────────┐
│ EXISTS        │              │ NOT EXISTS    │
└───────┬───────┘              └───────┬───────┘
        │                               │
        ▼                               ▼
┌─────────────────────────────────────────────────────────────┐
│ Compare spec with NetBox resource                            │
│ - If different: Update NetBox                               │
│ - If same: Skip NetBox update                               │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│ Check if status needs updating                               │
│ - Compare current status with desired status                 │
│ - If different: Update status                               │
│ - If same: Skip status update → Return early                │
└─────────────────────────────────────────────────────────────┘
```

## Preventing Reconciliation Loops

### Problem
Status updates trigger watch events → Reconciliation → Status update → Loop

### Solution
1. **Debounce:** 5-second debounce batches rapid events
2. **Status Comparison:** Only update status if values actually changed
3. **Generation Filtering:** Controller filters status-only updates (generation unchanged)
4. **Early Return:** Skip reconciliation if status already correct

### Status Update Decision Tree

```
Is status update needed?
├─ No status exists → YES, update
├─ netbox_id changed → YES, update
├─ netbox_url changed → YES, update
├─ state changed → YES, update
├─ error changed → YES, update
└─ All values match → NO, skip update (return early)
```

## Testing Status Updates

### Test Cases

1. **Initial Creation:**
   - CR created → Status should update with `netbox_id`, `netbox_url`, `state: Created`

2. **Spec Change:**
   - CR spec updated → NetBox updated → Status should update with `state: Updated`

3. **Drift Detection:**
   - Resource deleted in NetBox → Status should clear `netbox_id`, set `state: Pending`

4. **No Changes:**
   - Reconciliation with no changes → Status should NOT update (skip)

5. **IPClaim Allocation:**
   - IP allocated → Status should update with `ip`, `state: Allocated`, `netbox_ip_ref`

6. **IPClaim Re-allocation:**
   - IP already allocated → Status should NOT update if values match

## Summary

### Status Updates Occur When:

**Scenario 1: CR Spec Changed**
- ✅ Resource created in NetBox → Update status with `netbox_id`, `netbox_url`, `state: Created`
- ✅ Resource updated in NetBox → Update status with `state: Updated`
- ✅ IPClaim IP allocated → Update status with `ip`, `state: Allocated`, `netbox_ip_ref`

**Scenario 2: NetBox Changed (Drift)**
- ✅ Resource deleted in NetBox → Clear status (`netbox_id` = 0, `state: Pending`)
- ✅ IPClaim IP deallocated in NetBox → Clear status (`ip` = None, `state: Pending`)

**Scenario 3: Error Occurred**
- ✅ Reconciliation failed → Update status with `state: Failed`, `error: <message>`

### Status Updates Are Skipped When:

**Scenario 4: No Changes (Idempotent)**
- ❌ All status values already match desired values
- ❌ Resource exists in NetBox and spec matches
- ❌ IPClaim already allocated to same IP
- ❌ Reconciliation is idempotent (no-op)

### Key Values Checked:

**Standard NetBox CRDs (16 types):**
- `netbox_id: Option<u64>` - NetBox resource ID
- `netbox_url: Option<String>` - NetBox resource URL  
- `state: ResourceState` - Pending, Created, Updated, Failed
- `error: Option<String>` - Error message

**IPClaim (Special Case):**
- `ip: Option<String>` - **Allocated IP address** (critical - primary indicator)
- `state: AllocationState` - Pending, Allocated, Failed
- `netbox_ip_ref: Option<String>` - NetBox IPAddress URL
- `error: Option<String>` - Error message

**IPPool (Special Case):**
- `total_ips: u32` - Total IPs in pool
- `allocated_ips: u32` - Number of allocated IPs
- `available_ips: u32` - Number of available IPs

### Implementation Status

- ✅ All 16 standard NetBox CRDs use `status_needs_update()` helper
- ✅ IPClaim uses `ipclaim_status_needs_update()` helper (includes IP address check)
- ✅ Status updates only occur when values actually change
- ✅ Early return prevents unnecessary status updates and reconciliation loops

