# Controller Fix and Iterative Test Implementation Plan

**Date:** 2025-01-26  
**Status:** ✅ **Controller Fix Complete** - Ready for iterative test implementation

## Controller Fix

### Issue
```
Failed to update site 1: 400 Bad Request - {"tenant":{"non_field_errors":["Invalid data. Expected a dictionary, but got int."]}}
```

### Root Cause
The `update_site` function in `netbox-client` was sending the `tenant` field as an integer (`tenant_id`) instead of a dictionary/object format required by NetBox 4.0.

### Fix Applied
**File:** `crates/netbox-client/src/client.rs` (line ~1370)

**Before:**
```rust
if let Some(tid) = tenant_id {
    body["tenant"] = serde_json::Value::Number(tid.into());
}
```

**After:**
```rust
if let Some(tid) = tenant_id {
    // Send tenant as dictionary with id (NetBox 4.0 requires this format)
    body["tenant"] = serde_json::json!({"id": tid});
}
```

This matches the format used for `region` and `site_group` fields, which were already correct.

### Verification
- ✅ Code compiles successfully
- ⏳ Controller should now work without the 400 error
- ⏳ Ready for deployment and testing

## Iterative Test Implementation Plan

### Strategy
1. **Keep Controller Functional**: All test implementations must not break the controller
2. **Incremental Approach**: Implement tests one reconciler at a time
3. **Verify After Each**: Run tests and verify controller still works
4. **Demo-Ready State**: Controller must remain demoable at all times

### Implementation Order

#### Phase 1: NetBoxPrefix (Already Started) ✅
- [x] Tests written and compiling
- [ ] Run tests and verify they pass
- [ ] Fix any runtime issues
- [ ] Verify controller still works

#### Phase 2: Simple Reconcilers (Low Dependencies)
1. **NetBoxSite** (after controller fix verified)
   - Similar to NetBoxPrefix
   - Test create, update, idempotent paths
   
2. **NetBoxTenant**
   - Simple reconciler
   - Test create, update, idempotent paths

#### Phase 3: IPAM Reconcilers
3. **IPPool**
   - Depends on NetBoxPrefix
   - Test prefix resolution and status updates
   
4. **IPClaim**
   - Depends on IPPool
   - Test IP allocation and status updates

5. **NetBoxPrefix** (Complete)
   - Already implemented
   - Verify and fix if needed

#### Phase 4: Complex Reconcilers (Many Dependencies)
6. **NetBoxDevice**
   - Many dependencies (DeviceType, DeviceRole, Site, etc.)
   - Test dependency resolution
   - Test create with all dependencies

7. **NetBoxInterface**
   - Depends on NetBoxDevice
   - Test interface creation and updates

### Test Implementation Checklist (Per Reconciler)

For each reconciler:
- [ ] Remove `#[ignore]` from test functions
- [ ] Set up mock NetBoxClient with required data
- [ ] Set up mock KubeApi with CRD storage
- [ ] Create reconciler with mocks
- [ ] Implement test body with assertions
- [ ] Run test and verify it passes
- [ ] Verify controller still works in cluster
- [ ] Document any issues or patterns discovered

### Testing Workflow

1. **Before Each Test Implementation:**
   ```bash
   # Verify controller builds
   cargo build --release --target x86_64-unknown-linux-musl
   
   # Deploy and verify it works
   # (controller should reconcile resources successfully)
   ```

2. **During Test Implementation:**
   ```bash
   # Run specific test
   cargo test --lib test_reconcile_<resource>_create
   
   # Fix any issues
   # Re-run test until it passes
   ```

3. **After Each Test Implementation:**
   ```bash
   # Verify all tests still pass
   cargo test --lib
   
   # Rebuild controller
   cargo build --release --target x86_64-unknown-linux-musl
   
   # Deploy and verify controller still works
   ```

### Success Criteria

- ✅ Controller works without errors
- ✅ All implemented tests pass
- ✅ Controller remains demoable at all times
- ✅ Tests provide confidence in reconciler logic
- ✅ No regressions introduced

## Current Status

### ✅ Completed
- Controller fix for tenant field format
- NetBoxPrefix test structure (3 tests written)
- Test infrastructure ready (MockNetBoxClient, MockKubeApi)

### ⏳ Next Steps
1. Deploy fixed controller and verify it works
2. Run NetBoxPrefix tests and fix any issues
3. Implement NetBoxSite tests (similar pattern)
4. Continue iteratively with other reconcilers

## Notes

- All test implementations use the same pattern:
  - Create mock NetBoxClient
  - Create mock KubeApi and store CRD
  - Create reconciler with mocks
  - Call reconcile function
  - Assert results

- Controller must remain functional - if a test breaks something, fix it immediately before continuing

- Tests are in `#[cfg(test)]` modules, so they don't affect production builds

