# Trait System Design for Reconciliation

**Date:** 2025-12-25  
**Issue:** Code duplication across all reconcilers (drift detection, diffing, updates)

## Problem

We have 19 reconcilers, each with:
- Drift detection logic (check if resource exists, handle NotFound)
- Diffing logic (compare CR spec with NetBox resource)
- Update logic (call update_* if spec changed)
- Status update logic (update CR status after reconciliation)

This is a **code smell** - we're duplicating the same pattern 19 times.

## Solution: Helper Functions + Macro

Instead of a complex trait system (which has issues with Rust's type system), we use:

### 1. Helper Functions (`reconcile_helpers.rs`)

**`check_and_update_existing()`** - Generic drift detection and update:
```rust
pub async fn check_and_update_existing<FGet, FUpdate, FNeedsUpdate, Resource>(
    client: &NetBoxClient,
    netbox_id: u64,
    resource_name: &str,
    get_fn: FGet,           // Async function to get resource
    needs_update_fn: FNeedsUpdate,  // Function to check if update needed
    update_fn: FUpdate,     // Async function to update resource
) -> Result<Option<Resource>, ControllerError>
```

This handles:
- Getting existing resource from NetBox
- Checking if it needs updating (via closure)
- Updating if needed
- Detecting drift (NotFound)
- Returning appropriate errors

**`clear_status_on_drift()`** - Clear CR status when drift detected

### 2. Macro (Future Enhancement)

A macro could generate the full reconciliation boilerplate, but for now, helper functions are sufficient.

## Usage Example

### Before (Duplicated Code):
```rust
// Check if already created
if let Some(status) = &site_crd.status {
    if status.state == ResourceState::Created && status.netbox_id.is_some() {
        if let Some(netbox_id) = status.netbox_id {
            match self.netbox_client.get_site(netbox_id).await {
                Ok(existing) => {
                    if site_needs_update(&site_crd.spec, &existing, ...) {
                        match self.netbox_client.update_site(...).await {
                            Ok(updated) => { /* update status */ }
                            Err(e) => return Err(...),
                        }
                    } else {
                        return Ok(());
                    }
                }
                Err(NetBoxError::NotFound(_)) => {
                    // Clear status...
                }
                Err(e) => return Err(...),
            }
        }
    }
}
// ... create logic ...
```

### After (Using Helper):
```rust
let netbox_id = site_crd.status.as_ref().and_then(|s| s.netbox_id);
let netbox_site = if let Some(id) = netbox_id {
    match check_and_update_existing(
        &self.netbox_client,
        id,
        &format!("Site {}/{}", namespace, name),
        self.netbox_client.get_site(id),
        |existing| site_needs_update(&site_crd.spec, existing, tenant_id, region_id, site_group_id, &status_str),
        self.netbox_client.update_site(id, ...),
    ).await? {
        Some(resource) => resource,
        None => {
            // Drift detected - clear status and create
            clear_status_on_drift(&self.netbox_site_api, name, || {
                create_resource_status_patch(0, String::new(), ResourceState::Pending, Some("...".to_string()))
            }).await?;
            // Fall through to creation
            self.netbox_client.create_site(...).await?
        }
    }
} else {
    // Create new
    self.netbox_client.create_site(...).await?
};

// Update status
update_status(&self.netbox_site_api, name, &netbox_site, ResourceState::Created, None).await?;
```

## Benefits

1. **DRY (Don't Repeat Yourself)** - Common logic in one place
2. **Type-Safe** - Uses Rust's generics and closures
3. **Flexible** - Each reconciler can customize the diffing/update logic
4. **Maintainable** - Fix bugs once, applies to all reconcilers
5. **Testable** - Helper functions can be unit tested

## Migration Plan

1. ✅ Create `reconcile_helpers.rs` module
2. ✅ Refactor `reconcile_netbox_site()` to use helpers (as example)
3. ⏳ Refactor remaining reconcilers one by one
4. ⏳ Add macro if boilerplate is still too much

## Implementation Status

### Completed
- ✅ `reconcile_helpers.rs` module with `check_and_update_existing()` helper
- ✅ `NetBoxResource` trait for common NetBox resource types
- ✅ `reconcile_netbox_site()` refactored to use helper functions
- ✅ Drift detection pattern implemented
- ✅ Diffing and update pattern implemented

### In Progress
- ⏳ Refactoring remaining 18 reconcilers to use helper functions

### Helper Functions Available

1. **`check_and_update_existing()`** - Generic drift detection and update:
   - Checks if resource exists in NetBox (by ID from status)
   - Diffs and updates if needed (via closure)
   - Detects drift (NotFound) and signals for recreation
   - Returns `Ok(Some(resource))` if up-to-date or updated
   - Returns `Ok(None)` if deleted (drift detected)
   - Returns `Err(e)` for retryable errors

2. **`create_drift_status_patch()`** - Creates status patch for drift detection:
   - Sets `netboxId` to 0
   - Sets `state` to "Pending"
   - Sets error message

### Usage Pattern

The refactored `reconcile_netbox_site()` demonstrates the pattern:

```rust
// 1. Resolve references (tenant, region, site_group IDs)
let tenant_id = resolve_tenant_reference(...)?;
let region_id = resolve_region_reference(...)?;
let site_group_id = resolve_site_group_reference(...)?;

// 2. Check if already created - use helper for drift detection and diffing
let netbox_site = if let Some(netbox_id) = status.netbox_id {
    match reconcile_helpers::check_and_update_existing(
        &self.netbox_client,
        netbox_id,
        &format!("NetBoxSite {}/{}", namespace, name),
        self.netbox_client.get_site(netbox_id),
        |existing| Self::site_needs_update(...),
        self.netbox_client.update_site(...),
    ).await? {
        Some(resource) => resource,  // Exists and up-to-date
        None => {
            // Drift detected - clear status and fall through to creation
            clear_status_on_drift(...).await?;
            None
        }
    }
} else {
    None  // Need to create
};

// 3. Create if needed
let netbox_site = match netbox_site {
    Some(site) => site,  // Already exists
    None => {
        // Try idempotency fallback (query by name)
        // Then create if not found
        self.netbox_client.create_site(...).await?
    }
};

// 4. Update status
update_status(...).await?;
```

## Future Enhancements

1. **Macro for full reconciliation** - Generate entire reconcile function
2. **Trait for diffing** - `trait NetBoxDiffable { fn needs_update(&self, existing: &Self::NetBox) -> bool; }`
3. **Builder pattern** - `ReconciliationBuilder::new().with_drift_detection().with_diffing().build()`

