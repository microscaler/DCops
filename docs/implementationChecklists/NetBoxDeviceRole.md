# Implementation Checklist: NetBoxDeviceRole

**CRD:** `NetBoxDeviceRole`  
**Module:** `dcim/netbox_device_role.rs`  
**Reconciler:** `controllers/netbox/src/reconciler/dcim/device_role.rs`  
**Last Updated:** 2026-01-03

## CRD Spec Fields

### Required Fields
- [ ] `name: String` - **Helper:** ⚠️ TODO - `compare_string_field()`
- [ ] `slug: Option<String>` - **Helper:** ⚠️ TODO - `compare_slug_field()`
- [ ] `vm_role: bool` - **Helper:** ⚠️ TODO - Direct comparison

### Optional Fields
- [ ] `color: Option<String>` - **Helper:** ⚠️ TODO - `compare_optional_string_field()`
- [ ] `description: Option<String>` - **Helper:** ⚠️ TODO - `compare_optional_string_field()`
- [ ] `comments: Option<String>` - **Helper:** ⚠️ TODO - `compare_optional_string_field()`

### Tag Fields
- [ ] `tags: Option<Vec<NetBoxResourceReference>>` - **Helper:** ✅ `tags_differ()` + `update_tags_if_differ()`

### Controller Config Fields (NOT checked - not NetBox fields)
- [x] `drift_detection: Option<bool>` - Controller config, not a NetBox field

## Implementation Status

### Drift Detection Function
- [ ] `device_role_needs_update()` function created
- [ ] All CRD spec fields are checked
- [ ] All NetBox model fields (that map to CRD) are checked
- [ ] All fields use reusable helpers
- [ ] No inline comparison code

### Helper Functions
- [x] All required helpers exist in `reconcile_helpers.rs`
- [x] All helpers are documented

### Integration
- [ ] Reconciler uses `device_role_needs_update()` function
- [x] Drift detection is enabled by default
- [ ] Drift detection respects `drift_detection` flag
- [ ] Updates are performed when drift is detected
- [ ] `UPDATED` events are emitted

### Testing
- [ ] Unit tests for `device_role_needs_update()` function
- [ ] Unit tests for all field comparisons
- [ ] Integration tests for drift detection
- [ ] Tests verify all fields are checked

## Notes

- **TODO:** Implement `device_role_needs_update()` function
- **TODO:** Add drift detection logic to `reconcile_netbox_device_role()`
- All required helpers already exist

