# Implementation Checklist: NetBoxPrefix

**CRD:** `NetBoxPrefix`  
**Module:** `ipam/netbox_prefix.rs`  
**Reconciler:** `controllers/netbox/src/reconciler/ipam/prefix.rs`  
**Last Updated:** 2026-01-03

## CRD Spec Fields

### Required Fields
- [x] `prefix: String` - **Helper:** ⚠️ Not checked (immutable after creation)
- [x] `tenant: NetBoxResourceReference` - **Helper:** ✅ `compare_required_dependency_id()`
- [x] `status: PrefixStatus` - **Helper:** ✅ `compare_string_field()` (converted to string)

### Optional Fields
- [x] `description: Option<String>` - **Helper:** ✅ `compare_string_field()` (converted from Option)
- [x] `site: Option<NetBoxResourceReference>` - **Helper:** ⚠️ Not checked (Prefix model doesn't have site field)
- [x] `aggregate: Option<NetBoxResourceReference>` - **Helper:** ⚠️ Not checked (not in needs_update)
- [x] `vlan: Option<NetBoxResourceReference>` - **Helper:** ✅ `compare_optional_dependency_id()`
- [x] `role: Option<NetBoxResourceReference>` - **Helper:** ✅ `compare_optional_dependency_id()`
- [x] `comments: Option<String>` - **Helper:** ⚠️ Not checked (not in needs_update)

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
- [x] `prefix: String` - Maps to CRD `prefix` ⚠️ Not checked (immutable after creation)
- [x] `tenant: Option<NestedTenant>` - Maps to CRD `tenant` ✅ Checked (required)
- [x] `vlan: Option<NestedVLAN>` - Maps to CRD `vlan` ✅ Checked
- [x] `role: Option<NestedRole>` - Maps to CRD `role` ✅ Checked
- [x] `status: PrefixStatus` - Maps to CRD `status` ✅ Checked (enum converted to string)
- [x] `description: String` - Maps to CRD `description` ✅ Checked (Note: NetBox model uses String, not Option)
- [x] `tags: Vec<NestedTag>` - Maps to CRD `tags` ✅ Checked via `tags_differ()`

## Field Mapping

| CRD Field | NetBox Field | Helper Used | Status | Notes |
|-----------|--------------|-------------|--------|-------|
| `prefix` | `prefix` | ⚠️ Not checked | ⚠️ Immutable | Immutable after creation |
| `tenant` | `tenant` | ✅ `compare_required_dependency_id()` | ✅ Checked | ✅ Using helper (required) |
| `vlan` | `vlan` | ✅ `compare_optional_dependency_id()` | ✅ Checked | ✅ Using helper |
| `role` | `role` | ✅ `compare_optional_dependency_id()` | ✅ Checked | ✅ Using helper |
| `status` | `status` | ✅ `compare_string_field()` | ✅ Checked | ✅ Using helper (enum converted to string) |
| `description` | `description` | ✅ `compare_string_field()` | ✅ Checked | ✅ Using helper (Option<String> → String conversion) |
| `site` | N/A | ⚠️ Not checked | ⚠️ Not in model | Prefix model doesn't have site field |
| `aggregate` | N/A | ⚠️ Not checked | ⚠️ Not in needs_update | Not currently checked |
| `comments` | N/A | ⚠️ Not checked | ⚠️ Not in needs_update | Not currently checked |
| `tags` | `tags` | ✅ `tags_differ()` + `update_tags_if_differ()` | ✅ Checked | ✅ Using helper |

## Implementation Status

### Drift Detection Function
- [x] `prefix_needs_update()` function created
- [ ] ⚠️ Some CRD spec fields are NOT checked (`site`, `aggregate`, `comments`)
- [x] All checked NetBox model fields are checked
- [x] All fields use reusable helpers
- [x] No inline comparison code

### Helper Functions
- [x] All required helpers exist in `reconcile_helpers.rs`
- [x] All helpers are documented

### Integration
- [x] Reconciler uses `prefix_needs_update()` function
- [x] Drift detection is enabled by default
- [x] Drift detection respects `drift_detection` flag
- [x] Updates are performed when drift is detected
- [x] `UPDATED` events are emitted

### Testing
- [ ] Unit tests for `prefix_needs_update()` function
- [ ] Unit tests for all field comparisons
- [ ] Integration tests for drift detection
- [ ] Tests verify all fields are checked

## Notes

- `prefix` is immutable after creation (not checked for drift)
- `site` field is not in NetBox Prefix model (cannot be checked)
- `aggregate` and `comments` fields are not currently checked in `prefix_needs_update()` - **TODO: Add these**
- NetBox model uses `String` for `description` (not `Option<String>`), so conversion needed
- All other checked fields are properly checked using helpers

