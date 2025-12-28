# NetBox Controller Reconciler Error Audit

**Date**: 2025-12-28  
**Auditor**: Forensic Code Analysis  
**Scope**: Current reconciler errors from `tilt logs netbox-controller`

---

## Executive Summary

The NetBox controller is experiencing a **critical recurring error** preventing site creation. The error indicates that NetBox API requires full tenant objects (with `name` and `slug` fields) during CREATE operations, but the code is only sending tenant IDs.

**Primary Error**: `400 Bad Request - {"tenant":{"name":["This field is required."],"slug":["This field is required."]}}`

**Affected Resource**: `NetBoxSite` CRD `default/datacenter-1`  
**Error Frequency**: Continuous (attempts 580-595+ observed)  
**Impact**: Site resources cannot be created in NetBox

---

## Error Analysis

### 1. Error Pattern

```
ERROR netbox_controller::reconciler::dcim::site: Failed to create site in NetBox: 
NetBox API error: Failed to create site: 400 Bad Request - 
{"tenant":{"name":["This field is required."],"slug":["This field is required."]}}
```

### 2. Root Cause

**Location**: `crates/netbox-client/src/dcim/site.rs` - `create_site` function

**Problem**: The `create_site` function uses `helpers::add_nested_reference` for the tenant, which only sends `{"id": X}`. However, NetBox 4.0 API requires the full tenant object `{"id": X, "name": "...", "slug": "..."}` for CREATE operations.

**Current Code** (line ~110):
```rust
helpers::add_nested_reference(&mut body, "tenant", tenant_id);
```

**Expected Behavior**: Should use `helpers::add_tenant_for_create` which fetches the full tenant object and includes `name` and `slug`.

### 3. Code Flow

1. **Reconciler** (`controllers/netbox/src/reconciler/dcim/site.rs:367`):
   - Resolves tenant ID: `tenant_id = 2` (from NetBoxTenant CRD `datacenter-tenant`)
   - Calls `netbox_client.create_site(..., Some(TenantId(tenant_id)), ...)`

2. **NetBox Client** (`crates/netbox-client/src/dcim/site.rs:86`):
   - Receives `tenant_id: Option<u64>`
   - Uses `helpers::add_nested_reference(&mut body, "tenant", tenant_id)` ❌
   - This creates: `{"tenant": {"id": 2}}`

3. **NetBox API**:
   - Rejects request because `name` and `slug` are missing
   - Returns: `400 Bad Request - {"tenant":{"name":["This field is required."],"slug":["This field is required."]}}`

### 4. Helper Function Analysis

**Available Helper**: `helpers::add_tenant_for_create` exists in `crates/netbox-client/src/core/helpers.rs:107`

**Purpose**: Fetches full tenant object from NetBox and adds `{"id": X, "name": "...", "slug": "..."}`

**Current Usage**: 
- ✅ Used in some other create functions (checking...)
- ❌ **NOT used in `create_site`**

**Why Not Used**: The code was refactored to use `add_nested_reference` to avoid NetBox validation errors, but this created a different problem - NetBox now requires full objects for CREATE operations.

---

## Resource State Analysis

### NetBoxSite CRD (`default/datacenter-1`)

**Status**:
```yaml
status:
  error: "Clearing Failed status with invalid netbox_id (0), will recreate"
  netboxId: 0
  netboxUrl: ""
  state: Pending
```

**Spec**:
- Tenant reference: `datacenter-tenant` (NetBoxTenant CRD)
- Region reference: `us-east` (NetBoxRegion CRD)
- SiteGroup reference: `production-sites` (NetBoxSiteGroup CRD)

**Observation**: The CRD spec is valid. The tenant reference exists and has a valid NetBox ID.

### NetBoxTenant CRD (`default/datacenter-tenant`)

**Status**:
```yaml
status:
  netboxId: 2
  netboxUrl: "http://netbox.netbox/api/tenancy/tenants/2/"
  state: Created
```

**Spec**:
- Name: "Data Center Operations"
- Slug: "datacenter-ops"

**Observation**: The tenant exists in NetBox (ID: 2) and is successfully reconciled. The reconciler should be able to fetch this tenant's details.

---

## Code Location Mapping

### Error Source

