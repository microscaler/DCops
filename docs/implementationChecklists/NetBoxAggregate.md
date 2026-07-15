# Implementation Checklist: NetBoxAggregate

**CRD:** `NetBoxAggregate`  
**Module:** `ipam/netbox_aggregate.rs`  
**Reconciler:** `controllers/netbox/src/reconciler/ipam/aggregate.rs`  
**Last Updated:** 2026-01-03 (Drift detection completed)

## CRD Spec Fields

### Required Fields
- [ ] `prefix: String` - **Helper:** ⚠️ Not checked (immutable after creation)

### Optional Fields
- [ ] `rir: Option<String>` - **Helper:** ⚠️ TODO - `compare_optional_string_field()` (or resolve to RIR ID)
- [ ] `date_allocated: Option<String>` - **Helper:** ⚠️ TODO - `compare_optional_string_field()`
- [ ] `description: Option<String>` - **Helper:** ⚠️ TODO - `compare_optional_string_field()`
- [ ] `comments: Option<String>` - **Helper:** ⚠️ TODO - `compare_optional_string_field()`

### Tag Fields
- [ ] `tags: Option<Vec<NetBoxResourceReference>>` - **Helper:** ✅ `tags_differ()` + `update_tags_if_differ()`

### Controller Config Fields (NOT checked - not NetBox fields)
- [x] `drift_detection: Option<bool>` - Controller config, not a NetBox field

## Implementation Status

### Drift Detection Function
- [ ] `aggregate_needs_update()` function created
- [ ] All CRD spec fields are checked (except immutable prefix)
- [ ] All NetBox model fields (that map to CRD) are checked
- [ ] All fields use reusable helpers
- [ ] No inline comparison code

### Helper Functions
- [x] All required helpers exist in `reconcile_helpers.rs`
- [x] All helpers are documented

### Integration
- [ ] Reconciler uses `aggregate_needs_update()` function
- [x] Drift detection is enabled by default
- [ ] Drift detection respects `drift_detection` flag
- [ ] Updates are performed when drift is detected
- [ ] `UPDATED` events are emitted

### Testing
- [ ] Unit tests for `aggregate_needs_update()` function
- [ ] Unit tests for all field comparisons
- [ ] Integration tests for drift detection
- [ ] Tests verify all fields are checked

## Notes

- **TODO:** Implement `aggregate_needs_update()` function
- **TODO:** Add drift detection logic to `reconcile_netbox_aggregate()`
- `prefix` is immutable after creation (not checked for drift)
- `rir` field may need special handling if it's a reference vs string
- All required helpers already exist

