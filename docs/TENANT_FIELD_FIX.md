# Tenant Field Format Fix for NetBox 4.0

**Date:** 2025-01-26  
**Status:** ✅ **COMPLETE** - All tenant fields fixed

## Issue

NetBox 4.0 requires nested objects (like `tenant`, `region`, `site_group`) to be sent as dictionaries with an `id` field, not as plain integers. This was causing 400 Bad Request errors:

```
Failed to update site 1: 400 Bad Request - {"tenant":{"non_field_errors":["Invalid data. Expected a dictionary, but got int."]}}
```

## Root Cause

Multiple API methods were sending tenant (and some other nested fields) as integers:
```rust
body["tenant"] = serde_json::Value::Number(tid.into());  // ❌ Wrong for NetBox 4.0
```

## Fix Applied

Changed all tenant fields to use dictionary format:
```rust
body["tenant"] = serde_json::json!({"id": tid});  // ✅ Correct for NetBox 4.0
```

## Functions Fixed

### Site Operations
1. ✅ `create_site` - Fixed tenant, region, site_group fields
2. ✅ `update_site` - Fixed tenant field (region and site_group were already correct)

### Prefix Operations  
3. ✅ `create_prefix` - Fixed tenant field
4. ✅ `update_prefix` - Fixed tenant field

### Device Operations
5. ✅ `create_device` - Fixed tenant field
6. ✅ `update_device` - Fixed tenant field

### VLAN Operations
7. ✅ `create_vlan` - Fixed tenant field
8. ✅ `update_vlan` - Fixed tenant field

## Verification

- ✅ All code compiles successfully
- ✅ No remaining `body["tenant"] = Number` patterns found
- ⏳ Controller should now work without tenant-related 400 errors

## Next Steps

1. Rebuild controller binary with fixes
2. Deploy and verify controller works without errors
3. Continue with iterative test implementation

## Notes

- NetBox 4.0 is stricter about nested object formats than previous versions
- All nested fields (tenant, region, site_group, etc.) should use `{"id": value}` format
- This matches the format already used for region and site_group in update_site

