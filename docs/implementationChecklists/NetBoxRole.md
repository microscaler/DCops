# Implementation Checklist: NetBoxRole

**CRD:** `NetBoxRole` (IPAM Role, not Extras Role)  
**Module:** `ipam/netbox_role.rs`  
**Reconciler:** `controllers/netbox/src/reconciler/extras.rs` (reconcile_netbox_role)  
**Last Updated:** 2026-01-03 (Drift detection completed)

## CRD Spec Fields

### Required Fields
- [x] `name: String` - **Helper:** ✅ `compare_string_field()`
- [x] `slug: Option<String>` - **Helper:** ✅ `compare_slug_field()`

### Optional Fields
- [x] `description: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `weight: Option<u16>` - **Helper:** ✅ `compare_optional_numeric_field()`
- [x] `comments: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`

### Tag Fields
- [x] `tags: Option<Vec<NetBoxResourceReference>>` - **Helper:** ✅ `tags_differ()` + `update_tags_if_differ()`

### Controller Config Fields (NOT checked - not NetBox fields)
- [x] `drift_detection: Option<bool>` - Controller config, not a NetBox field

## Implementation Status

### Drift Detection Function
- [x] `role_needs_update()` function created
- [x] All CRD spec fields are checked
- [x] All NetBox model fields (that map to CRD) are checked
- [x] All fields use reusable helpers
- [x] No inline comparison code

### Helper Functions
- [x] All required helpers exist in `reconcile_helpers.rs`
- [x] All helpers are documented

### Integration
- [x] Reconciler uses `role_needs_update()` function
- [x] Drift detection is enabled by default
- [x] Drift detection respects `drift_detection` flag
- [x] Updates are performed when drift is detected
- [x] `UPDATED` events are emitted

### Testing
- [ ] Unit tests for `role_needs_update()` function
- [ ] Unit tests for all field comparisons
- [ ] Integration tests for drift detection
- [ ] Tests verify all fields are checked

## Notes

- ✅ **COMPLETED:** `role_needs_update()` function implemented (2026-01-03)
- ✅ **COMPLETED:** Drift detection logic added to `reconcile_netbox_role()` (2026-01-03)
- Note: This is the Extras Role (not IPAM Role) - reconciler is in `extras.rs`
- All required helpers already exist