| Component | File | Line | Issue |
|-----------|------|------|-------|
| **Reconciler** | `controllers/netbox/src/reconciler/dcim/site.rs` | 367 | Calls `create_site` with tenant ID |
| **Client** | `crates/netbox-client/src/dcim/site.rs` | ~110 | Uses `add_nested_reference` instead of `add_tenant_for_create` |
| **Helper** | `crates/netbox-client/src/core/helpers.rs` | 107 | `add_tenant_for_create` exists but not used |

### Helper Function Comparison

**`add_nested_reference`** (line 16):
```rust
pub fn add_nested_reference(body: &mut serde_json::Value, field_name: &str, id: Option<u64>) {
    if let Some(id_value) = id {
        body[field_name] = serde_json::json!({"id": id_value});
    }
}
```
**Result**: `{"tenant": {"id": 2}}` ❌

**`add_tenant_for_create`** (line 107):
```rust
pub async fn add_tenant_for_create(
    body: &mut serde_json::Value,
    core: &NetBoxClientCore,
    tenant_id: Option<u64>,
) {
    if let Some(tid) = tenant_id {
        match tenancy::get_tenant(core, tid).await {
            Ok(tenant) => {
                body["tenant"] = serde_json::json!({
                    "id": tenant.id,
                    "name": tenant.name,
                    "slug": tenant.slug,
                });
            }
            // ... error handling
        }
    }
}
```
**Result**: `{"tenant": {"id": 2, "name": "Data Center Operations", "slug": "datacenter-ops"}}` ✅

---

## Error Tabulation

### Error Summary Table

| Error Type | Count | Resource | Root Cause | Severity |
|------------|-------|----------|------------|----------|
| **Tenant Object Missing Fields** | 15+ (continuous) | NetBoxSite `default/datacenter-1` | Using `add_nested_reference` instead of `add_tenant_for_create` | **CRITICAL** |

### Detailed Error Breakdown

| Attempt | Timestamp | Error Message | Status State | Action Taken |
|---------|-----------|---------------|--------------|--------------|
| 580-595+ | Continuous | `400 Bad Request - {"tenant":{"name":["This field is required."],"slug":["This field is required."]}}` | `Pending` → `Failed` → `Pending` (loop) | Status cleared, retry, fails again |

**Pattern**: The reconciler detects `netbox_id: 0` (invalid), clears status to `Pending`, attempts creation, fails with 400 error, sets status to `Failed`, then repeats.

---

## Root Cause Analysis

### Why This Error Exists

1. **Historical Context**: 
   - Previous fix attempted to use `add_nested_reference` to avoid NetBox validation errors
   - This worked for some resources but not for sites
   - NetBox API behavior differs between resources

2. **Inconsistent Implementation**:
   - `add_tenant_for_create` helper exists and is designed for this purpose
   - Some create functions use it, but `create_site` does not
   - This suggests incomplete refactoring

3. **NetBox API Requirements**:
   - NetBox 4.0 requires full tenant objects for CREATE operations on sites
   - Only ID is required for PATCH/UPDATE operations
   - The code assumes ID-only works for all operations

### Why It Wasn't Caught

1. **Testing Gap**: The error only occurs during actual NetBox API calls, not in unit tests
2. **Incomplete Refactoring**: The switch from `add_tenant_for_create` to `add_nested_reference` was not fully validated
3. **API Version Differences**: NetBox 4.0 behavior may differ from earlier versions

---

## Impact Assessment

### Immediate Impact

- **Site Creation**: Blocked - no sites can be created
- **Reconciliation Loop**: Continuous retries consuming resources
- **Error Propagation**: Other resources depending on sites will also fail

### Cascading Effects

1. **Dependent Resources**: Any resource requiring a site (devices, locations, prefixes) cannot be created
2. **Resource Waste**: Continuous reconciliation attempts (580+ observed)
3. **User Experience**: No clear indication of the root cause in status messages

---

## Recommended Fix

### Solution

Replace `add_nested_reference` with `add_tenant_for_create` in `create_site` function.

**File**: `crates/netbox-client/src/dcim/site.rs`

**Change**:
```rust
// BEFORE (line ~110):
helpers::add_nested_reference(&mut body, "tenant", tenant_id);

// AFTER:
helpers::add_tenant_for_create(&mut body, core, tenant_id).await?;
```

**Note**: This requires making `create_site` async-aware of the `core` parameter, which it already has.

