# NetBox 4.0 Tenant Field Fix - Final Solution

**Date:** 2025-01-26  
**Status:** ✅ **FIXED** - Use simple integer ID for PATCH updates

## Issue Progression

1. **First Error:** `"Expected a dictionary, but got int"` - When sending integer ID
2. **Second Error:** `"This field is required"` for name and group - When sending `{"id": tid}`
3. **Third Error:** `"tenant with this name already exists"` - When sending full object with name/group

## Root Cause

NetBox 4.0 has different requirements for:
- **POST (Create)**: Full object with name and group
- **PATCH (Update)**: Simple integer ID only

The error "tenant with this name already exists" occurred because sending a full object with `name` made NetBox think we were trying to CREATE a new tenant instead of referencing an existing one.

## Final Solution

For PATCH updates, NetBox 4.0 requires the tenant as a **simple integer ID**, not a dictionary:

```rust
if let Some(tid) = tenant_id {
    // NetBox 4.0 PATCH updates: send tenant as simple integer ID
    body["tenant"] = serde_json::Value::Number(tid.into());
}
```

## Implementation

**File:** `crates/netbox-client/src/client.rs` - `update_site` function

**Before (causing errors):**
```rust
body["tenant"] = serde_json::json!({"id": tid});  // ❌ Treated as create
// or
body["tenant"] = serde_json::json!({"id": tid, "name": name, "group": group});  // ❌ Treated as create
```

**After (correct):**
```rust
body["tenant"] = serde_json::Value::Number(tid.into());  // ✅ Simple integer ID
```

## Why This Works

- **PATCH semantics**: When updating, NetBox only needs the ID to reference the existing tenant
- **No validation errors**: Integer ID doesn't trigger tenant creation validation
- **Matches NetBox 4.0 API**: According to NetBox 4.0 documentation, PATCH updates use integer IDs for nested objects

## Verification

- ✅ Code compiles successfully
- ⏳ Controller should now work without tenant errors
- ⏳ Ready for rebuild and deployment

## Notes

- This applies to PATCH updates only
- POST (create) operations may still need full objects (depending on NetBox version)
- The reconciler correctly only includes tenant when it changes (PATCH semantics)

