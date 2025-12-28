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
| `create_vlan` | `ipam/vlan.rs` | ~120 | `add_nested_reference` | ⚠️ **POTENTIAL** | **HIGH** |
| `create_location` | `dcim/location.rs` | ~120 | `add_nested_reference` | ⚠️ **POTENTIAL** | **HIGH** |

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

## Appendix: Related Code References

- `crates/netbox-client/src/dcim/site.rs:76-237` - `create_site` function
- `crates/netbox-client/src/core/helpers.rs:107-130` - `add_tenant_for_create` helper
- `crates/netbox-client/src/core/helpers.rs:16-21` - `add_nested_reference` helper
- `controllers/netbox/src/reconciler/dcim/site.rs:350-410` - Site reconciler

