# Dead Code & Unused Code Audit

This document provides a comprehensive audit of all dead code, unused code, and experimental files in the codebase. The goal is to identify what can be safely removed to showcase a clean codebase.

**Generated:** 2025-12-27  
**Analysis Scope:** Full workspace (`cargo check --workspace`)

---

## Summary Statistics

| Category | Count | Action Required |
|----------|-------|-----------------|
| **Unused Imports** | 15 | ✅ **REMOVE** - Easy cleanup |
| **Unused Variables** | 6 | ⚠️ **REVIEW** - May need prefixing or usage |
| **Unused Methods (False Positives)** | 2 | ✅ **KEEP** - Trait/utility methods |
| **Stub Controllers (Future Work)** | 1 | ⚠️ **KEEP** - Phase 2+ work |
| **Experimental Test Files** | 3 | ✅ **REMOVE** - Temporary debugging code |
| **Unused Status Types** | 10 | ⚠️ **REVIEW** - May be needed for future status updates |

**Total Actionable Items:** 34

---

## 1. Unused Imports (15 items)

### High Priority - Remove Immediately

| Import | Location | Reason | Action |
|--------|----------|--------|--------|
| `error` | `controllers/netbox/src/controller.rs:24` | Imported but never used | **REMOVE** |
| `NetBoxClient` | `controllers/netbox/src/reconciler/ipam/prefix.rs:9` | Only `NetBoxClientTrait` needed | **REMOVE** |
| `NetBoxClient` | `controllers/netbox/src/reconciler/dcim/site.rs:8` | Only `NetBoxClientTrait` needed | **REMOVE** |
| `NetBoxRegionStatus` | `controllers/netbox/src/reconciler/dcim/region.rs:8` | Status type not used in reconciler | **REMOVE** |
| `NetBoxSiteGroupStatus` | `controllers/netbox/src/reconciler/dcim/site_group.rs:8` | Status type not used in reconciler | **REMOVE** |
| `NetBoxDeviceRoleStatus` | `controllers/netbox/src/reconciler/dcim/device_role.rs:8` | Status type not used in reconciler | **REMOVE** |
| `NetBoxManufacturerStatus` | `controllers/netbox/src/reconciler/dcim/manufacturer.rs:8` | Status type not used in reconciler | **REMOVE** |
| `NetBoxPlatformStatus` | `controllers/netbox/src/reconciler/dcim/platform.rs:7` | Status type not used in reconciler | **REMOVE** |
| `NetBoxDeviceTypeStatus` | `controllers/netbox/src/reconciler/dcim/device_type.rs:7` | Status type not used in reconciler | **REMOVE** |
| `NetBoxInterfaceStatus` | `controllers/netbox/src/reconciler/dcim/interface.rs:7` | Status type not used in reconciler | **REMOVE** |
| `NetBoxMACAddressStatus` | `controllers/netbox/src/reconciler/dcim/mac_address.rs:7` | Status type not used in reconciler | **REMOVE** |
| `NetBoxRoleStatus` | `controllers/netbox/src/reconciler/extras.rs:7` | Status type not used in reconciler | **REMOVE** |
| `NetBoxTagStatus` | `controllers/netbox/src/reconciler/extras.rs:7` | Status type not used in reconciler | **REMOVE** |
| `KubeApiTrait` | `controllers/netbox/src/reconciler/ipam/aggregate.rs:6` | Trait methods accessed via `Self::` | **REMOVE** |
| `KubeApiTrait` | `controllers/netbox/src/reconciler/dcim/region.rs:6` | Trait methods accessed via `Self::` | **REMOVE** |
| `KubeApiTrait` | `controllers/netbox/src/reconciler/dcim/site_group.rs:6` | Trait methods accessed via `Self::` | **REMOVE** |
| `KubeApiTrait` | `controllers/netbox/src/reconciler/dcim/device_role.rs:6` | Trait methods accessed via `Self::` | **REMOVE** |
| `KubeApiTrait` | `controllers/netbox/src/reconciler/dcim/manufacturer.rs:6` | Trait methods accessed via `Self::` | **REMOVE** |

**Note:** Status types (`*Status`) are imported but not used because reconcilers use `Self::create_resource_status_patch()` which constructs JSON directly rather than using the status type structs. This is intentional - the status types are for CRD definitions, not reconciler code.

---

## 2. Unused Variables (6 items)

### Review Required - May Need Prefixing or Usage

