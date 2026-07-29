# Implementation Checklist: NetBoxMACAddress

**CRD:** `NetBoxMACAddress`  
**Module:** `dcim/netbox_mac_address.rs`  
**Reconciler:** `controllers/netbox/src/reconciler/dcim/mac_address.rs`  
**Last Updated:** 2026-01-03

## CRD Spec Fields

### Required Fields
- [ ] `mac_address: String` - **Helper:** ⚠️ TODO - `compare_string_field()` (may need normalization)
- [ ] `interface: String` - **Helper:** ⚠️ TODO - `compare_required_dependency_id()` (resolved to interface ID)

### Optional Fields
- [ ] `description: Option<String>` - **Helper:** ⚠️ TODO - `compare_optional_string_field()`
- [ ] `comments: Option<String>` - **Helper:** ⚠️ TODO - `compare_optional_string_field()`

### Tag Fields
- [ ] `tags: Option<Vec<NetBoxResourceReference>>` - **Helper:** ✅ `tags_differ()` + `update_tags_if_differ()`

### Controller Config Fields (NOT checked - not NetBox fields)
- [x] `drift_detection: Option<bool>` - Controller config, not a NetBox field

## Implementation Status

### Drift Detection Function
- [ ] `mac_address_needs_update()` function created
- [ ] All CRD spec fields are checked
- [ ] All NetBox model fields (that map to CRD) are checked
- [ ] All fields use reusable helpers
- [ ] No inline comparison code

### Helper Functions
- [x] All required helpers exist in `reconcile_helpers.rs`
- [x] All helpers are documented

### Integration
- [ ] Reconciler uses `mac_address_needs_update()` function
- [x] Drift detection is enabled by default
- [ ] Drift detection respects `drift_detection` flag
- [ ] Updates are performed when drift is detected
- [ ] `UPDATED` events are emitted

### Testing
- [ ] Unit tests for `mac_address_needs_update()` function
- [ ] Unit tests for all field comparisons
- [ ] Integration tests for drift detection
- [ ] Tests verify all fields are checked

## Notes

- **TODO:** Implement `mac_address_needs_update()` function
- **TODO:** Add drift detection logic to `reconcile_netbox_mac_address()`
- MAC address may need normalization (format: "aa:bb:cc:dd:ee:ff" vs "aa-bb-cc-dd-ee-ff")
- All required helpers already exist

