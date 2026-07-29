# Tenant Reference Audit

**Date:** 2025-12-25  
**Status:** 🔍 **AUDIT COMPLETE** - Issues identified and fixes planned

## Overview

This document audits tenant references across NetBox resources, CRDs, reconcilers, and API methods to ensure tenants are correctly set when creating/updating resources.

## NetBox API Models with Tenant Support

From `crates/netbox-client/src/models.rs`:

| Model | Tenant Field | Type |
|-------|--------------|------|
| `Prefix` | `pub tenant: Option<NestedTenant>` | ✅ Supported |
| `IPAddress` | `pub tenant: Option<NestedTenant>` | ✅ Supported |
| `Device` | `pub tenant: Option<NestedTenant>` | ✅ Supported |
| `Site` | `pub tenant: Option<NestedTenant>` | ✅ Supported |
| `VLAN` | `pub tenant: Option<NestedTenant>` | ✅ Supported |

## CRDs with Tenant Fields

From `crates/crds/src/`:

| CRD | Tenant Field | Status |
|-----|--------------|--------|
| `NetBoxPrefix` | `pub tenant: Option<NetBoxResourceReference>` | ✅ Has field |
| `NetBoxDevice` | `pub tenant: Option<NetBoxResourceReference>` | ✅ Has field |
| `NetBoxSite` | `pub tenant: Option<NetBoxResourceReference>` | ✅ Has field |
| `NetBoxVLAN` | `pub tenant: Option<NetBoxResourceReference>` | ✅ Has field |

## API Methods - Tenant Support

From `crates/netbox-client/src/client.rs`:

| Method | Tenant Parameter | Status |
|--------|------------------|--------|
| `create_prefix()` | ❌ **MISSING** | ❌ **BUG** |
| `update_prefix()` | ❌ **MISSING** | ❌ **BUG** |
| `create_device()` | `tenant_id: Option<u64>` | ✅ Supported |
| `update_device()` | `tenant_id: Option<u64>` | ✅ Supported |
| `create_site()` | `tenant_id: Option<u64>` | ✅ Supported |
| `update_site()` | `tenant_id: Option<u64>` | ✅ Supported |
| `create_vlan()` | `tenant_id: Option<u64>` | ✅ Supported |

## Reconcilers - Tenant Resolution and Usage

### ✅ NetBoxSite (`controllers/netbox/src/reconciler/dcim/site.rs`)
- **Resolves tenant:** ✅ Yes (lines 159-175)
- **Passes to create_site:** ✅ Yes
- **Passes to update_site:** ✅ Yes (with change detection)
- **Status:** ✅ **WORKING**

