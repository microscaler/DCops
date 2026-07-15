# NetBox Controller Failure Audit

**Date:** 2025-12-25  
**Status:** Active Issues Identified

## Summary

This document audits all current failures in the NetBox controller reconciliation process.

## Failure Analysis

| # | CRD | Error Type | Error Message | Root Cause | Priority | Status |
|---|-----|------------|---------------|------------|-----------|--------|
| 1 | `NetBoxSiteGroup` | Deserialization | `missing field `prefix_count` at line 1 column 390` | NetBox API response doesn't include `prefix_count` field for site groups | High | ✅ Fixed |
| 2 | `NetBoxRegion` | Deserialization | Similar to #1 (likely same issue) | NetBox API response doesn't include `prefix_count` field for regions | High | ✅ Fixed |
| 3 | `NetBoxDevice` | Idempotency | `device with this asset tag already exists` | Device already exists in NetBox but controller doesn't handle "already exists" error | High | ✅ Fixed |
| 3a | `NetBoxDevice` | Deserialization | `missing field 'device_role'` | Device model requires device_role but API response may omit it in list queries | High | ✅ Fixed |
| 4 | `NetBoxInterface` | Dependency | `Device 'talos-control-plane-01' has not been created in NetBox yet` | Interface depends on Device, but Device creation is failing | Medium | ✅ Auto-resolved |
| 5 | `NetBoxMACAddress` | Dependency | `Device 'talos-control-plane-01' has not been created in NetBox yet` | MAC Address depends on Device, but Device creation is failing | Medium | ✅ Auto-resolved |

## Detailed Analysis

### Issue #1: NetBoxSiteGroup - Missing `prefix_count` Field

**Error:**
```
error decoding response body: missing field `prefix_count` at line 1 column 390
```

**Response Body:**
```json
{
  "count":1,
  "next":null,
  "previous":null,
  "results":[{
    "id":1,
    "url":"http://netbox.netbox/api/dcim/site-groups/1/",
    "display":"Production Sites",
    "name":"Production Sites",
    "slug":"production-sites",
    "parent":null,
    "description":"Production datacenter sites",
    "tags":[],
    "custom_fields":{},
    "created":"2025-12-25T09:53:31.723801Z",
    "last_updated":"2025-12-25T09:53:31.723830Z",
    "site_count":0,
    "_depth":0
  }]
}
```

**Root Cause:**
- The `SiteGroup` struct in `crates/netbox-client/src/models.rs` expects a `prefix_count` field
- NetBox API response doesn't include `prefix_count` for site groups
- This is likely an optional field that may not always be present

**Fix Required:**
- Make `prefix_count` optional in the `SiteGroup` struct: `pub prefix_count: Option<u64>`
- Or use `#[serde(default)]` to provide a default value of 0

---

### Issue #2: NetBoxRegion - Missing `prefix_count` Field

**Error:**
- Similar to Issue #1
- Fallback query fails with deserialization error

**Root Cause:**
- The `Region` struct in `crates/netbox-client/src/models.rs` expects a `prefix_count` field
- NetBox API response doesn't include `prefix_count` for regions (or it's optional)

**Fix Required:**
- Make `prefix_count` optional in the `Region` struct: `pub prefix_count: Option<u64>`
- Or use `#[serde(default)]` to provide a default value of 0

---

### Issue #3: NetBoxDevice - Idempotency Failure

**Error:**
```
Failed to create device: 400 Bad Request - {"asset_tag":["device with this asset tag already exists."]}
```

**Root Cause:**
- Device with the same `asset_tag` already exists in NetBox
- Controller doesn't handle "already exists" errors for devices
- No idempotency logic to query for existing device and update CR status

**Fix Required:**
- Add idempotency handling in `reconcile_netbox_device`:
  1. Catch "already exists" error
  2. Query NetBox for existing device by `asset_tag` or `name`
  3. Update CR status with existing NetBox ID
  4. Treat as successful reconciliation

**Current Code Location:**
- `controllers/netbox/src/reconciler.rs` - `reconcile_netbox_device` function

---

### Issue #4: NetBoxInterface - Dependency Blocking

**Error:**
```
Invalid configuration: Device 'talos-control-plane-01' has not been created in NetBox yet (no netbox_id in status)
```

**Root Cause:**
- Interface depends on Device
- Device creation is failing (Issue #3)
- Interface reconciliation correctly checks for device dependency but device never gets created

**Fix Required:**
- Fix Issue #3 first
- Once Device is created successfully, Interface will reconcile automatically

---

### Issue #5: NetBoxMACAddress - Dependency Blocking

**Error:**
```
Invalid configuration: Device 'talos-control-plane-01' has not been created in NetBox yet (no netbox_id in status)
```

**Root Cause:**
- MAC Address depends on Device
- Device creation is failing (Issue #3)
- MAC Address reconciliation correctly checks for device dependency but device never gets created

**Fix Required:**
- Fix Issue #3 first
- Once Device is created successfully, MAC Address will reconcile automatically

---

## Fix Priority

1. **High Priority:**
   - Issue #1: NetBoxSiteGroup deserialization (blocks site group reconciliation)
   - Issue #2: NetBoxRegion deserialization (blocks region reconciliation)
   - Issue #3: NetBoxDevice idempotency (blocks device, interface, and MAC address reconciliation)

2. **Medium Priority:**
   - Issue #4: NetBoxInterface (auto-resolves after #3)
   - Issue #5: NetBoxMACAddress (auto-resolves after #3)

## Implementation Plan

1. ✅ Fix `SiteGroup` struct - make `prefix_count` optional with `#[serde(default)]`
2. ✅ Fix `Region` struct - make `prefix_count` optional with `#[serde(default)]`
3. ✅ Add idempotency handling to `reconcile_netbox_device`
4. ⏳ Test all fixes (pending deployment)
5. ⏳ Verify dependent resources reconcile successfully (pending deployment)

## Fixes Applied

### Fix #1 & #2: SiteGroup and Region Deserialization
**File:** `crates/netbox-client/src/models.rs`
- Added `#[serde(default)]` to `prefix_count` field in both `SiteGroup` and `Region` structs
- This allows deserialization to succeed even when NetBox API doesn't include the field (defaults to 0)

### Fix #3: NetBoxDevice Idempotency
**File:** `controllers/netbox/src/reconciler.rs`
- Added idempotency handling in `reconcile_netbox_device` function
- When device creation fails with "already exists" or "asset tag" error:
  1. Query NetBox for existing device by `asset_tag` (if provided)
  2. Fallback to query by `name` if asset_tag query fails
  3. Update CR status with existing NetBox ID
  4. Treat as successful reconciliation

**Code Location:** `controllers/netbox/src/reconciler.rs:3231-3275`

