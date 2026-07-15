# NetBox 4.0 Tenant Field Fix

**Date:** 2025-01-26  
**Status:** ✅ **COMPLETE** - Full tenant object required for updates

## Issue

NetBox 4.0 requires the full tenant object structure when updating resources, not just `{"id": tid}`. The error was:

```
Failed to update site 1: 400 Bad Request - {"tenant":{"group":["This field is required."],"name":["This field is required."]}}
```

## Root Cause

NetBox 4.0 validates nested objects more strictly. When sending a tenant in an update, it requires:
- `id` - The tenant ID
- `name` - The tenant name (required)
- `group` - The tenant group (required, can be null)

## Solution

When updating a site (or other resource) with a tenant, we now:
1. Fetch the full tenant from NetBox using `get_tenant(tid)`
2. Extract the `name` and `group` fields
3. Construct the full tenant object: `{"id": tid, "name": name, "group": {...} or null}`

## Implementation

**File:** `crates/netbox-client/src/client.rs` - `update_site` function

```rust
if let Some(tid) = tenant_id {
    // NetBox 4.0 requires full tenant object with name and group for updates
    match self.get_tenant(tid).await {
        Ok(tenant) => {
            let mut tenant_obj = serde_json::json!({
                "id": tid,
                "name": tenant.name,
            });
            if let Some(group) = tenant.group {
                tenant_obj["group"] = serde_json::json!({
                    "id": group.id,
                    "name": group.name,
                });
            } else {
                tenant_obj["group"] = serde_json::Value::Null;
            }
            body["tenant"] = tenant_obj;
        }
        Err(e) => {
            // Fallback to ID only (may fail, but we try)
            debug!("Failed to fetch tenant {}: {}, using ID only", tid, e);
            body["tenant"] = serde_json::json!({"id": tid});
        }
    }
}
```

## Performance Impact

- **Extra API Call**: One additional `get_tenant()` call per site update when tenant changes
- **Mitigation**: Only happens when tenant actually changes (reconciler logic)
- **Acceptable**: Necessary for NetBox 4.0 compatibility

## Verification

- ✅ Code compiles successfully
- ⏳ Controller should now work without tenant validation errors
- ⏳ Ready for deployment and testing

## Notes

- This only applies when the tenant is being updated (changed)
- If tenant hasn't changed, we don't include it in the update body (PATCH semantics)
- The reconciler already handles this correctly - only passes tenant_id when it changed

