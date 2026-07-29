# Update Logic Audit - All Reconcilers

**Date:** 2025-12-25  
**Status:** 🔍 **AUDIT COMPLETE** - Issues identified

## Overview

This document audits which reconcilers have proper update logic to detect changes and update NetBox resources when CR specs change.

## Update Logic Status by Reconciler

| Reconciler | Has Update Logic? | Helper Used | Status |
|------------|-------------------|-------------|--------|
| **NetBoxSite** | ✅ Yes | `check_and_update_existing()` + `site_needs_update()` | ✅ **WORKING** |
| **NetBoxPrefix** | ❌ No | `check_existing()` only | 🔴 **BROKEN** |
| **NetBoxDevice** | ❌ No | `check_existing()` only | 🔴 **BROKEN** |
| **NetBoxVLAN** | ❌ No | `check_existing()` only | 🔴 **BROKEN** |
| **NetBoxTenant** | ❌ No | `check_existing()` only | 🟡 **OK** (no update fields) |
| **NetBoxAggregate** | ❌ No | `check_existing()` only | 🔴 **BROKEN** |
| **NetBoxRole** | ❌ No | `check_existing()` only | 🟡 **OK** (simple resource) |
| **NetBoxTag** | ❌ No | `check_existing()` only | 🟡 **OK** (simple resource) |
| **NetBoxRegion** | ❌ No | `check_existing()` only | 🔴 **BROKEN** |
| **NetBoxSiteGroup** | ❌ No | `check_existing()` only | 🔴 **BROKEN** |
| **NetBoxLocation** | ❌ No | `check_existing()` only | 🔴 **BROKEN** |
| **NetBoxDeviceRole** | ❌ No | `check_existing()` only | 🟡 **OK** (simple resource) |
| **NetBoxManufacturer** | ❌ No | `check_existing()` only | 🟡 **OK** (simple resource) |
| **NetBoxPlatform** | ❌ No | `check_existing()` only | 🟡 **OK** (simple resource) |
| **NetBoxDeviceType** | ❌ No | `check_existing()` only | 🔴 **BROKEN** |
| **NetBoxInterface** | ❌ No | `check_existing()` only | 🔴 **BROKEN** |
| **NetBoxMACAddress** | ❌ No | `check_existing()` only | 🔴 **BROKEN** |

## Critical Issue: Missing Update Logic

### 🔴 Problem

Most reconcilers use `check_existing()` which:
- ✅ Detects if resource exists in NetBox
- ✅ Detects if resource was deleted (drift)
- ❌ **Does NOT detect if spec changed**
- ❌ **Does NOT update NetBox when spec changes**

### Impact

When a CR spec changes (e.g., tenant reference added):
1. Controller reconciles
2. Finds resource exists in NetBox (by netbox_id)
3. **Assumes resource is up-to-date**
4. **Does NOT update NetBox**
5. **NetBox resource remains unchanged**

### Example: NetBoxPrefix Tenant Issue

**Scenario:**
- Prefix exists in NetBox (ID: 2) without tenant
- CR spec has tenant reference
- Controller reconciles
- Uses `check_existing()` → finds prefix exists
- **Returns early without updating tenant**
- **NetBox prefix still has no tenant**

## Working Example: NetBoxSite

NetBoxSite has proper update logic:

```rust
// 1. Check if resource exists
match reconcile_helpers::check_and_update_existing(
    &self.netbox_client,
    netbox_id,
    &format!("NetBoxSite {}/{}", namespace, name),
    self.netbox_client.get_site(netbox_id),
    |existing| Self::site_needs_update(
        &site_crd.spec,
        existing,
        tenant_id,
        region_id,
        site_group_id,
        &status_str,
    ),
    self.netbox_client.update_site(...),
).await {
    // Updates NetBox if changes detected
}
```

**Key Components:**
1. `check_and_update_existing()` - Generic helper for drift detection + updates
2. `site_needs_update()` - Compares spec with existing NetBox resource
3. `update_site()` - Updates NetBox if changes detected

## Required Fixes

### Priority 1: Critical Resources (Have Complex Fields)

1. **NetBoxPrefix** 🔴
   - Has: tenant, site, vlan, role, description, status
   - Needs: `prefix_needs_update()` + `check_and_update_existing()`

2. **NetBoxDevice** 🔴
   - Has: tenant, site, location, platform, device_type, device_role, description
   - Needs: `device_needs_update()` + `check_and_update_existing()`

3. **NetBoxVLAN** 🔴
   - Has: tenant, site, role, description, status
   - Needs: `vlan_needs_update()` + `check_and_update_existing()`

4. **NetBoxAggregate** 🔴
   - Has: rir, tenant, description
   - Needs: `aggregate_needs_update()` + `check_and_update_existing()`

5. **NetBoxDeviceType** 🔴
   - Has: manufacturer, model, description
   - Needs: `device_type_needs_update()` + `check_and_update_existing()`

