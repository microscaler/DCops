# Implementation Checklist: NetBoxVLAN

**CRD:** `NetBoxVLAN`  
**Module:** `ipam/netbox_vlan.rs`  
**Reconciler:** `controllers/netbox/src/reconciler/dcim/vlan.rs`  
**Last Updated:** 2026-01-03 (Drift detection completed)

## CRD Spec Fields

### Required Fields
- [x] `vid: u16` - **Helper:** ✅ Direct comparison (`spec.vid != existing.vid`)
- [x] `name: String` - **Helper:** ✅ `compare_string_field()`
- [x] `tenant: NetBoxResourceReference` - **Helper:** ✅ `compare_optional_dependency_id()` (Note: tenant is optional in NetBox model, but required in CRD)
- [x] `status: VlanStatus` - **Helper:** ✅ `compare_enum_field()` (with enum conversion)

### Optional Fields
- [x] `site: Option<NetBoxResourceReference>` - **Helper:** ✅ `compare_optional_dependency_id()`
- [x] `group: Option<NetBoxResourceReference>` - **Helper:** ✅ `compare_optional_dependency_id()` (Note: VLAN group CRD not yet implemented)
- [x] `role: Option<NetBoxResourceReference>` - **Helper:** ✅ `compare_optional_dependency_id()`
- [x] `description: Option<String>` - **Helper:** ✅ `compare_optional_string_field()` (Note: String in NetBox model, Option<String> in CRD)
- [x] `comments: Option<String>` - **Helper:** ✅ `compare_optional_string_field()` (Note: String in NetBox model, Option<String> in CRD)

### Tag Fields
- [x] `tags: Option<Vec<NetBoxResourceReference>>` - **Helper:** ✅ `tags_differ()` + `update_tags_if_differ()`

### Controller Config Fields (NOT checked - not NetBox fields)
- [x] `drift_detection: Option<bool>` - Controller config, not a NetBox field

## Implementation Status

### Drift Detection Function
- [x] `vlan_needs_update()` function created
- [x] All CRD spec fields are checked
- [x] All NetBox model fields (that map to CRD) are checked
- [x] All fields use reusable helpers
- [x] No inline comparison code (except direct vid comparison)

### Helper Functions
- [x] All required helpers exist in `reconcile_helpers.rs`
- [x] All helpers are documented

### Integration
- [x] Reconciler uses `vlan_needs_update()` function
- [x] Drift detection is enabled by default
- [x] Drift detection respects `drift_detection` flag
- [x] Updates are performed when drift is detected
- [x] `UPDATED` events are emitted
- [x] Dependencies (site_id, tenant_id, role_id, group_id) are resolved once at top level and reused

### Testing
- [ ] Unit tests for `vlan_needs_update()` function
- [ ] Unit tests for all field comparisons
- [ ] Integration tests for drift detection
- [ ] Tests verify all fields are checked

## Notes

- ✅ **COMPLETED:** `vlan_needs_update()` function implemented (2026-01-03)
- ✅ **COMPLETED:** Drift detection logic added to `reconcile_netbox_vlan()` (2026-01-03)
- ✅ **COMPLETED:** VlanStatus enum conversion between CRD and NetBox model (2026-01-03)
- ✅ **COMPLETED:** Optimized dependency resolution - resolved once at top level and reused (2026-01-03)
- Note: VLAN group CRD is not yet implemented, so group_id is set to None for now
- Note: description and comments are String in NetBox model but Option<String> in CRD - handled by wrapping in Some()
- All required helpers already exist

