# Implementation Checklist: NetBoxTag

**CRD:** `NetBoxTag`  
**Module:** `extras/netbox_tag.rs`  
**Reconciler:** `controllers/netbox/src/reconciler/extras.rs` (reconcile_netbox_tag)  
**Last Updated:** 2026-01-03 (Drift detection completed) (Drift detection completed)

## CRD Spec Fields

### Required Fields
- [x] `name: String` - **Helper:** ✅ `compare_string_field()`
- [x] `slug: Option<String>` - **Helper:** ✅ `compare_slug_field()`

### Optional Fields
- [x] `color: Option<String>` - **Helper:** ✅ `compare_optional_string_field()` (Note: String in NetBox model, Option<String> in CRD)
- [x] `description: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `comments: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `tenant: Option<NetBoxResourceReference>` - **Helper:** ✅ Not checked (used for token resolution only)

### Tag Fields
- [x] N/A - Tags don't have tags (self-referential)

### Controller Config Fields (NOT checked - not NetBox fields)
- [x] `drift_detection: Option<bool>` - Controller config, not a NetBox field

## Implementation Status

### Drift Detection Function
- [x] `tag_needs_update()` function created
- [x] All CRD spec fields are checked
- [x] All NetBox model fields (that map to CRD) are checked
- [x] All fields use reusable helpers
- [x] No inline comparison code

### Helper Functions
- [x] All required helpers exist in `reconcile_helpers.rs`
- [x] All helpers are documented

### Integration
- [x] Reconciler uses `tag_needs_update()` function
- [x] Drift detection is enabled by default
- [x] Drift detection respects `drift_detection` flag
- [x] Updates are performed when drift is detected
- [x] `UPDATED` events are emitted

### Testing
- [ ] Unit tests for `tag_needs_update()` function
- [ ] Unit tests for all field comparisons
- [ ] Integration tests for drift detection
- [ ] Tests verify all fields are checked

## Notes

- ✅ **COMPLETED:** `tag_needs_update()` function implemented (2026-01-03)
- ✅ **COMPLETED:** Drift detection logic added to `reconcile_netbox_tag()` (2026-01-03)
- ✅ **COMPLETED:** Added `update_tag` method to NetBox client (2026-01-03)
- Note: color is String in NetBox model but Option<String> in CRD - handled by wrapping in Some()
- `tenant` field is used for token resolution only, not stored in NetBox (tags are global)
- All required helpers already exist

