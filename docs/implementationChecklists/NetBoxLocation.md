# Implementation Checklist: NetBoxLocation

**CRD:** `NetBoxLocation`  
**Module:** `dcim/netbox_location.rs`  
**Reconciler:** `controllers/netbox/src/reconciler/dcim/location.rs`  
**Last Updated:** 2026-01-03 (Drift detection completed)

## CRD Spec Fields

### Required Fields
- [x] `name: String` - **Helper:** ✅ `compare_string_field()`
- [x] `slug: Option<String>` - **Helper:** ✅ `compare_slug_field()`
- [x] `site: NetBoxResourceReference` - **Helper:** ✅ `compare_required_dependency_id()`
- [x] `tenant: NetBoxResourceReference` - **Helper:** ✅ `compare_optional_dependency_id()` (Note: tenant is optional in NetBox model, but required in CRD)

### Optional Fields
- [x] `parent: Option<NetBoxResourceReference>` - **Helper:** ✅ `compare_optional_dependency_id()`
- [x] `facility: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `description: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `comments: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`

### Tag Fields
- [x] `tags: Option<Vec<NetBoxResourceReference>>` - **Helper:** ✅ `tags_differ()` + `update_tags_if_differ()`

### Controller Config Fields (NOT checked - not NetBox fields)
- [x] `drift_detection: Option<bool>` - Controller config, not a NetBox field

## Implementation Status

### Drift Detection Function
- [x] `location_needs_update()` function created
- [x] All CRD spec fields are checked
- [x] All NetBox model fields (that map to CRD) are checked
- [x] All fields use reusable helpers
- [x] No inline comparison code

### Helper Functions
- [x] All required helpers exist in `reconcile_helpers.rs`
- [x] All helpers are documented

### Integration
- [x] Reconciler uses `location_needs_update()` function
- [x] Drift detection is enabled by default
- [x] Drift detection respects `drift_detection` flag
- [x] Updates are performed when drift is detected
- [x] `UPDATED` events are emitted
- [x] Dependencies (site_id, tenant_id, parent_id) are resolved once at top level and reused

### Testing
- [ ] Unit tests for `location_needs_update()` function
- [ ] Unit tests for all field comparisons
- [ ] Integration tests for drift detection
- [ ] Tests verify all fields are checked

## Notes

- ✅ **COMPLETED:** `location_needs_update()` function implemented (2026-01-03)
- ✅ **COMPLETED:** Drift detection logic added to `reconcile_netbox_location()` (2026-01-03)
- ✅ **COMPLETED:** Optimized dependency resolution - resolved once at top level and reused (2026-01-03)
- ✅ **COMPLETED:** Updated Location model to include `tenant` and `facility` fields (2026-01-03)
- ✅ **COMPLETED:** Updated mock implementation to support tenant and facility (2026-01-03)
- Supports hierarchical organization via `parent` field
- All required helpers already exist
- Note: NetBox API doesn't support updating `site` field - location must be recreated to change site

