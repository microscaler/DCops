# GitOps Compliance Analysis

## GitOps Principles

1. **Check if something exists, create it if not**
2. **If something exists, return with a status good or continue**
3. **If something can't be created due to conflict, then error**

## Current Reconciler Algorithm Analysis

### ✅ Principle 1: Check if exists, create if not

**Status**: ✅ **PARTIALLY COMPLIANT**

**Current Implementation**:
- Reconciler checks CRD status first (via `validate_status_and_drift`)
- If no status or invalid status, queries NetBox for existing resource by name/identifier
- If not found, attempts to create

**Example (Site Reconciler)**:
```rust
// Line 344-350: Query for existing before creating
let existing_site = match netbox_client.query_sites(
    &[("name", &site_crd.spec.name)],
    false,
).await {
    Ok(sites) => sites.first().cloned(),
    Err(_) => None
};

if let Some(existing) = existing_site {
    info!("Site {} already exists in NetBox (ID: {})", site_crd.spec.name, existing.id);
    existing
} else {
    // Create new site
    match netbox_client.create_site(...).await {
        Ok(created) => created,
        Err(e) => return Err(...),  // ❌ PROBLEM: Doesn't handle conflicts
    }
}
```

**Issue**: Query might miss the resource (race condition, different query criteria), then CREATE fails with conflict, but we don't handle it.

---

### ✅ Principle 2: If exists, return with status good

**Status**: ✅ **COMPLIANT**

**Current Implementation**:
- If resource exists and is up-to-date, returns `Ok(())` with correct status
- Uses `status_needs_update` helper to check if status update is needed
- Only updates status if it changed

**Example (Site Reconciler)**:
```rust
// Line 304-340: If resource exists and is up-to-date
Some(site) => {
    let needs_status_update = status_needs_update(...);
    if needs_status_update {
        // Update status
        return Ok(());
    } else {
        debug!("Already has correct status, skipping update");
        return Ok(());
    }
}
```

---

### ❌ Principle 3: Handle conflicts properly

**Status**: ❌ **NOT FULLY COMPLIANT**

**Current Implementation**:
- When CREATE fails, reconciler returns error immediately
- Does NOT check if the error is a conflict (resource already exists)
- Does NOT query for existing resource after conflict error
- Does NOT use existing resource if found

**Example (Site Reconciler - PROBLEM)**:
```rust
// Line 379-384: CREATE fails, just returns error
Err(e) => {
    let error_msg = format!("Failed to create site in NetBox: {}", e);
    error!("{}", error_msg);
    update_status_error(...);
    return Err(ControllerError::NetBox(e));  // ❌ Doesn't handle conflicts
}
```

**What Should Happen**:
```rust
Err(e) => {
    let error_str = format!("{}", e);
    // Check if error is a conflict (resource already exists)
    if error_str.contains("already exists") || 
       error_str.contains("duplicate") || 
       error_str.contains("unique constraint") ||
       error_str.contains("tenant with this name already exists") {
        
        // Query for existing resource
        match netbox_client.query_sites(&[("name", &site_crd.spec.name)], false).await {
            Ok(sites) => {
                if let Some(existing) = sites.first() {
                    info!("Site {} already exists in NetBox (ID: {}), using existing (idempotency)", 
                        site_crd.spec.name, existing.id);
                    return Ok(existing);  // ✅ Use existing resource
                }
            }
            Err(_) => {}
        }
    }
    // Only error if it's not a conflict or we can't find existing
    let error_msg = format!("Failed to create site in NetBox: {}", e);
    error!("{}", error_msg);
    update_status_error(...);
    return Err(ControllerError::NetBox(e));
}
```

**Good Example (Device Reconciler)**:
```rust
// Line 355-446: Device reconciler DOES handle conflicts
Err(e) => {
    let error_str = format!("{}", e);
    if error_str.contains("already exists") || error_str.contains("asset tag") {
        warn!("Device {} already exists in NetBox, attempting to retrieve it (idempotency)", ...);
        
        // Try to find existing device by asset_tag or name
        // ... query logic ...
        
        if let Some(found) = found_device {
            info!("Found existing device after conflict");
            found  // ✅ Use existing resource
        } else {
            // Can't find existing, return error
            return Err(...);
        }
    } else {
        // Real error, return it
        return Err(...);
    }
}
```

