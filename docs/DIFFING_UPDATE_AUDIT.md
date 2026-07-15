# Diffing and Update Audit

**Date:** 2025-12-25  
**Issue:** Reconcilers don't perform diffing or updates - they only create resources

## Problem Statement

When a CR spec is updated (e.g., adding tenant, region, etc. to NetBoxSite), the controller:
1. Checks if the resource exists in NetBox
2. If it exists, returns `Ok(())` immediately
3. **Never checks if the spec changed**
4. **Never updates NetBox to match the new spec**

This is a fundamental design flaw affecting **ALL reconcilers**.

## Root Cause

### Current Pattern (BROKEN)
```rust
// Check if already created
if let Some(status) = &resource_crd.status {
    if status.state == Created && status.netbox_id.is_some() {
        match self.netbox_client.get_resource(netbox_id).await {
            Ok(_) => {
                info!("Resource already created in NetBox (ID: {})", netbox_id);
                return Ok(()); // ❌ RETURNS WITHOUT CHECKING IF SPEC CHANGED
            }
            // ...
        }
    }
}
// ... create logic ...
```

### Required Pattern (FIXED)
```rust
// Check if already created AND verify spec matches
if let Some(status) = &resource_crd.status {
    if status.state == Created && status.netbox_id.is_some() {
        if let Some(netbox_id) = status.netbox_id {
            match self.netbox_client.get_resource(netbox_id).await {
                Ok(existing) => {
                    // ✅ DIFF: Compare CR spec with NetBox resource
                    if needs_update(&resource_crd.spec, &existing) {
                        info!("Resource spec changed, updating in NetBox");
                        match self.netbox_client.update_resource(netbox_id, ...).await {
                            Ok(updated) => {
                                // Update status with new resource data
                                return Ok(());
                            }
                            Err(e) => return Err(ControllerError::NetBox(e)),
                        }
                    } else {
                        info!("Resource already created and up-to-date in NetBox (ID: {})", netbox_id);
                        return Ok(());
                    }
                }
                Err(NetBoxError::NotFound(_)) => {
                    // Drift detected - clear status and recreate
                    // ...
                }
                // ...
            }
        }
    }
}
// ... create logic ...
```

## Affected Reconcilers

**ALL reconcilers** need this fix:
- [ ] NetBoxPrefix
- [ ] NetBoxTenant
- [ ] NetBoxSite ⚠️ **User reported issue**
- [ ] NetBoxRole
- [ ] NetBoxTag
- [ ] NetBoxAggregate
- [ ] NetBoxDeviceRole
- [ ] NetBoxManufacturer
- [ ] NetBoxPlatform
- [ ] NetBoxDeviceType
- [ ] NetBoxDevice
- [ ] NetBoxInterface
- [ ] NetBoxMACAddress
- [ ] NetBoxVLAN
- [ ] NetBoxRegion
- [ ] NetBoxSiteGroup
- [ ] NetBoxLocation

## Implementation Strategy

### Step 1: Create Diff Helper Functions

For each resource type, create a helper function that compares CR spec with NetBox resource:

```rust
fn site_needs_update(spec: &NetBoxSiteSpec, existing: &Site) -> bool {
    // Compare all fields
    if spec.name != existing.name { return true; }
    if spec.slug.as_deref() != Some(&existing.slug) { return true; }
    if spec.description.as_deref() != existing.description.as_deref() { return true; }
    // ... compare all fields
    false
}
```

### Step 2: Update Reconciliation Logic

Replace early return with diffing and update logic:

1. Get existing resource from NetBox
2. Compare spec with existing resource
3. If different, call `update_*` method
4. Update CR status with new resource data
5. If same, return early

### Step 3: Handle Reference Resolution

When diffing, need to resolve references (tenant, region, etc.) to IDs for comparison:
- CR spec has: `tenant: { kind: "NetBoxTenant", name: "datacenter-tenant" }`
- NetBox has: `tenant: { id: 1, name: "datacenter-tenant" }`
- Need to resolve CR reference to ID before comparing

## NetBox Client Update Methods

Check which update methods exist:
- ✅ `update_prefix` - exists
- ✅ `update_device` - exists
- ✅ `update_interface` - exists
- ✅ `update_vlan` - exists
- ❓ `update_site` - need to check
- ❓ `update_tenant` - need to check
- ❓ Others - need to audit

## Testing Strategy

1. Create a resource via CR
2. Verify it exists in NetBox
3. Update the CR spec (e.g., add tenant, change description)
4. Wait for reconciliation
5. Verify NetBox resource was updated to match new spec
6. Verify CR status reflects the update

## Priority

**CRITICAL** - This is a fundamental flaw that makes the controller non-functional for updates. Users expect GitOps behavior where CR changes are reflected in NetBox.

