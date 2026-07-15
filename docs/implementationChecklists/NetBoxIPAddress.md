# Implementation Checklist: NetBoxIPAddress

**CRD:** `NetBoxIPAddress`  
**Module:** `ipam/netbox_ip_address.rs`  
**Reconciler:** `controllers/netbox/src/reconciler/ipam/ip_address.rs`  
**Last Updated:** 2026-01-03

## CRD Spec Fields

### Required Fields
- [x] `tenant: NetBoxResourceReference` - **Helper:** ✅ `compare_required_dependency_id()`
- [x] `status: IPAddressStatus` - **Helper:** ✅ `compare_string_field()` (converted to string)

### Optional Fields
- [x] `address: Option<String>` - **Helper:** ⚠️ Not checked (immutable after creation)
- [x] `ip_range: Option<NetBoxResourceReference>` - **Helper:** ⚠️ Not checked (not in needs_update)
- [x] `vrf: Option<NetBoxResourceReference>` - **Helper:** ⚠️ Not checked (not in needs_update)
- [x] `vlan: Option<NetBoxResourceReference>` - **Helper:** ⚠️ Not checked (not in model response)
- [x] `role: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `dns_name: Option<String>` - **Helper:** ✅ `compare_string_field()` (converted from Option)
- [x] `description: Option<String>` - **Helper:** ✅ `compare_string_field()` (converted from Option)
- [x] `comments: Option<String>` - **Helper:** ✅ `compare_string_field()` (converted from Option)
- [x] `mac_address: Option<String>` - **Helper:** ⚠️ Not checked (used for interface resolution only)
- [x] `interface: Option<NetBoxResourceReference>` - **Helper:** ⚠️ Not checked (not in needs_update)

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
- [x] `address: String` - Maps to CRD `address` ⚠️ Not checked (immutable after creation)
- [x] `tenant: Option<NestedTenant>` - Maps to CRD `tenant` ✅ Checked (required)
- [x] `role: Option<String>` - Maps to CRD `role` ✅ Checked
- [x] `dns_name: String` - Maps to CRD `dns_name` ✅ Checked (Note: NetBox model uses String, not Option)
- [x] `description: String` - Maps to CRD `description` ✅ Checked (Note: NetBox model uses String, not Option)
- [x] `comments: String` - Maps to CRD `comments` ✅ Checked (Note: NetBox model uses String, not Option)
- [x] `status: IPAddressStatus` - Maps to CRD `status` ✅ Checked (enum converted to string)
- [x] `tags: Vec<NestedTag>` - Maps to CRD `tags` ✅ Checked via `tags_differ()`

## Field Mapping

| CRD Field | NetBox Field | Helper Used | Status | Notes |
|-----------|--------------|-------------|--------|-------|
| `address` | `address` | ⚠️ Not checked | ⚠️ Immutable | Immutable after creation |
| `tenant` | `tenant` | ✅ `compare_required_dependency_id()` | ✅ Checked | ✅ Using helper (required) |
| `role` | `role` | ✅ `compare_optional_string_field()` | ✅ Checked | ✅ Using helper |
| `dns_name` | `dns_name` | ✅ `compare_string_field()` | ✅ Checked | ✅ Using helper (Option<String> → String conversion) |
| `description` | `description` | ✅ `compare_string_field()` | ✅ Checked | ✅ Using helper (Option<String> → String conversion) |
| `comments` | `comments` | ✅ `compare_string_field()` | ✅ Checked | ✅ Using helper (Option<String> → String conversion) |
| `status` | `status` | ✅ `compare_string_field()` | ✅ Checked | ✅ Using helper (enum converted to string) |
| `ip_range` | N/A | ⚠️ Not checked | ⚠️ Not in needs_update | Not currently checked |
| `vrf` | N/A | ⚠️ Not checked | ⚠️ Not in needs_update | Not currently checked |
| `vlan` | N/A | ⚠️ Not checked | ⚠️ Not in model | Not in NetBox model response |
| `mac_address` | N/A | ⚠️ Not checked | ⚠️ Used for resolution | Used for interface resolution only |
| `interface` | N/A | ⚠️ Not checked | ⚠️ Not in needs_update | Not currently checked |
| `tags` | `tags` | ✅ `tags_differ()` + `update_tags_if_differ()` | ✅ Checked | ✅ Using helper |

## Implementation Status

### Drift Detection Function
- [x] `ip_address_needs_update()` function created
- [ ] ⚠️ Some CRD spec fields are NOT checked (`ip_range`, `vrf`, `vlan`, `mac_address`, `interface`)
- [x] All checked NetBox model fields are checked
- [x] All fields use reusable helpers
- [x] No inline comparison code

### Helper Functions
- [x] All required helpers exist in `reconcile_helpers.rs`
- [x] All helpers are documented

### Integration
- [x] Reconciler uses `ip_address_needs_update()` function
- [x] Drift detection is enabled by default
- [x] Drift detection respects `drift_detection` flag
- [x] Updates are performed when drift is detected
- [x] `UPDATED` events are emitted

### Testing
- [ ] Unit tests for `ip_address_needs_update()` function
- [ ] Unit tests for all field comparisons
- [ ] Integration tests for drift detection
- [ ] Tests verify all fields are checked

## Notes

- `address` is immutable after creation (not checked for drift)
- `ip_range`, `vrf`, `vlan`, `mac_address`, and `interface` fields are not currently checked in `ip_address_needs_update()` - **TODO: Add these if they should be checked**
- NetBox model uses `String` for `dns_name`, `description`, and `comments` (not `Option<String>`), so conversion needed
- `vlan` is not in NetBox model response (cannot be checked from existing resource)
- All other checked fields are properly checked using helpers

