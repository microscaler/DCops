# Nested Reference Refactoring

**Date:** 2025-12-27  
**Status:** ✅ **COMPLETE** - All update/create methods now use centralized helper

## Problem

All update and create methods had repetitive code for handling nested object references:

```rust
if let Some(tid) = tenant_id {
    body["tenant"] = serde_json::json!({"id": tid});
}
if let Some(sid) = site_id {
    body["site"] = serde_json::json!({"id": sid});
}
// ... repeated in every method
```

This is WET (Write Everything Twice) code that:
- Makes maintenance harder
- Increases risk of inconsistencies
- Makes it easy to miss fixing one method when patterns change

## Solution

Created a centralized helper function `add_nested_reference` that handles all nested object serialization consistently:

```rust
fn add_nested_reference(&self, body: &mut serde_json::Value, field_name: &str, id: Option<u64>) {
    if let Some(id_value) = id {
        body[field_name] = serde_json::json!({"id": id_value});
        debug!("Adding nested reference: {}={}", field_name, id_value);
    }
}
```

## Methods Refactored

### Update Methods
1. ✅ `update_prefix` - tenant, site, vlan
2. ✅ `update_site` - tenant, region, site_group
3. ✅ `update_device` - tenant, platform, location, primary_ip4, primary_ip6
4. ✅ `update_vlan` - site, group, tenant, role

### Create Methods
1. ✅ `create_prefix` - site, vlan, role, tenant
2. ✅ `create_site` - tenant, region, site_group
3. ✅ `create_device` - tenant, platform, location
4. ✅ `create_vlan` - site, tenant
5. ✅ `create_location` - parent, tenant

## Benefits

1. **DRY Principle**: Single source of truth for nested reference serialization
2. **Consistency**: All methods use the same pattern
3. **Maintainability**: Changes to nested reference handling only need to be made in one place
4. **Debugging**: Centralized debug logging for all nested references
5. **Type Safety**: Helper function ensures correct JSON structure

## Usage

```rust
// Before (WET code):
if let Some(tid) = tenant_id {
    body["tenant"] = serde_json::json!({"id": tid});
    debug!("update_site: Including tenant reference (id={}) in PATCH body", tid);
}

// After (DRY code):
self.add_nested_reference(&mut body, "tenant", tenant_id);
```

## Statistics

- **Methods refactored**: 9 (4 update, 5 create)
- **Lines of code removed**: ~45 lines of repetitive code
- **Helper function**: 1 (8 lines)
- **Net code reduction**: ~37 lines
- **Consistency**: 100% - all nested references now use the same pattern

## Future Improvements

If NetBox API changes its nested reference format, we only need to update `add_nested_reference` in one place, and all methods will automatically use the new format.