### ✅ NetBoxDevice (`controllers/netbox/src/reconciler/dcim/device.rs`)
- **Resolves tenant:** ✅ Yes (lines 175-190)
- **Passes to create_device:** ✅ Yes (line 220)
- **Passes to update_device:** ❓ Not checked (device doesn't have update logic yet)
- **Status:** ✅ **WORKING** (for creation)

### ✅ NetBoxVLAN (`controllers/netbox/src/reconciler/dcim/vlan.rs`)
- **Resolves tenant:** ✅ Yes (lines 129-146)
- **Passes to create_vlan:** ✅ Yes (line 192)
- **Status:** ✅ **WORKING**

### ❌ NetBoxPrefix (`controllers/netbox/src/reconciler/ipam/prefix.rs`)
- **Resolves tenant:** ✅ Yes (lines 231-252)
- **Passes to create_prefix:** ❌ **NO** - `create_prefix()` doesn't accept tenant_id!
- **Passes to update_prefix:** ❌ **NO** - `update_prefix()` doesn't accept tenant_id!
- **Status:** ❌ **BROKEN** - Tenant resolved but not used

## Issues Identified

### 🔴 Critical Issue #1: `create_prefix()` Missing Tenant Parameter

**Location:** `crates/netbox-client/src/client.rs:859`

**Problem:**
```rust
pub async fn create_prefix(
    &self,
    prefix: &str,
    description: Option<String>,
    site_id: Option<u64>,
    vlan_id: Option<u32>,
    status: Option<&str>,
    role_id: Option<u64>,
    tags: Option<Vec<String>>,
    // ❌ MISSING: tenant_id: Option<u64>
) -> Result<Prefix, NetBoxError>
```

**Impact:**
- NetBoxPrefix CRD has tenant field
- Reconciler resolves tenant ID
- But tenant is never passed to NetBox API
- Prefixes are created without tenant assignment

**Fix Required:**
1. Add `tenant_id: Option<u64>` parameter to `create_prefix()`
2. Add tenant to request body if provided
3. Update reconciler to pass resolved tenant_id

### 🔴 Critical Issue #2: `update_prefix()` Missing Tenant Parameter

**Location:** `crates/netbox-client/src/client.rs:938`

**Problem:**
```rust
pub async fn update_prefix(
    &self,
    id: u64,
    prefix: Option<&str>,
    description: Option<String>,
    status: Option<&str>,
    role: Option<String>,
    tags: Option<Vec<String>>,
    // ❌ MISSING: tenant_id: Option<u64>
) -> Result<Prefix, NetBoxError>
```

**Impact:**
- Cannot update tenant on existing prefixes
- Tenant changes in CRD spec are ignored

**Fix Required:**
1. Add `tenant_id: Option<u64>` parameter to `update_prefix()`
2. Add tenant to request body if provided
3. Update reconciler to pass resolved tenant_id (when update logic is added)

### 🟡 Medium Issue #3: Prefix Reconciler Doesn't Pass Tenant

**Location:** `controllers/netbox/src/reconciler/ipam/prefix.rs:320`

**Problem:**
- Reconciler resolves tenant_id (lines 231-252)
- But `create_prefix()` call doesn't accept tenant_id parameter
- Tenant is silently ignored

**Impact:**
- Prefixes created without tenant even when CRD specifies tenant

**Fix Required:**
- Wait for `create_prefix()` to be fixed, then update call site

## Summary Table

| Resource | CRD Has Tenant | API Method Has Tenant | Reconciler Resolves | Reconciler Passes | Status |
|----------|----------------|----------------------|---------------------|-------------------|--------|
| **Prefix** | ✅ | ✅ | ✅ | ✅ | ✅ **FIXED** |
| **Device** | ✅ | ✅ | ✅ | ✅ | ✅ **WORKING** |
| **Site** | ✅ | ✅ | ✅ | ✅ | ✅ **WORKING** |
| **VLAN** | ✅ | ✅ | ✅ | ✅ | ✅ **WORKING** |

## Action Items

1. ✅ **Audit Complete** - All tenant references identified
2. ✅ **Fix `create_prefix()`** - Add tenant_id parameter
3. ✅ **Fix `update_prefix()`** - Add tenant_id parameter
4. ✅ **Update prefix reconciler** - Pass tenant_id to create_prefix and update_prefix
5. ✅ **Verify other resources** - Device, Site, VLAN are working correctly

## Fixes Applied

### ✅ Fixed `create_prefix()` - Added tenant_id parameter
- **File:** `crates/netbox-client/src/client.rs:859`
- **Change:** Added `tenant_id: Option<u64>` parameter
- **Change:** Added tenant to request body if provided

### ✅ Fixed `update_prefix()` - Added tenant_id parameter
- **File:** `crates/netbox-client/src/client.rs:943`
- **Change:** Added `tenant_id: Option<u64>` parameter
- **Change:** Added tenant to request body if provided

### ✅ Updated prefix reconciler - Pass tenant_id
- **File:** `controllers/netbox/src/reconciler/ipam/prefix.rs`
- **Change:** Pass `tenant_id` to `create_prefix()` call (line 346)
- **Change:** Pass `tenant_id` to `update_prefix()` call (line 318)

## Testing Checklist

After fixes:
- [ ] Create NetBoxPrefix with tenant reference → Verify tenant set in NetBox
- [ ] Update NetBoxPrefix tenant → Verify tenant updated in NetBox
- [ ] Create NetBoxPrefix without tenant → Verify no tenant set (optional)
- [ ] Verify existing Device, Site, VLAN tenant assignments still work

