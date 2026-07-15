# Implementation Checklist: NetBoxSite

**CRD:** `NetBoxSite`  
**Module:** `dcim/netbox_site.rs`  
**Reconciler:** `controllers/netbox/src/reconciler/dcim/site.rs`  
**Last Updated:** 2026-01-03

## CRD Spec Fields

### Required Fields
- [x] `name: String` - **Helper:** ✅ `compare_string_field()`
- [x] `slug: Option<String>` - **Helper:** ✅ `compare_slug_field()`
- [x] `tenant: NetBoxResourceReference` - **Helper:** ✅ `compare_required_dependency_id()`
- [x] `status: SiteStatus` - **Helper:** ✅ `compare_string_field()` (converted to string)

### Optional Fields
- [x] `description: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `physical_address: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `shipping_address: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `latitude: Option<f64>` - **Helper:** ✅ `compare_optional_numeric_field()`
- [x] `longitude: Option<f64>` - **Helper:** ✅ `compare_optional_numeric_field()`
- [x] `region: Option<NetBoxResourceReference>` - **Helper:** ✅ `compare_optional_dependency_id()`
- [x] `site_group: Option<NetBoxResourceReference>` - **Helper:** ✅ `compare_optional_dependency_id()`
- [x] `facility: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `time_zone: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`
- [x] `comments: Option<String>` - **Helper:** ✅ `compare_optional_string_field()`

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
- [x] `slug: String` - Maps to CRD `slug` ✅ Checked
- [x] `description: Option<String>` - Maps to CRD `description` ✅ Checked
- [x] `physical_address: Option<String>` - Maps to CRD `physical_address` ✅ Checked
- [x] `shipping_address: Option<String>` - Maps to CRD `shipping_address` ✅ Checked
- [x] `latitude: Option<f64>` - Maps to CRD `latitude` ✅ Checked
- [x] `longitude: Option<f64>` - Maps to CRD `longitude` ✅ Checked
- [x] `tenant: Option<NestedTenant>` - Maps to CRD `tenant` ✅ Checked (required)
- [x] `region: Option<NestedRegion>` - Maps to CRD `region` ✅ Checked
- [x] `site_group: Option<NestedSiteGroup>` - Maps to CRD `site_group` ✅ Checked
- [x] `status: SiteStatus` - Maps to CRD `status` ✅ Checked (converted to string)
- [x] `facility: Option<String>` - Maps to CRD `facility` ✅ Checked
- [x] `time_zone: Option<String>` - Maps to CRD `time_zone` ✅ Checked
- [x] `comments: Option<String>` - Maps to CRD `comments` ✅ Checked
- [x] `tags: Vec<NestedTag>` - Maps to CRD `tags` ✅ Checked via `tags_differ()`

## Field Mapping

| CRD Field | NetBox Field | Helper Used | Status | Notes |
|-----------|--------------|-------------|--------|-------|
| `name` | `name` | ✅ `compare_string_field()` | ✅ Checked | ✅ Using helper |
| `slug` | `slug` | ✅ `compare_slug_field()` | ✅ Checked | ✅ Using helper |
| `description` | `description` | ✅ `compare_optional_string_field()` | ✅ Checked | ✅ Using helper |
| `physical_address` | `physical_address` | ✅ `compare_optional_string_field()` | ✅ Checked | ✅ Using helper |
| `shipping_address` | `shipping_address` | ✅ `compare_optional_string_field()` | ✅ Checked | ✅ Using helper |
| `latitude` | `latitude` | ✅ `compare_optional_numeric_field()` | ✅ Checked | ✅ Using helper |
| `longitude` | `longitude` | ✅ `compare_optional_numeric_field()` | ✅ Checked | ✅ Using helper |
| `tenant` | `tenant` | ✅ `compare_required_dependency_id()` | ✅ Checked | ✅ Using helper (required) |
| `region` | `region` | ✅ `compare_optional_dependency_id()` | ✅ Checked | ✅ Using helper |
| `site_group` | `site_group` | ✅ `compare_optional_dependency_id()` | ✅ Checked | ✅ Using helper |
| `status` | `status` | ✅ `compare_string_field()` | ✅ Checked | ✅ Using helper (enum converted to string) |
| `facility` | `facility` | ✅ `compare_optional_string_field()` | ✅ Checked | ✅ Using helper |
| `time_zone` | `time_zone` | ✅ `compare_optional_string_field()` | ✅ Checked | ✅ Using helper |
| `comments` | `comments` | ✅ `compare_optional_string_field()` | ✅ Checked | ✅ Using helper |
| `tags` | `tags` | ✅ `tags_differ()` + `update_tags_if_differ()` | ✅ Checked | ✅ Using helper |

## Implementation Status

### Drift Detection Function
- [x] `site_needs_update()` function created
- [x] All CRD spec fields are checked
- [x] All NetBox model fields (that map to CRD) are checked
- [x] All fields use reusable helpers
- [x] No inline comparison code

### Helper Functions
- [x] All required helpers exist in `reconcile_helpers.rs`
- [x] All helpers are documented

### Integration
- [x] Reconciler uses `site_needs_update()` function
- [x] Drift detection is enabled by default
- [x] Drift detection respects `drift_detection` flag
- [x] Updates are performed when drift is detected
- [x] `UPDATED` events are emitted

### Testing
- [ ] Unit tests for `site_needs_update()` function
- [ ] Unit tests for all field comparisons
- [ ] Integration tests for drift detection
- [ ] Tests verify all fields are checked

## Notes

- Complex resource with many fields including geographic coordinates
- Tenant is required (not optional)
- Status enum is converted to string for comparison
- All fields are properly checked using helpers

