# Implementation Checklist: NetBox<Resource>

**CRD:** `NetBox<Resource>`  
**Module:** `<module>/netbox_<resource>.rs`  
**Reconciler:** `controllers/netbox/src/reconciler/<module>/<resource>.rs`  
**Last Updated:** YYYY-MM-DD

## CRD Spec Fields

List all fields from `crates/crds/src/<module>/netbox_<resource>.rs`:

### Required Fields
- [ ] `name: String` - **Helper:** `compare_string_field()` (needs to be created)
- [ ] `slug: Option<String>` - **Helper:** `compare_slug_field()` (needs to be created)

### Optional Fields
- [ ] `description: Option<String>` - **Helper:** `compare_optional_string_field()` (needs to be created)
- [ ] `comments: Option<String>` - **Helper:** `compare_optional_string_field()` (needs to be created)

### Dependency Fields
- [ ] `tenant: NetBoxResourceReference` - **Helper:** `resolve_required_dependency_id()`
- [ ] `region: Option<NetBoxResourceReference>` - **Helper:** `resolve_optional_dependency_id()`
- [ ] `group: Option<NetBoxResourceReference>` - **Helper:** `resolve_optional_dependency_id()`

### Enum Fields
- [ ] `status: <StatusEnum>` - **Helper:** `compare_enum_field()` (needs to be created)

### Numeric Fields
- [ ] `latitude: Option<f64>` - **Helper:** `compare_optional_numeric_field()` (needs to be created)
- [ ] `longitude: Option<f64>` - **Helper:** `compare_optional_numeric_field()` (needs to be created)

### String Fields
- [ ] `physical_address: Option<String>` - **Helper:** `compare_optional_string_field()` (needs to be created)
- [ ] `shipping_address: Option<String>` - **Helper:** `compare_optional_string_field()` (needs to be created)

### Tag Fields
- [ ] `tags: Option<Vec<NetBoxResourceReference>>` - **Helper:** `tags_differ()` + `update_tags_if_differ()`

### Controller Config Fields (NOT checked - not NetBox fields)
- [ ] `drift_detection: Option<bool>` - Controller config, not a NetBox field
- [ ] `reconcile_interval: Option<u64>` - Controller config, not a NetBox field

## NetBox Model Fields

List all fields from `crates/netbox-client/src/models.rs`:

### Read-Only Fields (NOT checked - read-only)
- [ ] `id: u64` - Read-only, stored in status
- [ ] `url: String` - Read-only, stored in status
- [ ] `display: String` - Read-only, computed field
- [ ] `created: String` - Read-only, timestamp
- [ ] `last_updated: String` - Read-only, timestamp
- [ ] `*_count: u64` - Read-only, computed field

### Managed Fields (MUST be checked)
- [ ] `name: String` - Maps to CRD `name`
- [ ] `slug: String` - Maps to CRD `slug`
- [ ] `description: Option<String>` - Maps to CRD `description`
- [ ] `comments: Option<String>` - Maps to CRD `comments`
- [ ] `tenant: Option<NestedTenant>` - Maps to CRD `tenant`
- [ ] `region: Option<NestedRegion>` - Maps to CRD `region`
- [ ] `group: Option<NestedGroup>` - Maps to CRD `group`
- [ ] `status: <StatusEnum>` - Maps to CRD `status`
- [ ] `latitude: Option<f64>` - Maps to CRD `latitude`
- [ ] `longitude: Option<f64>` - Maps to CRD `longitude`
- [ ] `physical_address: Option<String>` - Maps to CRD `physical_address`
- [ ] `shipping_address: Option<String>` - Maps to CRD `shipping_address`
- [ ] `tags: Vec<NestedTag>` - Maps to CRD `tags`

## Field Mapping

| CRD Field | NetBox Field | Helper Used | Status |
|-----------|--------------|-------------|--------|
| `name` | `name` | `compare_string_field()` (needs helper) | ⚠️ TODO |
| `slug` | `slug` | `compare_slug_field()` (needs helper) | ⚠️ TODO |
| `description` | `description` | `compare_optional_string_field()` (needs helper) | ⚠️ TODO |
| `comments` | `comments` | `compare_optional_string_field()` (needs helper) | ⚠️ TODO |
| `tenant` | `tenant` | `resolve_required_dependency_id()` | ✅ Has helper |
| `region` | `region` | `resolve_optional_dependency_id()` | ✅ Has helper |
| `group` | `group` | `resolve_optional_dependency_id()` | ✅ Has helper |
| `status` | `status` | `compare_enum_field()` (needs helper) | ⚠️ TODO |
| `latitude` | `latitude` | `compare_optional_numeric_field()` (needs helper) | ⚠️ TODO |
| `longitude` | `longitude` | `compare_optional_numeric_field()` (needs helper) | ⚠️ TODO |
| `physical_address` | `physical_address` | `compare_optional_string_field()` (needs helper) | ⚠️ TODO |
| `shipping_address` | `shipping_address` | `compare_optional_string_field()` (needs helper) | ⚠️ TODO |
| `tags` | `tags` | `tags_differ()` + `update_tags_if_differ()` | ✅ Has helper |

## Implementation Status

### Drift Detection Function
- [ ] `*_needs_update()` function created
- [ ] All CRD spec fields are checked
- [ ] All NetBox model fields (that map to CRD) are checked
- [ ] All fields use reusable helpers
- [ ] No inline comparison code

### Helper Functions
- [ ] All required helpers exist in `reconcile_helpers.rs`
- [ ] All helpers have unit tests
- [ ] All helpers are documented

### Integration
- [ ] Reconciler uses `*_needs_update()` function
- [ ] Drift detection is enabled by default
- [ ] Drift detection respects `drift_detection` flag
- [ ] Updates are performed when drift is detected
- [ ] `DRIFT_DETECTED` events are emitted

### Testing
- [ ] Unit tests for `*_needs_update()` function
- [ ] Unit tests for all field comparisons
- [ ] Integration tests for drift detection
- [ ] Tests verify all fields are checked

## Notes

- Add any special considerations or edge cases here
- Document any fields that require special handling
- Note any limitations or known issues

## Helper Creation Checklist

If helpers are missing, create them:

- [ ] Helper function created in `reconcile_helpers.rs`
- [ ] Helper function documented with examples
- [ ] Unit tests added for helper
- [ ] Helper added to README.md
- [ ] All reconcilers updated to use new helper

