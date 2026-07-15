# Implementation Checklist: NetBoxRIR

**CRD:** `NetBoxRIR`  
**Module:** `ipam/netbox_rir.rs`  
**Reconciler:** `controllers/netbox/src/reconciler/ipam/rir.rs`  
**Last Updated:** 2026-01-03 (Drift detection completed)

## CRD Spec Fields

### Required Fields
- [x] `name: String` - **Helper:** ✅ `compare_string_field()`
- [x] `slug: Option<String>` - **Helper:** ✅ `compare_slug_field()`

### Optional Fields
- [x] `description: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `is_private: Option<bool>` - **Helper:** ✅ Direct comparison (unwraps Option<bool> to bool)
- [x] `comments: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`

### Tag Fields
- [x] `tags: Option<Vec<NetBoxResourceReference>>` - **Helper:** ✅ `tags_differ()` + `update_tags_if_differ()`

### Controller Config Fields (NOT checked - not NetBox fields)
- [x] `drift_detection: Option<bool>` - Controller config, not a NetBox field

## Implementation Status

### Drift Detection Function
- [x] `rir_needs_update()` function created
- [x] All CRD spec fields are checked
- [x] All NetBox model fields (that map to CRD) are checked
- [x] All fields use reusable helpers
- [x] No inline comparison code (except direct bool comparison for is_private)

### Helper Functions
- [x] All required helpers exist in `reconcile_helpers.rs`
- [x] All helpers are documented

### Integration
- [x] Reconciler uses `rir_needs_update()` function
- [x] Drift detection is enabled by default
- [x] Drift detection respects `drift_detection` flag
- [x] Updates are performed when drift is detected
- [x] `UPDATED` events are emitted

### Testing
- [ ] Unit tests for `rir_needs_update()` function
- [ ] Unit tests for all field comparisons
- [ ] Integration tests for drift detection
- [ ] Tests verify all fields are checked

## Notes

- ✅ **COMPLETED:** `rir_needs_update()` function implemented (2026-01-03)
- ✅ **COMPLETED:** Drift detection logic added to `reconcile_netbox_rir()` (2026-01-03)
- Note: `is_private` is Option<bool> in CRD but bool in NetBox model - handled by unwrap_or(false)
- All required helpers already exist

