# Implementation Checklist: NetBoxInterface

**CRD:** `NetBoxInterface`  
**Module:** `dcim/netbox_interface.rs`  
**Reconciler:** `controllers/netbox/src/reconciler/dcim/interface.rs`  
**Last Updated:** 2026-01-03 (Drift detection completed) (Drift detection completed)

## CRD Spec Fields

### Required Fields
- [x] `device: String` - **Helper:** ✅ Direct comparison (device_id != existing.device.id)
- [x] `name: String` - **Helper:** ✅ `compare_string_field()`
- [x] `type: String` - **Helper:** ✅ `compare_string_field()`
- [x] `enabled: bool` - **Helper:** ✅ Direct comparison (`spec.enabled != existing.enabled`)

### Optional Fields
- [x] `mac_address: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `mtu: Option<u16>` - **Helper:** ✅ `compare_optional_numeric_field()`
- [x] `description: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `comments: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`

### Tag Fields
- [x] `tags: Option<Vec<NetBoxResourceReference>>` - **Helper:** ✅ `tags_differ()` + `update_tags_if_differ()`

### Controller Config Fields (NOT checked - not NetBox fields)
- [x] `drift_detection: Option<bool>` - Controller config, not a NetBox field

## Implementation Status

### Drift Detection Function
- [x] `interface_needs_update()` function created
- [x] All CRD spec fields are checked
- [x] All NetBox model fields (that map to CRD) are checked
- [x] All fields use reusable helpers
- [x] No inline comparison code (except direct comparisons for device_id and enabled)

### Helper Functions
- [x] All required helpers exist in `reconcile_helpers.rs`
- [x] All helpers are documented

### Integration
- [x] Reconciler uses `interface_needs_update()` function
- [x] Drift detection is enabled by default
- [x] Drift detection respects `drift_detection` flag
- [x] Updates are performed when drift is detected
- [x] `UPDATED` events are emitted
- [x] Device ID is resolved once at top level and reused

### Testing
- [ ] Unit tests for `interface_needs_update()` function
- [ ] Unit tests for all field comparisons
- [ ] Integration tests for drift detection
- [ ] Tests verify all fields are checked

## Notes

- ✅ **COMPLETED:** `interface_needs_update()` function implemented (2026-01-03)
- ✅ **COMPLETED:** Drift detection logic added to `reconcile_netbox_interface()` (2026-01-03)
- ✅ **COMPLETED:** Added tags support to `update_interface` method in NetBox client (2026-01-03)
- ✅ **COMPLETED:** Implemented `HasTags` for `Interface` model (2026-01-03)
- Note: device is String (name) in CRD but NestedDevice in NetBox model - resolved to device_id at top level and compared directly
- Note: device_id is not passed to update_interface (device cannot be changed after creation)
- All required helpers already exist

