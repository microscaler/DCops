# Implementation Checklists for NetBox CRDs

This directory contains implementation checklists for each NetBox CRD reconciler. Each checklist ensures that:

1. **ALL fields** in the CRD spec are checked for drift
2. **Reusable helpers** are used to maintain DRY code
3. **No fields are skipped** - GitOps requires complete reconciliation

## Directory Structure

```
docs/implementationChecklists/
├── README.md (this file)
├── TEMPLATE.md (template for creating new checklists)
├── NetBoxTenant.md
├── NetBoxTenantGroup.md
├── NetBoxPlatform.md
├── NetBoxManufacturer.md
└── ... (one file per CRD)
```

## How to Use

1. **When creating a new reconciler:**
   - Copy `TEMPLATE.md` to `<CRD>.md`
   - Fill in all fields from the CRD spec
   - Check off each field as you implement drift detection
   - Ensure all fields use reusable helpers

2. **When updating a CRD:**
   - Update the corresponding checklist file
   - Add new fields to the checklist
   - Verify helpers exist for new field types
   - Create helpers if needed (see Helper Library below)

3. **When reviewing code:**
   - Check the implementation checklist
   - Verify all fields are checked
   - Verify helpers are used (not inline code)

## Why We Don't Use Macros

**Previous Attempt:** We tried using macros (`impl_netbox_delegate!`) to generate async trait methods, but it failed due to `async_trait` lifetime expansion conflicts (see `crates/netbox-client/src/macros.rs`).

**Current Approach:** We use simple helper function composition instead of macros:
- **Simpler**: No macro complexity, just function calls
- **More Maintainable**: Easy to read, debug, and test
- **Still DRY**: All comparison logic in reusable helpers
- **No Risk**: Avoids repeating past macro failures

See `docs/implementationChecklists/MACRO_ANALYSIS.md` for detailed analysis.

**Example:**
```rust
// Instead of macros, use helper composition:
let needs_update = 
    compare_string_field(&spec.name, &netbox.name)
    || compare_slug_field(&spec.slug, &netbox.slug, auto_generated)
    || compare_optional_string_field(&spec.description, &netbox.description)
    || compare_optional_string_field(&spec.comments, &netbox.comments)
    || compare_optional_dependency_id(spec_group_id, netbox_group_id);
```

## Helper Library

### Available Helpers

#### Tag Helpers
- `tags_differ()` - Compare tag arrays
- `convert_tags_to_strings()` - Convert tag JSON to string array
- `update_tags_if_differ()` - Update tags if they differ

#### Dependency Helpers
- `resolve_required_dependency_id()` - Resolve required dependency CRD → NetBox ID
- `resolve_optional_dependency_id()` - Resolve optional dependency CRD → NetBox ID
- `validate_reference_kind()` - Validate CRD reference kind

#### Status Helpers
- `status_needs_update()` - Check if status needs updating
- `validate_status_and_drift()` - Validate status and detect drift
- `create_pending_status_patch()` - Create status patch for Pending state
- `create_drift_status_patch()` - Create status patch for drift detection

#### Field Comparison Helpers
**✅ Available:** All field comparison helpers are now implemented in `reconcile_helpers.rs`:
- ✅ `compare_string_field()` - For required string fields like `name`
- ✅ `compare_slug_field()` - For slug fields with auto-generation support
- ✅ `compare_optional_string_field()` - For optional string fields like `description`, `comments`
- ✅ `compare_optional_dependency_id()` - For optional dependency IDs (group, region, etc.)
- ✅ `compare_required_dependency_id()` - For required dependency IDs (tenant, etc.)
- ✅ `compare_optional_numeric_field()` - For optional numeric fields (latitude, longitude, etc.)
- ✅ `compare_enum_field()` - For enum fields (status, role, type, etc.)

**Usage Pattern:**
```rust
use crate::reconcile_helpers::{
    compare_string_field,
    compare_slug_field,
    compare_optional_string_field,
    compare_optional_dependency_id,
};

let needs_update = 
    compare_string_field(&spec.name, &netbox.name)
    || compare_slug_field(&spec.slug, &netbox.slug, auto_generated)
    || compare_optional_string_field(&spec.description, &netbox.description)
    || compare_optional_string_field(&spec.comments, &netbox.comments)
    || compare_optional_dependency_id(spec_group_id, netbox_group_id);
```

#### Utility Helpers
- `extract_name_and_namespace()` - Extract name/namespace from CRD
- `is_conflict_error()` - Check if error is a conflict
- `is_valid_mac_address()` - Validate MAC address
- `normalize_mac_address()` - Normalize MAC address format

## Creating New Helpers

When a field type appears in multiple CRDs, create a reusable helper:

1. Add helper function to `controllers/netbox/src/reconcile_helpers.rs`
2. Document the helper with examples
3. Add unit tests
4. Update this README with the new helper
5. Update all reconcilers to use the new helper

## Field Types and Required Helpers

### String Fields
- **Required Helper:** `compare_string_field(spec: &Option<String>, netbox: &Option<String>) -> bool`
- **Usage:** For `description`, `comments`, and other optional string fields

### Slug Fields
- **Required Helper:** `compare_slug_field(spec: &Option<String>, netbox: &str, auto_generated: String) -> bool`
- **Usage:** For `slug` fields that may be auto-generated from `name`

### Enum Fields
- **Required Helper:** `compare_enum_field<T: PartialEq>(spec: &T, netbox: &T) -> bool`
- **Usage:** For `status`, `role`, `type`, and other enum fields

### Numeric Fields
- **Required Helper:** `compare_numeric_field<T: PartialEq>(spec: &Option<T>, netbox: &Option<T>) -> bool`
- **Usage:** For `latitude`, `longitude`, `u_height`, and other numeric fields

### Dependency Fields
- **Already have helpers:** `resolve_required_dependency_id()`, `resolve_optional_dependency_id()`
- **Usage:** For `tenant`, `region`, `group`, `manufacturer`, and other dependency references

## Checklist Format

Each checklist file should contain:

1. **CRD Spec Fields** - All fields from the CRD spec
2. **NetBox Model Fields** - All fields from the NetBox API model
3. **Field Mapping** - Which CRD fields map to which NetBox fields
4. **Helper Usage** - Which helper is used for each field
5. **Implementation Status** - Checkbox for each field
6. **Notes** - Any special considerations

## Verification

Before marking a reconciler as complete:

- [ ] All CRD spec fields are checked
- [ ] All NetBox model fields (that map to CRD) are checked
- [ ] All fields use reusable helpers (no inline comparison code)
- [ ] All helpers have unit tests
- [ ] All fields are tested in integration tests
- [ ] Checklist file is up-to-date

