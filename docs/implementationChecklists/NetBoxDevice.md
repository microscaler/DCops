# Implementation Checklist: NetBoxDevice

**CRD:** `NetBoxDevice`  
**Module:** `dcim/netbox_device.rs`  
**Reconciler:** `controllers/netbox/src/reconciler/dcim/device.rs`  
**Last Updated:** 2026-01-03

## CRD Spec Fields

### Required Fields
- [ ] `device_type: NetBoxResourceReference` - **Helper:** ⚠️ TODO - `compare_required_dependency_id()`
- [ ] `device_role: NetBoxResourceReference` - **Helper:** ⚠️ TODO - `compare_required_dependency_id()`
- [ ] `site: NetBoxResourceReference` - **Helper:** ⚠️ TODO - `compare_required_dependency_id()`
- [ ] `tenant: NetBoxResourceReference` - **Helper:** ⚠️ TODO - `compare_required_dependency_id()`
- [ ] `status: DeviceStatus` - **Helper:** ⚠️ TODO - `compare_string_field()` (converted to string)

### Optional Fields
- [ ] `name: Option<String>` - **Helper:** ⚠️ TODO - `compare_optional_string_field()`
- [ ] `location: Option<NetBoxResourceReference>` - **Helper:** ⚠️ TODO - `compare_optional_dependency_id()`
- [ ] `platform: Option<NetBoxResourceReference>` - **Helper:** ⚠️ TODO - `compare_optional_dependency_id()`
- [ ] `serial: Option<String>` - **Helper:** ⚠️ TODO - `compare_optional_string_field()`
- [ ] `asset_tag: Option<String>` - **Helper:** ⚠️ TODO - `compare_optional_string_field()`
- [ ] `primary_ip4: Option<PrimaryIPReference>` - **Helper:** ⚠️ TODO - Complex comparison needed
- [ ] `primary_ip6: Option<PrimaryIPReference>` - **Helper:** ⚠️ TODO - Complex comparison needed
- [ ] `description: Option<String>` - **Helper:** ⚠️ TODO - `compare_optional_string_field()`
- [ ] `comments: Option<String>` - **Helper:** ⚠️ TODO - `compare_optional_string_field()`

### Tag Fields
- [ ] `tags: Option<Vec<NetBoxResourceReference>>` - **Helper:** ✅ `tags_differ()` + `update_tags_if_differ()`

### Controller Config Fields (NOT checked - not NetBox fields)
- [x] `drift_detection: Option<bool>` - Controller config, not a NetBox field

## Implementation Status

### Drift Detection Function
- [ ] `device_needs_update()` function created
- [ ] All CRD spec fields are checked
- [ ] All NetBox model fields (that map to CRD) are checked
- [ ] All fields use reusable helpers
- [ ] No inline comparison code

### Helper Functions
- [x] All required helpers exist in `reconcile_helpers.rs`
- [x] All helpers are documented

### Integration
- [ ] Reconciler uses `device_needs_update()` function
- [x] Drift detection is enabled by default
- [ ] Drift detection respects `drift_detection` flag
- [ ] Updates are performed when drift is detected
- [ ] `UPDATED` events are emitted

### Testing
- [ ] Unit tests for `device_needs_update()` function
- [ ] Unit tests for all field comparisons
- [ ] Integration tests for drift detection
- [ ] Tests verify all fields are checked

## Notes

- **TODO:** Implement `device_needs_update()` function
- **TODO:** Add drift detection logic to `reconcile_netbox_device()`
- Complex resource with many dependencies
- Primary IP fields (`primary_ip4`, `primary_ip6`) need special handling
- All required helpers already exist

