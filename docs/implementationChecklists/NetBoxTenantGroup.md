# Implementation Checklist: NetBoxTenantGroup

**CRD:** `NetBoxTenantGroup`  
**Module:** `tenancy/netbox_tenant_group.rs`  
**Reconciler:** `controllers/netbox/src/reconciler/tenancy/tenant_group.rs`  
**Last Updated:** 2026-01-03

## CRD Spec Fields

### Required Fields
- [x] `name: String` - **Helper:** ✅ `compare_string_field()`
- [x] `slug: Option<String>` - **Helper:** ✅ `compare_slug_field()`

### Optional Fields
- [x] `description: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `comments: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `parent: Option<NetBoxResourceReference>` - **Helper:** ✅ `compare_optional_dependency_id()`

### Tag Fields
- [x] `tags: Option<Vec<NetBoxResourceReference>>` - **Helper:** ✅ `tags_differ()` + `update_tags_if_differ()`

### Controller Config Fields (NOT checked - not NetBox fields)
- [x] `drift_detection: Option<bool>` - Controller config, not a NetBox field

## NetBox Model Fields

### Read-Only Fields (NOT checked - read-only)
- [x] `id: u64` - Read-only, stored in status
- [x] `url: String` - Read-only, stored in status
- [x] `display: String` - Read-only, computed field
- [x] `created: String` - Read-only, timestamp
- [x] `last_updated: String` - Read-only, timestamp

### Managed Fields (MUST be checked)
- [x] `name: String` - Maps to CRD `name` ✅ Checked
- [x] `slug: String` - Maps to CRD `slug` ✅ Checked
- [x] `description: Option<String>` - Maps to CRD `description` ✅ Checked
- [x] `comments: Option<String>` - Maps to CRD `comments` ✅ Checked
- [x] `parent: Option<NestedTenantGroup>` - Maps to CRD `parent` ✅ Checked
- [x] `tags: Vec<NestedTag>` - Maps to CRD `tags` ✅ Checked via `tags_differ()`

## Field Mapping

| CRD Field | NetBox Field | Helper Used | Status | Notes |
|-----------|--------------|-------------|--------|-------|
| `name` | `name` | ✅ `compare_string_field()` | ✅ Checked | ✅ Using helper |
| `slug` | `slug` | ✅ `compare_slug_field()` | ✅ Checked | ✅ Using helper |
| `description` | `description` | ✅ `compare_optional_string_field()` | ✅ Checked | ✅ Using helper |
| `comments` | `comments` | ✅ `compare_optional_string_field()` | ✅ Checked | ✅ Using helper |
| `parent` | `parent` | ✅ `compare_optional_dependency_id()` | ✅ Checked | ✅ Using helper (hierarchical) |
| `tags` | `tags` | ✅ `tags_differ()` + `update_tags_if_differ()` | ✅ Checked | ✅ Using helper |

## Implementation Status

### Drift Detection Function
- [x] `tenant_group_needs_update()` function created
- [x] All CRD spec fields are checked
- [x] All NetBox model fields (that map to CRD) are checked
- [x] All fields use reusable helpers
- [x] No inline comparison code

### Helper Functions
- [x] All required helpers exist in `reconcile_helpers.rs`
- [x] All helpers are documented

### Integration
- [x] Reconciler uses `tenant_group_needs_update()` function
- [x] Drift detection is enabled by default
- [x] Drift detection respects `drift_detection` flag
- [x] Updates are performed when drift is detected
- [x] `UPDATED` events are emitted

### Testing
- [ ] Unit tests for `tenant_group_needs_update()` function
- [ ] Unit tests for all field comparisons
- [ ] Integration tests for drift detection
- [ ] Tests verify all fields are checked

## Notes

- Supports hierarchical organization via `parent` field
- All fields are properly checked using helpers