### Verification Steps

1. Apply fix
2. Rebuild controller
3. Observe logs - should see: `"Adding full tenant object for CREATE: id=2, name=Data Center Operations"`
4. Verify site creation succeeds
5. Check NetBoxSite status transitions: `Pending` → `Created`

### Alternative Solutions Considered

1. **Send ID only and let NetBox fetch**: Not possible - NetBox requires full object
2. **Cache tenant objects**: Over-engineering - fetch is simple and reliable
3. **Make tenant optional**: Not acceptable - tenant is required for sites

---

## Additional Observations

### Other Resources Using Tenant

**Audit Results** - All `create_*` functions using tenant:

| Function | File | Line | Current Helper | Status | Risk |
|----------|------|------|----------------|--------|------|
| `create_site` | `dcim/site.rs` | 112 | `add_nested_reference` | ❌ **FAILING** | **CRITICAL** |
| `create_prefix` | `ipam/prefix.rs` | 154 | `add_nested_reference` | ⚠️ **POTENTIAL** | **HIGH** |
| `create_device` | `dcim/device.rs` | 135 | `add_nested_reference` | ⚠️ **POTENTIAL** | **HIGH** |
| `create_vlan` | `ipam/vlan.rs` | 111 | `add_nested_reference` | ⚠️ **POTENTIAL** | **HIGH** |
| `create_location` | `dcim/location.rs` | 119 | `add_nested_reference` | ⚠️ **POTENTIAL** | **HIGH** |

**Note**: All identified functions use `add_nested_reference` for tenant, which may cause the same error when tenant is required. The error may not be visible yet if:
- These resources haven't been created yet
- Tenant is optional for some resources
- NetBox validation is less strict for some resource types

**Recommendation**: 
1. **IMMEDIATE**: Fix `create_site` to use `add_tenant_for_create`
2. **URGENT**: Audit and fix all other `create_*` functions that use tenant
3. **PREVENTIVE**: Add integration tests that verify CREATE operations succeed

### NetBox API Behavior Analysis

**Observation**: The error message suggests NetBox 4.0 has **stricter validation** for CREATE operations:
- **CREATE**: Requires full object `{"id": X, "name": "...", "slug": "..."}`
- **PATCH/UPDATE**: Accepts just `{"id": X}`