| Variable | Location | Context | Recommendation |
|----------|----------|---------|----------------|
| `site_id` | `crates/netbox-client/src/mock/ipam.rs:183` | `create_prefix` function parameter | **PREFIX** - Use `_site_id` if not needed |
| `site_id` | `crates/netbox-client/src/mock/ipam.rs:231` | `update_prefix` function parameter | **PREFIX** - Use `_site_id` if not needed |
| `group_id` | `crates/netbox-client/src/mock/ipam.rs:348` | `create_vlan` function parameter | **PREFIX** - Use `_group_id` if not needed |
| `group_id` | `crates/netbox-client/src/mock/ipam.rs:381` | `update_vlan` function parameter | **PREFIX** - Use `_group_id` if not needed |
| `tenant_id` | `crates/netbox-client/src/mock/dcim.rs:287` | `create_location` function parameter | **PREFIX** - Use `_tenant_id` if not needed |
| `facility` | `crates/netbox-client/src/mock/dcim.rs:287` | `create_location` function parameter | **PREFIX** - Use `_facility` if not needed |
| `new_status` | `controllers/netbox/src/reconciler/ipam/aggregate.rs:165` | Local variable in aggregate reconciler | **REVIEW** - May be leftover from refactoring |

**Analysis:**
- Mock function parameters that match trait signatures but aren't used in mock implementation
- These should be prefixed with `_` to indicate intentional non-use
- `new_status` in aggregate reconciler should be checked - may be leftover code

---

## 3. Unused Methods (False Positives - Keep)

| Method | Location | Type | Reason to Keep |
|--------|----------|------|----------------|
| `url()` | `controllers/netbox/src/reconcile_helpers.rs:12` | Trait method | Used via `dyn NetBoxResource` trait objects |
| `inner()` | `controllers/netbox/src/kube_api_trait.rs:83` | Utility method | Useful for debugging/testing, no harm keeping |

**Action:** ✅ **KEEP** - These are false positives. See `UNUSED_METHODS_AUDIT.md` for details.

---

## 4. Stub Controllers (Future Work - Keep for Now)

### PXE Intent Controller (Phase 2+)

| Item | Location | Status | Action |
|------|----------|--------|--------|
| `Controller` struct | `controllers/pxe-intent/src/controller.rs:11` | Stub with `todo!()` | **KEEP** - Phase 2+ work |
| `Controller::new()` | `controllers/pxe-intent/src/controller.rs:18` | Stub with `todo!()` | **KEEP** - Phase 2+ work |
| `Controller::run()` | `controllers/pxe-intent/src/controller.rs:24` | Stub with `todo!()` | **KEEP** - Phase 2+ work |
| `Reconciler` struct | `controllers/pxe-intent/src/reconciler.rs:10` | Stub with `todo!()` | **KEEP** - Phase 2+ work |
| `Reconciler::new()` | `controllers/pxe-intent/src/reconciler.rs:16` | Stub with `todo!()` | **KEEP** - Phase 2+ work |
| `Reconciler::reconcile_boot_intent()` | `controllers/pxe-intent/src/reconciler.rs:22` | Stub with `todo!()` | **KEEP** - Phase 2+ work |
| `Reconciler::reconcile_boot_profile()` | `controllers/pxe-intent/src/reconciler.rs:28` | Stub with `todo!()` | **KEEP** - Phase 2+ work |
| `Watcher` struct | `controllers/pxe-intent/src/watcher.rs:9` | Stub with `todo!()` | **KEEP** - Phase 2+ work |
| `Watcher::new()` | `controllers/pxe-intent/src/watcher.rs:15` | Stub with `todo!()` | **KEEP** - Phase 2+ work |
| `Watcher::watch_boot_intents()` | `controllers/pxe-intent/src/watcher.rs:21` | Stub with `todo!()` | **KEEP** - Phase 2+ work |
| `Watcher::watch_boot_profiles()` | `controllers/pxe-intent/src/watcher.rs:27` | Stub with `todo!()` | **KEEP** - Phase 2+ work |
| `ControllerError` enum | `controllers/pxe-intent/src/error.rs` | Defined but unused | **KEEP** - Will be used when controller is implemented |

**Recommendation:** ✅ **KEEP** - These are intentional stubs for Phase 2+ work. They provide structure for future implementation.

### RouterOS Controller (Phase 2+)

| Item | Location | Status | Action |
|------|----------|--------|--------|
| `main()` | `controllers/routeros/src/main.rs:11` | Stub with TODOs | **KEEP** - Phase 2+ work |

**Recommendation:** ✅ **KEEP** - Intentional stub for Phase 2+ work.

---

## 5. Experimental Test Files (Remove)

### Temporary Debugging Scripts

| File | Location | Purpose | Status | Action |
|------|----------|---------|--------|--------|
| `test_tenant_format.rs` | `scripts/test_tenant_format.rs` | Temporary script to test NetBox API tenant format | Experimental | ✅ **REMOVE** |
| `test_netbox_tenant_format.rs` | `test_netbox_tenant_format.rs` | Duplicate of above, uses reqwest directly | Experimental | ✅ **REMOVE** |
| `test_tenant_update.rs` | `controllers/netbox/examples/test_tenant_update.rs` | Test script for tenant update debugging | Experimental | ✅ **REMOVE** |

