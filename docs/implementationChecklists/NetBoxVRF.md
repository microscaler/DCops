# Implementation Checklist: NetBoxVRF

**CRD:** `NetBoxVRF`  
**Module:** `ipam/netbox_vrf.rs`  
**Reconciler:** `controllers/netbox/src/reconciler/ipam/vrf.rs`  
**Last Updated:** 2026-01-03

## CRD Spec Fields

### Required Fields
- [x] `name: String` - **Helper:** ✅ `compare_string_field()`
- [x] `enforce_unique: bool` - **Helper:** ✅ Direct comparison (`!=`)

### Optional Fields
- [x] `rd: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `tenant: Option<NetBoxResourceReference>` - **Helper:** ✅ `compare_optional_dependency_id()`
- [x] `description: Option<String>` - **Helper:** ✅ `compare_string_field()` (converted from Option)
- [x] `comments: Option<String>` - **Helper:** ✅ `compare_string_field()` (converted from Option)
- [x] `import_targets: Option<Vec<NetBoxResourceReference>>` - **Helper:** ✅ Direct vector comparison (sorted)
- [x] `export_targets: Option<Vec<NetBoxResourceReference>>` - **Helper:** ✅ Direct vector comparison (sorted)

### Tag Fields
- [x] `tags: Option<Vec<NetBoxResourceReference>>` - **Helper:** ✅ `tags_differ()` + `update_tags_if_differ()`

### Controller Config Fields (NOT checked - not NetBox fields)
- [x] `drift_detection: Option<bool>` - Controller config, not a NetBox field

## NetBox Model Fields

### Read-Only Fields (NOT checked - read-only)
- [x] `id: u64` - Read-only, stored in status
- [x] `url: String` - Read-only, stored in status
- [x] `display: String` - Read-only, computed field
- [x] `created: String` - Read-only, timestamp
- [x] `last_updated: String` - Read-only, timestamp

### Managed Fields (MUST be checked)
- [x] `name: String` - Maps to CRD `name` ✅ Checked
- [x] `rd: Option<String>` - Maps to CRD `rd` ✅ Checked
- [x] `enforce_unique: bool` - Maps to CRD `enforce_unique` ✅ Checked
- [x] `tenant: Option<NestedTenant>` - Maps to CRD `tenant` ✅ Checked
- [x] `description: String` - Maps to CRD `description` ✅ Checked (Note: NetBox model uses String, not Option)
- [x] `comments: String` - Maps to CRD `comments` ✅ Checked (Note: NetBox model uses String, not Option)
- [x] `import_targets: Vec<NestedRouteTarget>` - Maps to CRD `import_targets` ✅ Checked (vector comparison)
- [x] `export_targets: Vec<NestedRouteTarget>` - Maps to CRD `export_targets` ✅ Checked (vector comparison)
- [x] `tags: Vec<NestedTag>` - Maps to CRD `tags` ✅ Checked via `tags_differ()`

## Field Mapping

| CRD Field | NetBox Field | Helper Used | Status | Notes |
|-----------|--------------|-------------|--------|-------|
| `name` | `name` | ✅ `compare_string_field()` | ✅ Checked | ✅ Using helper |
| `rd` | `rd` | ✅ `compare_optional_string_field()` | ✅ Checked | ✅ Using helper |
| `enforce_unique` | `enforce_unique` | ✅ Direct comparison | ✅ Checked | Direct boolean comparison |
| `tenant` | `tenant` | ✅ `compare_optional_dependency_id()` | ✅ Checked | ✅ Using helper |
| `description` | `description` | ✅ `compare_string_field()` | ✅ Checked | ✅ Using helper (Option<String> → String conversion) |
| `comments` | `comments` | ✅ `compare_string_field()` | ✅ Checked | ✅ Using helper (Option<String> → String conversion) |
| `import_targets` | `import_targets` | ✅ Direct vector comparison | ✅ Checked | Vectors sorted before comparison |
| `export_targets` | `export_targets` | ✅ Direct vector comparison | ✅ Checked | Vectors sorted before comparison |
| `tags` | `tags` | ✅ `tags_differ()` + `update_tags_if_differ()` | ✅ Checked | ✅ Using helper |

## Implementation Status

### Drift Detection Function
- [x] `vrf_needs_update()` function created
- [x] All CRD spec fields are checked
- [x] All NetBox model fields (that map to CRD) are checked
- [x] Most fields use reusable helpers
- [x] Vector comparisons use direct comparison (sorted)

### Helper Functions
- [x] All required helpers exist in `reconcile_helpers.rs`
- [x] All helpers are documented

### Integration
- [x] Reconciler uses `vrf_needs_update()` function
- [x] Drift detection is enabled by default
- [x] Drift detection respects `drift_detection` flag
- [x] Updates are performed when drift is detected
- [x] `UPDATED` events are emitted

### Testing
- [ ] Unit tests for `vrf_needs_update()` function
- [ ] Unit tests for all field comparisons
- [ ] Integration tests for drift detection
- [ ] Tests verify all fields are checked

## Notes

- NetBox model uses `String` for `description` and `comments` (not `Option<String>`), so conversion needed
- Route target vectors (`import_targets`, `export_targets`) are sorted before comparison to handle order differences
- Boolean field (`enforce_unique`) uses direct comparison
- All other fields are properly checked using helpers

