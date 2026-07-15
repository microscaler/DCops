# NetBox Client Refactoring Plan

**Date:** 2025-12-27  
**Status:** In Progress

## Goal

Eliminate all code duplication in `crates/netbox-client/src/client.rs` by:
1. Using existing `add_nested_reference` helper everywhere
2. Implementing and using new helper functions
3. Refactoring all 27 create/update methods

## Helper Functions Status

- ✅ `add_nested_reference` - Implemented, used in 9 methods, needs 6 more
- ✅ `add_required_nested_reference` - Implemented, needs to be used
- ✅ `add_nullable_nested_reference` - Implemented, needs to be used
- ✅ `generate_slug` - Implemented, needs to be used in 12 methods
- ✅ `add_optional_string_field` - Implemented, needs to be used in all methods
- ✅ `add_optional_string_field_owned` - Implemented, needs to be used
- ✅ `add_optional_number_field` - Implemented, needs to be used in 5 methods
- ✅ `add_optional_bool_field` - Implemented, needs to be used in 4 methods
- ✅ `add_optional_enum_field` - Implemented, needs to be used in 8 methods

## Refactoring Order

### Phase 1: Slug Generation (12 methods)
1. `create_tenant`
2. `create_tenant_group`
3. `create_site`
4. `create_region`
5. `create_site_group`
6. `create_location`
7. `create_device_role`
8. `create_manufacturer`
9. `create_platform`
10. `create_device_type`
11. `create_role`
12. `create_tag`

### Phase 2: Nested References (6 methods)
1. `create_aggregate` - Use `add_required_nested_reference` for `rir`
2. `create_tenant_group` - Use `add_nested_reference` for `parent`
3. `create_region` - Use `add_nested_reference` for `parent`
4. `create_site_group` - Use `add_nested_reference` for `parent`
5. `create_platform` - Use `add_nested_reference` for `manufacturer`
6. `create_device_type` - Use `add_nested_reference` for `manufacturer`
7. `create_interface` - Use `add_required_nested_reference` for `device`
8. `create_tenant` - Use `add_nullable_nested_reference` for `group`

### Phase 3: Optional Fields (All methods)
- Replace all `if let Some(value) = field { body["field"] = ... }` patterns
- Use appropriate helper based on field type

## Progress Tracking

- [ ] Phase 1: Slug generation (0/12)
- [ ] Phase 2: Nested references (0/8)
- [ ] Phase 3: Optional fields (0/27)
- [ ] Verification: Compilation
- [ ] Verification: Tests pass

