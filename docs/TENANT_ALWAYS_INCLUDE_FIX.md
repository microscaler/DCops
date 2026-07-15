# Fix: Always Include Tenant in PATCH Requests

## Problem

NetBox 4.0 was returning validation errors when we tried to update a Site:
```json
{"tenant":{"name":["This field cannot be blank."],"slug":["This field cannot be blank."]}}
```

## Root Cause Analysis

After analysis, we discovered that:
1. The CRD **does** have a tenant reference
2. The tenant **does** exist in NetBox (ID: 1)
3. We were only including tenant in PATCH requests if it **changed**
4. NetBox 4.0 seems to require tenant to be **always present** in PATCH requests, even if unchanged

## Solution

Changed the logic to **always include tenant/region/site_group** in PATCH requests if they exist, rather than only including them when changed.

### Before
```rust
let update_tenant_id = if tenant_id != existing_tenant_id {
    tenant_id // Only include if changed
} else {
    None // Don't include if unchanged
};
```

### After
```rust
// ALWAYS include tenant/region/site_group in PATCH requests
// NetBox 4.0 seems to require these fields to be present even if unchanged
let update_tenant_id = tenant_id; // Always include if we have a tenant_id
let update_region_id = region_id; // Always include if we have a region_id
let update_site_group_id = site_group_id; // Always include if we have a site_group_id
```

## Rationale

NetBox 4.0's nested serializer validation appears to require nested fields to be present in PATCH requests, even if they haven't changed. When we omit them, NetBox may be trying to validate them as empty/blank, causing validation errors.

By always including them (when they exist), we ensure NetBox receives the complete object state and can properly validate it.

## Files Changed

- `controllers/netbox/src/reconciler/dcim/site.rs` - Always include tenant/region/site_group in updates

