# Implementation Checklist: NetBoxTenant

**CRD:** `NetBoxTenant`  
**Module:** `tenancy/netbox_tenant.rs`  
**Reconciler:** `controllers/netbox/src/reconciler/tenancy/tenant.rs`  
**Last Updated:** 2026-01-03

## CRD Spec Fields

### Required Fields
- [x] `name: String` - **Helper:** ⚠️ Inline comparison (needs `compare_string_field()` helper)
- [x] `slug: Option<String>` - **Helper:** ⚠️ Inline comparison with auto-generation logic (needs `compare_slug_field()` helper)
- [x] `token_secret: SecretReference` - **Helper:** Controller config, not a NetBox field (excluded)

### Optional Fields
- [x] `description: Option<String>` - **Helper:** ⚠️ Inline comparison (needs `compare_optional_string_field()` helper)
- [x] `comments: Option<String>` - **Helper:** ⚠️ Inline comparison (needs `compare_optional_string_field()` helper)

### Dependency Fields
- [x] `group: Option<NetBoxResourceReference>` - **Helper:** ✅ Resolved inline, then compared (needs `compare_optional_dependency_id()` helper)

### Tag Fields
- [x] `tags: Option<Vec<NetBoxResourceReference>>` - **Helper:** ✅ `tags_differ()` + `update_tags_if_differ()`

### Controller Config Fields (NOT checked - not NetBox fields)
- [x] `drift_detection: Option<bool>` - Controller config, not a NetBox field
- [x] `reconcile_interval: Option<u64>` - Controller config, not a NetBox field

## NetBox Model Fields

### Read-Only Fields (NOT checked - read-only)
- [x] `id: u64` - Read-only, stored in status
- [x] `url: String` - Read-only, stored in status
- [x] `display: String` - Read-only, computed field
- [x] `created: String` - Read-only, timestamp
- [x] `last_updated: String` - Read-only, timestamp

### Managed Fields (MUST be checked)
- [x] `name: String` - Maps to CRD `name` ✅ Checked inline
- [x] `slug: String` - Maps to CRD `slug` ✅ Checked inline (with auto-generation logic)
- [x] `description: Option<String>` - Maps to CRD `description` ✅ Checked inline
- [x] `comments: Option<String>` - Maps to CRD `comments` ✅ Checked inline
- [x] `group: Option<NestedTenantGroup>` - Maps to CRD `group` ✅ Checked inline (resolved first)
- [x] `tags: Vec<NestedTag>` - Maps to CRD `tags` ✅ Checked via `tags_differ()`

## Field Mapping

| CRD Field | NetBox Field | Helper Used | Status | Notes |
|-----------|--------------|-------------|--------|-------|
| `name` | `name` | ✅ `compare_string_field()` | ✅ Checked | ✅ Using helper |
| `slug` | `slug` | ✅ `compare_slug_field()` | ✅ Checked | ✅ Using helper |
| `description` | `description` | ✅ `compare_optional_string_field()` | ✅ Checked | ✅ Using helper |
| `comments` | `comments` | ✅ `compare_optional_string_field()` | ✅ Checked | ✅ Using helper |
| `group` | `group` | ✅ `compare_optional_dependency_id()` | ✅ Checked | ✅ Using helper |
| `tags` | `tags` | ✅ `tags_differ()` + `update_tags_if_differ()` | ✅ Checked | ✅ Using helper |

## Implementation Status

### Drift Detection Function
- [x] Field comparison logic implemented (inline in `reconcile_netbox_tenant()`)
- [x] All CRD spec fields are checked
- [x] All NetBox model fields (that map to CRD) are checked
- [x] ✅ **All fields use reusable helpers** - Refactored 2026-01-03
- [x] No fields are skipped

### Helper Functions
- [x] ✅ `compare_string_field()` - For `name` field (created 2026-01-03)
- [x] ✅ `compare_slug_field()` - For `slug` field with auto-generation (created 2026-01-03)
- [x] ✅ `compare_optional_string_field()` - For `description`, `comments` (created 2026-01-03)
- [x] ✅ `compare_optional_dependency_id()` - For `group` dependency (created 2026-01-03)

### Integration
- [x] Reconciler checks all fields in `needs_update` closure
- [x] Drift detection is enabled by default
- [x] Drift detection respects `drift_detection` flag
- [x] Updates are performed when drift is detected
- [x] `DRIFT_DETECTED` events are emitted (via `UPDATED` event)

### Testing
- [ ] Unit tests for field comparisons (needs to be added)
- [ ] Integration tests for drift detection (needs to be added)
- [ ] Tests verify all fields are checked (needs to be added)

## Current Implementation

The tenant reconciler now uses reusable helpers for DRY field comparison:

```rust
use crate::reconcile_helpers::{
    compare_string_field,
    compare_slug_field,
    compare_optional_string_field,
    compare_optional_dependency_id,
};

let auto_generated_slug = tenant_crd.spec.name.to_lowercase().replace(' ', "-");
let needs_update = 
    compare_string_field(&tenant_crd.spec.name, &tenant.name)
    || compare_slug_field(&tenant_crd.spec.slug, &tenant.slug, auto_generated_slug)
    || compare_optional_string_field(&tenant_crd.spec.description, &tenant.description)
    || compare_optional_string_field(&tenant_crd.spec.comments, &tenant.comments)
    || compare_optional_dependency_id(spec_group_id, netbox_group_id);
```

**Status:** ✅ Refactored to use helpers (2026-01-03)

## Why We Don't Use Macros

We considered using macros (like `#[helpers="helper1, helper2"]`), but decided against it because:

1. **Previous macro failure**: We tried macros before (`impl_netbox_delegate!`) and they failed with `async_trait` lifetime conflicts
2. **Our use case is simpler**: We're just composing boolean comparisons with `||` - no need for macro complexity
3. **Helper composition is better**: 
   - Simpler (no macro_rules! complexity)
   - More maintainable (easy to read and debug)
   - Type-safe (full Rust type checking)
   - Testable (each helper can be unit tested)
   - Still DRY (all comparison logic in reusable helpers)

See `docs/implementationChecklists/MACRO_ANALYSIS.md` for detailed analysis.

## Notes

- The tenant reconciler has special token resolution logic (uses its own secret, not TokenResolver)
- Group dependency is resolved before comparison (CRD reference → NetBox ID)
- Tags are handled separately via `tags_differ()` and `update_tags_if_differ()` helpers
- All fields are currently checked, but using inline code instead of reusable helpers