---

## Summary

| Principle | Status | Notes |
|-----------|--------|-------|
| **1. Check if exists, create if not** | ✅ Partial | Queries before creating, but doesn't handle CREATE conflicts |
| **2. If exists, return with status good** | ✅ Compliant | Correctly handles existing resources |
| **3. Handle conflicts properly** | ❌ **NON-COMPLIANT** | Most reconcilers don't handle CREATE conflict errors |

## Recommendations

### ✅ Shared Helper Created

**Status**: Created `is_conflict_error` helper in `reconcile_helpers.rs`

**Current State**: 
- ✅ `is_conflict_error` helper available for conflict detection
- ⚠️ Query pattern is still WET (duplicated across reconcilers)
- ⚠️ `handle_create_conflict` helper exists but unused due to Rust closure complexity

**Next Steps**:
1. Use `is_conflict_error` in all reconcilers (reduces WET for conflict detection)
2. Refactor query pattern into a simpler helper or macro (eliminates remaining WET)

### Pattern to Follow

**Current Pattern** (using shared helper):
```rust
use crate::reconcile_helpers::is_conflict_error;

match netbox_client.create_resource(...).await {
    Ok(created) => created,
    Err(e) => {
        if is_conflict_error(&e) {
            // Try query strategy 1
            // Try query strategy 2  
            // Try fallback query
            // Use found resource or error
        } else {
            // Not a conflict, return error
        }
    }
}
```

**Future Pattern** (with improved helper):
- Create macro or simpler helper to eliminate query duplication
- All reconcilers use same conflict handling pattern

### Reconcilers Needing Fix

- ❌ **Site Reconciler** - Missing conflict handling
- ❌ **Location Reconciler** - Missing conflict handling  
- ❌ **VLAN Reconciler** - Missing conflict handling
- ❌ **Prefix Reconciler** - Has conflict handling ✅
- ❌ **Device Reconciler** - Has conflict handling ✅
- ❌ **Platform Reconciler** - Missing conflict handling
- ❌ **Manufacturer Reconciler** - Missing conflict handling
- ❌ **DeviceType Reconciler** - Missing conflict handling
- ❌ **DeviceRole Reconciler** - Missing conflict handling
- ❌ **Region Reconciler** - Missing conflict handling
- ❌ **SiteGroup Reconciler** - Missing conflict handling
- ❌ **Tenant Reconciler** - Missing conflict handling
- ❌ **Role Reconciler** - Missing conflict handling
- ❌ **Tag Reconciler** - Missing conflict handling
- ❌ **Aggregate Reconciler** - Missing conflict handling
- ❌ **Interface Reconciler** - Missing conflict handling
- ❌ **MAC Address Reconciler** - Missing conflict handling

---

## Current Error Example

**NetBoxSite Error**:
```
400 Bad Request - {"tenant":{"name":["tenant with this name already exists."],"slug":["tenant with this slug already exists."]}}
```

**What Happens**:
1. Reconciler queries for site by name → Not found (or query fails)
2. Attempts CREATE → NetBox returns conflict error
3. Reconciler returns error immediately ❌
4. Status set to `Failed`
5. Retries continuously with same error

**What Should Happen** (GitOps Compliant):
1. Reconciler queries for site by name → Not found
2. Attempts CREATE → NetBox returns conflict error
3. Reconciler detects conflict error ✅
4. Queries NetBox again for existing site ✅
5. Finds existing site → Uses it ✅
6. Updates status to `Created` with existing site ID ✅
7. Reconciliation succeeds ✅

---

## Implementation Priority

1. **HIGH**: Fix Site Reconciler (blocking all dependent resources)
2. **HIGH**: Fix Location Reconciler (depends on site)
3. **HIGH**: Fix VLAN Reconciler (depends on site)
4. **MEDIUM**: Fix other reconcilers following same pattern
5. **LOW**: Create shared helper for conflict handling pattern