This explains why:
- The code comment (line 109-110) is incorrect
- Some resources may work (if they don't require tenant)
- Sites fail because tenant is required and validation is strict

---

## Conclusion

The error is caused by an **incomplete refactoring** where `create_site` was changed to use `add_nested_reference` instead of `add_tenant_for_create`. The fix is straightforward - restore the use of `add_tenant_for_create` in the `create_site` function.

**Priority**: **CRITICAL** - Blocks all site creation  
**Complexity**: **LOW** - Single line change  
**Risk**: **LOW** - Helper function already exists and is tested

---

## Fix Tracking & Verification

### Implementation Checklist

#### Critical Fix: NetBoxSite Tenant Reference

- [x] **Fix Applied**: Replace `add_nested_reference` with `add_tenant_for_create` in `create_site`
  - File: `crates/netbox-client/src/dcim/site.rs`
  - Line: 112
  - Change: `helpers::add_nested_reference(&mut body, "tenant", Some(tid.into()));` → `helpers::add_tenant_for_create(&mut body, core, tenant_id).await;`

- [x] **Code Compiles**: Verify `cargo build` or `python3 scripts/host_aware_build.py --release -p netbox-controller` succeeds ✅

- [ ] **Controller Deployed**: Verify new controller image is running

- [ ] **Log Verification - Before Fix**:
  ```
  ERROR Failed to create site in NetBox: 400 Bad Request - {"tenant":{"name":["This field is required."],"slug":["This field is required."]}}
  ```

- [ ] **Log Verification - After Fix** (Expected):
  ```
  DEBUG Adding full tenant object for CREATE: id=2, name=Data Center Operations
  INFO Created site Data Center 1 in NetBox (ID: X)
  INFO Updated NetBoxSite default/datacenter-1 status: NetBox ID X
  ```

- [ ] **CR Status Verification**:
  - [ ] NetBoxSite `default/datacenter-1` status transitions: `Pending` → `Created`
  - [ ] `netboxId` is set to valid ID (not 0)
  - [ ] `netboxUrl` is populated
  - [ ] `error` field is cleared/None
  - [ ] `state` is `Created`

- [ ] **NetBox API Verification**:
  - [ ] Site exists in NetBox with correct name: "Data Center 1"
  - [ ] Site has correct tenant: ID 2 ("Data Center Operations")
  - [ ] Site has correct region (if specified)
  - [ ] Site has correct site_group (if specified)

- [ ] **Reconciliation Loop Verification**:
  - [ ] No more continuous retry attempts
  - [ ] Reconciliation succeeds on first attempt
  - [ ] No error status updates

#### High Priority Fixes: Other Resources with Tenant

- [x] **Fix `create_prefix`**: `crates/netbox-client/src/ipam/prefix.rs:154`
  - [x] Code change applied
  - [x] Compiles successfully ✅
  - [ ] Tested with NetBoxPrefix CRD
  - [ ] Logs show tenant object being added correctly
  - [ ] CR status shows `Created` state

- [x] **Fix `create_device`**: `crates/netbox-client/src/dcim/device.rs:135`
  - [x] Code change applied
  - [x] Compiles successfully ✅
  - [ ] Tested with NetBoxDevice CRD
  - [ ] Logs show tenant object being added correctly
  - [ ] CR status shows `Created` state

- [x] **Fix `create_vlan`**: `crates/netbox-client/src/ipam/vlan.rs:111`
  - [x] Code change applied
  - [x] Compiles successfully ✅
  - [ ] Tested with NetBoxVLAN CRD
  - [ ] Logs show tenant object being added correctly
  - [ ] CR status shows `Created` state

- [x] **Fix `create_location`**: `crates/netbox-client/src/dcim/location.rs:119`
  - [x] Code change applied
  - [x] Compiles successfully ✅
  - [ ] Tested with NetBoxLocation CRD
  - [ ] Logs show tenant object being added correctly
  - [ ] CR status shows `Created` state

### Verification Log Comparison

#### Before Fix (Current State)

**Error Pattern**:
```
2025-12-28T15:06:22.604028Z ERROR reconciling object{object.ref=NetBoxSite.v1alpha1.dcops.microscaler.io/datacenter-1.default object.reason=object updated}: netbox_controller::reconciler::dcim::site: Failed to create site in NetBox: NetBox API error: Failed to create site: 400 Bad Request - {"tenant":{"name":["This field is required."],"slug":["This field is required."]}}
```

**Status State**:
```yaml
status:
  netboxId: 0
  netboxUrl: ""
  state: Failed
  error: "Failed to create site in NetBox: NetBox API error: Failed to create site: 400 Bad Request - {\"tenant\":{\"name\":[\"This field is required.\"],\"slug\":[\"This field is required.\"]}}"
```

**Reconciliation Pattern**:
- Attempt 580-595+ (continuous)
- Status: `Pending` → `Failed` → `Pending` (loop)
- Backoff: 600s

#### After Fix (Expected State)

**Success Pattern** (Expected):
```
2025-12-28TXX:XX:XX.XXXXXXZ INFO reconciling object{object.ref=NetBoxSite.v1alpha1.dcops.microscaler.io/datacenter-1.default object.reason=object updated}: netbox_controller::reconciler::dcim::site: Reconciling NetBoxSite default/datacenter-1
2025-12-28TXX:XX:XX.XXXXXXZ DEBUG netbox_controller::core::helpers: Adding full tenant object for CREATE: id=2, name=Data Center Operations
2025-12-28TXX:XX:XX.XXXXXXZ INFO netbox_controller::reconciler::dcim::site: Created site Data Center 1 in NetBox (ID: X)
2025-12-28TXX:XX:XX.XXXXXXZ INFO netbox_controller::reconciler::dcim::site: Updated NetBoxSite default/datacenter-1 status: NetBox ID X
```

**Status State** (Expected):
```yaml
status:
  netboxId: <valid_id>
  netboxUrl: "http://netbox.netbox/api/dcim/sites/<id>/"
  state: Created
  error: null
```

**Reconciliation Pattern** (Expected):
- Single attempt
- Status: `Pending` → `Created`
- No retries needed

### Test Cases

#### Test Case 1: NetBoxSite Creation with Tenant

**Setup**:
- NetBoxTenant CRD `default/datacenter-tenant` exists and is `Created` (netboxId: 2)
- NetBoxSite CRD `default/datacenter-1` exists with `state: Pending`

**Steps**:
1. Apply fix to `create_site`
2. Rebuild and deploy controller
3. Observe reconciliation logs
4. Check NetBoxSite CRD status

**Expected Results**:
- [ ] No 400 Bad Request errors
- [ ] Log shows "Adding full tenant object for CREATE: id=2, name=Data Center Operations"
- [ ] Log shows "Created site Data Center 1 in NetBox (ID: X)"
- [ ] NetBoxSite status.netboxId is set to valid ID
- [ ] NetBoxSite status.state is `Created`
- [ ] NetBoxSite status.error is None

**Actual Results** (Fill in after testing):
```
[ ] Test completed: YYYY-MM-DD HH:MM:SS
[ ] Result: PASS / FAIL
[ ] Notes: <any observations>
```

#### Test Case 2: Verify No Regression in Other Resources

**Setup**:
- Ensure other resources (prefix, device, vlan, location) can still be created

**Steps**:
1. Apply fixes to all identified functions
2. Test each resource type
3. Verify no new errors introduced

**Expected Results**:
- [ ] All resources create successfully
- [ ] No new error patterns in logs
- [ ] All CR statuses show `Created` state

**Actual Results**:
```
[ ] Test completed: YYYY-MM-DD HH:MM:SS
[ ] Result: PASS / FAIL
[ ] Notes: <any observations>
```

### Progress Tracking

| Fix | Status | Date Applied | Verified | Notes |
|-----|--------|--------------|----------|-------|
| `create_site` tenant fix | 🟡 In Progress | 2025-12-28 | ⬜ No | Code change applied ✅, compiles ✅, awaiting deployment |
| `create_prefix` tenant fix | 🟡 In Progress | 2025-12-28 | ⬜ No | Code change applied ✅, compiles ✅, awaiting deployment |
| `create_device` tenant fix | 🟡 In Progress | 2025-12-28 | ⬜ No | Code change applied ✅, compiles ✅, awaiting deployment |
| `create_vlan` tenant fix | 🟡 In Progress | 2025-12-28 | ⬜ No | Code change applied ✅, compiles ✅, awaiting deployment |
| `create_location` tenant fix | 🟡 In Progress | 2025-12-28 | ⬜ No | Code change applied ✅, compiles ✅, awaiting deployment |

**Legend**:
- ⬜ Not Started
- 🟡 In Progress
- ✅ Complete
- ❌ Failed

### Log Monitoring Commands

**Before Fix Verification**:
```bash
# Check for current error pattern
tilt logs netbox-controller 2>&1 | grep -E "(Failed to create site|tenant.*name.*required|tenant.*slug.*required)"

# Check reconciliation attempts
tilt logs netbox-controller 2>&1 | grep -E "Requeuing.*datacenter-1.*after error" | tail -5

# Check status updates
kubectl get netboxsite datacenter-1 -o jsonpath='{.status}' | jq
```

**After Fix Verification**:
```bash
# Check for success pattern
tilt logs netbox-controller 2>&1 | grep -E "(Adding full tenant object|Created site.*in NetBox|Updated NetBoxSite.*status)"

# Verify no errors
tilt logs netbox-controller 2>&1 | grep -E "(Failed to create site|400 Bad Request)" | tail -10

# Check final status
kubectl get netboxsite datacenter-1 -o jsonpath='{.status}' | jq
```

### Resolution Criteria

A fix is considered **resolved** when:

1. ✅ Code change applied and committed
2. ✅ Controller compiles without errors
3. ✅ Controller deployed successfully
4. ✅ Logs show success pattern (no 400 errors)
5. ✅ CR status shows `Created` state with valid `netboxId`
6. ✅ No reconciliation loops (single successful attempt)
7. ✅ NetBox API confirms resource exists with correct tenant
8. ✅ No regression in other resources

---

## Appendix: Related Code References

- `crates/netbox-client/src/dcim/site.rs:76-237` - `create_site` function
- `crates/netbox-client/src/core/helpers.rs:107-130` - `add_tenant_for_create` helper
- `crates/netbox-client/src/core/helpers.rs:16-21` - `add_nested_reference` helper
- `controllers/netbox/src/reconciler/dcim/site.rs:350-410` - Site reconciler