6. **NetBoxInterface** 🔴
   - Has: device, type, description, enabled, mtu
   - Needs: `interface_needs_update()` + `check_and_update_existing()`

7. **NetBoxMACAddress** 🔴
   - Has: interface, address
   - Needs: `mac_address_needs_update()` + `check_and_update_existing()`

### Priority 2: Medium Priority (Simple Resources)

8. **NetBoxRegion** 🔴
   - Has: name, slug, description, parent
   - Needs: `region_needs_update()` + `check_and_update_existing()`

9. **NetBoxSiteGroup** 🔴
   - Has: name, slug, description
   - Needs: `site_group_needs_update()` + `check_and_update_existing()`

10. **NetBoxLocation** 🔴
    - Has: name, site, description
    - Needs: `location_needs_update()` + `check_and_update_existing()`

### Priority 3: Low Priority (Very Simple Resources)

11. **NetBoxTenant** 🟡
    - Has: name, slug, description, group
    - Update logic not critical (rarely changes)

12. **NetBoxRole** 🟡
    - Has: name, slug, description
    - Update logic not critical (rarely changes)

13. **NetBoxTag** 🟡
    - Has: name, slug, description
    - Update logic not critical (rarely changes)

14. **NetBoxDeviceRole** 🟡
    - Has: name, slug, description, color
    - Update logic not critical (rarely changes)

15. **NetBoxManufacturer** 🟡
    - Has: name, slug, description
    - Update logic not critical (rarely changes)

16. **NetBoxPlatform** 🟡
    - Has: name, slug, description
    - Update logic not critical (rarely changes)

## Implementation Pattern

### Step 1: Create `*_needs_update()` Function

```rust
fn prefix_needs_update(
    spec: &NetBoxPrefixSpec,
    existing: &netbox_client::Prefix,
    desired_tenant_id: Option<u64>,
    desired_site_id: Option<u64>,
    desired_vlan_id: Option<u32>,
    desired_role_id: Option<u64>,
    desired_status: &str,
) -> bool {
    // Compare tenant
    let existing_tenant_id = existing.tenant.as_ref().map(|t| t.id);
    if desired_tenant_id != existing_tenant_id {
        return true;
    }
    
    // Compare site
    let existing_site_id = existing.site.as_ref().map(|s| s.id);
    if desired_site_id != existing_site_id {
        return true;
    }
    
    // Compare description
    if spec.description.as_deref() != existing.description.as_deref() {
        return true;
    }
    
    // Compare status
    let existing_status = match existing.status {
        netbox_client::PrefixStatus::Active => "active",
        // ... other statuses
    };
    if desired_status != existing_status {
        return true;
    }
    
    false // No changes needed
}
```

### Step 2: Replace `check_existing()` with `check_and_update_existing()`

```rust
// OLD (broken):
match reconcile_helpers::check_existing(
    &self.netbox_client,
    netbox_id,
    &format!("NetBoxPrefix {}/{}", namespace, name),
    self.netbox_client.get_prefix(netbox_id),
).await {
    Ok(Some(resource)) => {
        // Resource exists - assumes up-to-date (WRONG!)
        Some(resource)
    }
    // ...
}

// NEW (working):
match reconcile_helpers::check_and_update_existing(
    &self.netbox_client,
    netbox_id,
    &format!("NetBoxPrefix {}/{}", namespace, name),
    self.netbox_client.get_prefix(netbox_id),
    |existing| Self::prefix_needs_update(
        &prefix_crd.spec,
        existing,
        tenant_id,
        site_id,
        vlan_id,
        role_id,
        &status_str,
    ),
    self.netbox_client.update_prefix(
        netbox_id,
        None, // Don't change prefix CIDR
        prefix_crd.spec.description.clone(),
        Some(status_str),
        None, // role - needs role_id resolution
        tenant_id,
        None, // tags
    ),
).await {
    Ok(Some(resource)) => {
        // Resource exists and is up-to-date (or was updated)
        Some(resource)
    }
    // ...
}
```

## Summary

- ✅ **2 reconcilers** (NetBoxSite, NetBoxPrefix) have proper update logic
- 🔴 **9 reconcilers** need update logic (critical/medium priority)
- 🟡 **6 reconcilers** have simple resources (low priority)

## Fixes Applied

### ✅ Fixed NetBoxPrefix Update Logic
- **File:** `controllers/netbox/src/reconciler/ipam/prefix.rs`
- **Change:** Added `prefix_needs_update()` function to compare spec with existing
- **Change:** Replaced `check_existing()` with `check_and_update_existing()`
- **Change:** Added `site_id` and `vlan_id` parameters to `update_prefix()` API method
- **Status:** ✅ **FIXED** - Prefix will now detect tenant changes and update NetBox

**Critical Fix Required:** NetBoxDevice, NetBoxVLAN need update logic to detect and apply tenant changes (and other field changes).

