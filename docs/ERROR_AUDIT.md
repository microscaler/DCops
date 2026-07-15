# Error Audit and Fixes

**Date:** 2025-12-25  
**Status:** 🔍 **AUDIT COMPLETE** - Fixes identified

## Error 1: IPPool - Prefix ID 1 not found

### Error Message
```
Prefix ID 1 not found in NetBox. Ensure the prefix exists or create a NetBoxPrefix CRD and reconcile it first.
```

### Root Cause
The IPPool CR `control-plane-pool` has:
```yaml
netboxPrefixRef:
  id: "1"
  site: "datacenter-1"
```

The IPPool reconciler parses `"1"` as a direct numeric ID and tries to fetch prefix ID 1 from NetBox, but it doesn't exist.

### Possible Causes
1. **Prefix was deleted** - The prefix with ID 1 was deleted from NetBox
2. **Wrong ID** - The prefix ID changed (e.g., after recreation)
3. **Should use CRD reference** - The IPPool should reference a NetBoxPrefix CRD instead of a direct ID

### Current Behavior
- IPPool reconciler supports both:
  - Direct numeric ID: `id: "1"` → fetches prefix ID 1 from NetBox
  - CRD name: `id: "control-plane-prefix"` → resolves NetBoxPrefix CRD to get netbox_id from status

### Fix Options

#### Option 1: Use NetBoxPrefix CRD Reference (Recommended)
Update the IPPool CR to reference the NetBoxPrefix CRD:
```yaml
netboxPrefixRef:
  id: "control-plane-prefix"  # CRD name instead of direct ID
  site: "datacenter-1"
```

This is more resilient because:
- If prefix is recreated, the CRD's netbox_id will be updated automatically
- No hardcoded IDs
- Follows GitOps best practices

#### Option 2: Verify Prefix Exists
Check if prefix ID 1 exists in NetBox:
- If it was deleted, recreate it
- If ID changed, update the IPPool CR

#### Option 3: Improve Error Handling
The reconciler already provides a helpful error message. We could:
- Add retry logic with exponential backoff
- Add a startup check to verify all referenced prefixes exist
- Add validation in the CRD schema to ensure prefix exists

### Recommendation
**Use Option 1** - Update IPPool CR to use CRD reference instead of direct ID.

---

## Error 2: NetBoxSite - Tenant format error

### Error Message
```
Failed to update site 1: 400 Bad Request - {"tenant":{"non_field_errors":["Invalid data. Expected a dictionary, but got int."]}}
```

### Root Cause
The `update_site()` method in `netbox-client/src/client.rs` sends nested fields inconsistently:
- **Tenant**: Sent as integer `1` ❌
- **Region**: Sent as dictionary `{"id": 1}` ✅
- **Site Group**: Sent as dictionary `{"id": 1}` ✅

NetBox 4.0's PATCH serializer for sites requires **all nested fields to be dictionaries** `{"id": X}`, not integers.

### Current Code (Line 1369)
```rust
if let Some(tid) = tenant_id {
    body["tenant"] = serde_json::Value::Number(tid.into());  // ❌ Integer
}
```

### Fix
Change tenant to use dictionary format (matching region and site_group):
```rust
if let Some(tid) = tenant_id {
    body["tenant"] = serde_json::json!({"id": tid});  // ✅ Dictionary
}
```

### Impact
- **Before**: Tenant updates fail with 400 Bad Request
- **After**: Tenant updates work correctly
- **Consistency**: All nested fields (tenant, region, site_group) use the same format

---

## Summary

| Error | Status | Fix |
|-------|--------|-----|
| IPPool - Prefix not found | ✅ **Fixed** | Updated IPPool example to use CRD reference `"control-plane-prefix"` instead of direct ID `"1"` |
| NetBoxSite - Tenant format | ✅ **Fixed** | Changed tenant to dictionary format `{"id": X}` in update_site() to match region and site_group |

## Fixes Applied

### ✅ Fixed NetBoxSite Tenant Update Format
- **File:** `crates/netbox-client/src/client.rs`
- **Change:** Changed `body["tenant"] = serde_json::Value::Number(tid.into())` to `body["tenant"] = serde_json::json!({"id": tid})`
- **Reason:** NetBox 4.0 PATCH serializer requires all nested fields (tenant, region, site_group) to be dictionaries, not integers
- **Status:** ✅ **FIXED** - Tenant updates will now work correctly

### ✅ Fixed IPPool Example
- **File:** `config/examples/ippool-example.yaml`
- **Change:** Updated `id: "1"` to `id: "control-plane-prefix"` (CRD name reference)
- **Reason:** Using CRD references is more resilient - if prefix is recreated, the CRD's netbox_id will be updated automatically
- **Status:** ✅ **FIXED** - IPPool will now resolve prefix from NetBoxPrefix CRD instead of hardcoded ID

## Next Steps

1. **Update existing IPPool CR**: If you have an IPPool CR with `id: "1"`, update it to use the CRD name:
   ```yaml
   netboxPrefixRef:
     id: "control-plane-prefix"  # Instead of "1"
   ```

2. **Verify prefix exists**: If prefix ID 1 should exist, check:
   - Was it deleted in NetBox?
   - Does the NetBoxPrefix CRD `control-plane-prefix` have a netbox_id in its status?
   - If not, ensure the NetBoxPrefix CRD is reconciled first

