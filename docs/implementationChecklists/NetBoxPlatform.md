# Implementation Checklist: NetBoxPlatform

**CRD:** `NetBoxPlatform`  
**Module:** `dcim/netbox_platform.rs`  
**Reconciler:** `controllers/netbox/src/reconciler/dcim/platform.rs`  
**Last Updated:** 2026-01-03

## CRD Spec Fields

### Required Fields
- [x] `name: String` - **Helper:** ✅ `compare_string_field()`
- [x] `slug: Option<String>` - **Helper:** ✅ `compare_slug_field()`

### Optional Fields
- [x] `manufacturer: Option<NetBoxResourceReference>` - **Helper:** ✅ `compare_optional_dependency_id()`
- [x] `napalm_driver: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `napalm_args: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `description: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `comments: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`

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
- [x] `manufacturer: Option<NestedManufacturer>` - Maps to CRD `manufacturer` ✅ Checked
- [x] `napalm_driver: Option<String>` - Maps to CRD `napalm_driver` ✅ Checked
- [x] `napalm_args: Option<String>` - Maps to CRD `napalm_args` ✅ Checked
- [x] `description: Option<String>` - Maps to CRD `description` ✅ Checked
- [x] `comments: Option<String>` - Maps to CRD `comments` ✅ Checked
- [x] `tags: Vec<NestedTag>` - Maps to CRD `tags` ✅ Checked via `tags_differ()`

## Field Mapping

| CRD Field | NetBox Field | Helper Used | Status | Notes |
|-----------|--------------|-------------|--------|-------|
| `name` | `name` | ✅ `compare_string_field()` | ✅ Checked | ✅ Using helper |
| `slug` | `slug` | ✅ `compare_slug_field()` | ✅ Checked | ✅ Using helper |
| `manufacturer` | `manufacturer` | ✅ `compare_optional_dependency_id()` | ✅ Checked | ✅ Using helper |
| `napalm_driver` | `napalm_driver` | ✅ `compare_optional_string_field()` | ✅ Checked | ✅ Using helper |
| `napalm_args` | `napalm_args` | ✅ `compare_optional_string_field()` | ✅ Checked | ✅ Using helper |
| `description` | `description` | ✅ `compare_optional_string_field()` | ✅ Checked | ✅ Using helper |
| `comments` | `comments` | ✅ `compare_optional_string_field()` | ✅ Checked | ✅ Using helper |
| `tags` | `tags` | ✅ `tags_differ()` + `update_tags_if_differ()` | ✅ Checked | ✅ Using helper |

## Implementation Status

### Drift Detection Function
- [x] `platform_needs_update()` function created
- [x] All CRD spec fields are checked
- [x] All NetBox model fields (that map to CRD) are checked
- [x] All fields use reusable helpers
- [x] No inline comparison code

### Helper Functions
- [x] All required helpers exist in `reconcile_helpers.rs`
- [x] All helpers are documented

### Integration
- [x] Reconciler uses `platform_needs_update()` function
- [x] Drift detection is enabled by default
- [x] Drift detection respects `drift_detection` flag
- [x] Updates are performed when drift is detected
- [x] `UPDATED` events are emitted

### Testing
- [ ] Unit tests for `platform_needs_update()` function
- [ ] Unit tests for all field comparisons
- [ ] Integration tests for drift detection
- [ ] Tests verify all fields are checked

## Notes

- Has optional manufacturer dependency
- NAPALM fields (driver, args) are checked
- All fields are properly checked using helpers