**Analysis:**
- These were created during debugging of the tenant field format issue
- The issue has been resolved (see commit history)
- These files are no longer needed
- They're not part of the test suite (not in `#[cfg(test)]` modules)

**Recommendation:** ✅ **REMOVE** - Temporary debugging code that's no longer needed.

---

## 6. Unused Status Type Imports (Review)

### Status Types Imported But Not Used

These status types are imported in reconcilers but not directly used because:
- Reconcilers use `Self::create_resource_status_patch()` which constructs JSON directly
- The status types are for CRD definitions, not reconciler code
- However, they may be needed if we want to use typed status updates in the future

| Status Type | Location | Used? | Recommendation |
|-------------|----------|-------|---------------|
| `NetBoxRegionStatus` | `dcim/region.rs` | ❌ No | **REMOVE** - Not used in reconciler |
| `NetBoxSiteGroupStatus` | `dcim/site_group.rs` | ❌ No | **REMOVE** - Not used in reconciler |
| `NetBoxDeviceRoleStatus` | `dcim/device_role.rs` | ❌ No | **REMOVE** - Not used in reconciler |
| `NetBoxManufacturerStatus` | `dcim/manufacturer.rs` | ❌ No | **REMOVE** - Not used in reconciler |
| `NetBoxPlatformStatus` | `dcim/platform.rs` | ❌ No | **REMOVE** - Not used in reconciler |
| `NetBoxDeviceTypeStatus` | `dcim/device_type.rs` | ❌ No | **REMOVE** - Not used in reconciler |
| `NetBoxInterfaceStatus` | `dcim/interface.rs` | ❌ No | **REMOVE** - Not used in reconciler |
| `NetBoxMACAddressStatus` | `dcim/mac_address.rs` | ❌ No | **REMOVE** - Not used in reconciler |
| `NetBoxRoleStatus` | `extras.rs` | ❌ No | **REMOVE** - Not used in reconciler |
| `NetBoxTagStatus` | `extras.rs` | ❌ No | **REMOVE** - Not used in reconciler |

**Note:** If we want to use typed status updates in the future, we can add these imports back. For now, they're just noise.

---

## Action Plan

### Phase 1: Quick Wins (Unused Imports & Variables)

1. **Remove unused imports** (15 items)
   - Run `cargo fix --workspace` to auto-fix some
   - Manually remove status type imports
   - Remove `KubeApiTrait` imports where not needed

2. **Fix unused variables** (6 items)
   - Prefix with `_` if intentionally unused
   - Review `new_status` in aggregate reconciler

### Phase 2: Cleanup Experimental Files

3. **Remove experimental test files** (3 files)
   - `scripts/test_tenant_format.rs`
   - `test_netbox_tenant_format.rs`
   - `controllers/netbox/examples/test_tenant_update.rs`

### Phase 3: Documentation

4. **Document stub controllers**
   - Add comments explaining Phase 2+ status
   - Consider `#[allow(dead_code)]` attributes with explanations

---

## Impact Assessment

### Safe to Remove (No Impact)
- ✅ Unused imports (15 items)
- ✅ Experimental test files (3 files)
- ✅ Unused status type imports (10 items)

### Review Required (Potential Impact)
- ⚠️ Unused variables (6 items) - May indicate missing functionality
- ⚠️ `new_status` variable in aggregate reconciler - May be leftover code

### Keep (Intentional)
- ✅ Stub controllers (Phase 2+ work)
- ✅ Trait methods (`url()`, `inner()`)

---

## Estimated Cleanup Impact

| Category | Files Affected | Lines Removed | Risk Level |
|----------|----------------|---------------|------------|
| Unused Imports | 15 files | ~15 lines | ✅ **LOW** |
| Unused Variables | 6 files | ~6 lines (prefix changes) | ✅ **LOW** |
| Experimental Files | 3 files | ~252 lines | ✅ **LOW** |
| **Total** | **24 files** | **~273 lines** | ✅ **LOW** |

---

## Recommendations

### Immediate Actions (High Priority)

1. ✅ **Remove all unused imports** - Zero risk, improves code clarity
2. ✅ **Remove experimental test files** - No longer needed, were temporary debugging
3. ✅ **Prefix unused variables** - Indicates intentional non-use, prevents warnings

### Future Considerations (Low Priority)

4. ⚠️ **Review stub controllers** - Consider moving to separate branch or marking more clearly
5. ⚠️ **Consider typed status updates** - If we want to use status types in reconcilers, we can add imports back

---

## Conclusion

**Total Dead Code Identified:** 34 items  
**Safe to Remove:** 28 items (82%)  
**Review Required:** 6 items (18%)  
**Keep (Intentional):** 13 items (stub controllers)

**Recommendation:** Proceed with Phase 1 and Phase 2 cleanup. This will remove ~171 lines of dead code and eliminate all actionable warnings, resulting in a cleaner, more maintainable codebase.

