# Test Coverage Priority Actions

## Summary

Based on analysis of the codebase, here are the highest priority areas for improving test coverage:

## High Priority (Core Functions Used by All Reconcilers)

### 1. `reconcile_helpers.rs` - Critical Helper Functions

These functions are used by **all reconcilers** and need comprehensive test coverage:

#### `validate_status_and_drift()` - Status Validation and Drift Detection
**Current Coverage:** 0% (no tests)
**Priority:** 🔴 **CRITICAL**

**Test Cases Needed:**
1. ✅ No status - returns `Recreate`
2. ✅ Failed state with `netbox_id == 0` - returns `StatusCleared`
3. ✅ Failed state with valid `netbox_id`, resource exists - returns `UseExisting`
4. ✅ Failed state with valid `netbox_id`, resource not found - returns `StatusCleared`
5. ✅ Failed state with valid `netbox_id`, API error - returns error
6. ✅ Failed state with no `netbox_id` - returns `Recreate`
7. ✅ Created state with `netbox_id == 0` - returns `StatusCleared`
8. ✅ Created state with valid `netbox_id`, resource exists - returns `UseExisting`
9. ✅ Created state with valid `netbox_id`, resource not found (drift) - returns `StatusCleared`
10. ✅ Created state with valid `netbox_id`, API error - returns error
11. ✅ Created state with no `netbox_id` - returns `Recreate`
12. ✅ Other states (Pending, Updated) - returns `Recreate`

**Estimated Effort:** 2-3 hours
**Impact:** HIGH - Used by all reconcilers

#### `check_and_update_existing()` - Generic Update Pattern
**Current Coverage:** 0% (no tests)
**Priority:** 🔴 **CRITICAL**

**Test Cases Needed:**
1. ✅ Resource exists, needs update, update succeeds
2. ✅ Resource exists, needs update, update fails
3. ✅ Resource exists, already up-to-date
4. ✅ Resource not found (drift) - returns `None`
5. ✅ API error (not NotFound) - returns error

**Estimated Effort:** 1-2 hours
**Impact:** HIGH - Used by many reconcilers

#### `resolve_required_dependency_id()` - Required Dependency Resolution
**Current Coverage:** 0% (no tests)
**Priority:** 🟠 **HIGH**

**Test Cases Needed:**
1. ✅ Dependency exists, resolution succeeds
2. ✅ Dependency not found - returns error
3. ✅ Invalid reference - returns error
4. ✅ API error - returns error

**Estimated Effort:** 1-2 hours
**Impact:** HIGH - Used by all reconcilers with dependencies

#### `resolve_optional_dependency_id()` - Optional Dependency Resolution
**Current Coverage:** 0% (no tests)
**Priority:** 🟠 **HIGH**

**Test Cases Needed:**
1. ✅ Dependency exists, resolution succeeds
2. ✅ Dependency not found - returns `None` (not error)
3. ✅ Invalid reference - returns error
4. ✅ API error - returns error

**Estimated Effort:** 1-2 hours
**Impact:** HIGH - Used by many reconcilers

#### `update_resource_status()` - Generic Status Update
**Current Coverage:** 0% (no tests)
**Priority:** 🟠 **HIGH**

**Test Cases Needed:**
1. ✅ Status update succeeds
2. ✅ Kube API error - error handling
3. ✅ Status patch creation validation

**Estimated Effort:** 1 hour
**Impact:** MEDIUM - Used by all reconcilers but straightforward

#### `update_tags_if_differ()` - Tag Update Logic
**Current Coverage:** 0% (no tests)
**Priority:** 🟡 **MEDIUM**

**Test Cases Needed:**
1. ✅ Tags differ, update succeeds
2. ✅ Tags differ, update fails
3. ✅ Tags same, update not called
4. ✅ Empty tags handling

**Estimated Effort:** 1 hour
**Impact:** MEDIUM - Used by reconcilers that support tags

#### `convert_tags_to_strings()` - Tag Format Conversion
**Current Coverage:** 0% (no tests)
**Priority:** 🟡 **MEDIUM**

**Test Cases Needed:**
1. ✅ Valid tag JSON conversion
2. ✅ Empty tags
3. ✅ Invalid tag format

**Estimated Effort:** 30 minutes
**Impact:** LOW - Utility function

#### `is_conflict_error()` - Conflict Detection
**Current Coverage:** 0% (no tests)
**Priority:** 🟡 **MEDIUM**

**Test Cases Needed:**
1. ✅ Conflict error detection (409 status)
2. ✅ Non-conflict error (other status codes)

**Estimated Effort:** 30 minutes
**Impact:** MEDIUM - Used for GitOps conflict handling

## Medium Priority (Supporting Functions)

### 2. `check_existing()` - Simple Drift Detection
**Current Coverage:** 0% (no tests)
**Priority:** 🟡 **MEDIUM** (deprecated but still used)

**Test Cases Needed:**
1. ✅ Resource exists
2. ✅ Resource not found (drift)
3. ✅ API error

**Estimated Effort:** 30 minutes
**Impact:** LOW - Deprecated function

### 3. `create_pending_status_patch()` and `create_drift_status_patch()`
**Current Coverage:** 0% (no tests, marked `#[allow(dead_code)]`)
**Priority:** 🟢 **LOW**

**Action:** Verify if these are actually used. If not, consider removing.

## Implementation Plan

### Phase 1: Critical Functions (Highest Impact)
1. `validate_status_and_drift()` - 2-3 hours
2. `check_and_update_existing()` - 1-2 hours
3. `resolve_required_dependency_id()` - 1-2 hours
4. `resolve_optional_dependency_id()` - 1-2 hours

**Total:** 5-9 hours
**Expected Coverage Increase:** ~10-15%

### Phase 2: Supporting Functions
5. `update_resource_status()` - 1 hour
6. `update_tags_if_differ()` - 1 hour
7. `is_conflict_error()` - 30 minutes
8. `convert_tags_to_strings()` - 30 minutes

**Total:** 3 hours
**Expected Coverage Increase:** ~3-5%

### Phase 3: Verification
9. Run `cargo llvm-cov` to get actual coverage numbers
10. Identify any remaining gaps
11. Add edge case tests as needed

**Total:** 1-2 hours

## Testing Strategy

### Mocking Requirements

For testing `reconcile_helpers.rs` functions, we need:

1. **Mock NetBox Client:**
   - Use `MockNetBoxClient` from `netbox-client` crate
   - Configure responses for `get_*` methods
   - Configure responses for `update_*` methods

2. **Mock Kubernetes API:**
   - Use `MockKubeApi` from test utils
   - Configure status patch responses

3. **Test Data:**
   - Use existing test helpers from `test_utils.rs`
   - Create test CRDs with various status states
   - Create test NetBox resources

### Test Structure

Each test should:
1. Set up mocks with expected behavior
2. Call the function under test
3. Assert the result matches expectations
4. Verify mocks were called as expected

## Expected Outcomes

After implementing Phase 1 and Phase 2:

- **Current Estimated Coverage:** ~60-70%
- **Target Coverage:** 80%
- **After Phase 1:** ~70-80% ✅
- **After Phase 2:** ~75-85% ✅

## Next Steps

1. ✅ Fix compilation error in `mock_token_resolver.rs` (completed)
2. Start with `validate_status_and_drift()` tests (highest impact)
3. Add tests incrementally, verifying coverage after each batch
4. Run `cargo llvm-cov` to track progress

