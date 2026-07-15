# Missing Resources Audit

**Date**: 2025-12-28  
**Purpose**: Audit failed reconciliations to identify missing CRDs, CRs, and Reconcilers

## Summary

After analyzing Tilt logs and codebase, the following issues were identified:

### 1. ✅ FIXED: NetBoxSite Tenant Reference Issue

**Error**: `Failed to create site in NetBox: 400 Bad Request - {"tenant":{"name":["This field is required."],"slug":["This field is required."]}}`

**Root Cause**: The `create_site` function was using `add_nested_reference` which only sends `{"id": X}`, but NetBox requires the full tenant object (`{"id": X, "name": "...", "slug": "..."}`) for CREATE operations on sites.

**NetBox Source Code Validation**:
- Checked `/Users/casibbald/Workspace/microscaler/netbox/netbox/netbox/api/serializers/base.py`
- When `nested=True`, `BaseModelSerializer.to_internal_value()` calls `get_related_object_by_attrs()` which accepts:
  - Integer PK: `1` or `"1"`
  - Dictionary: `{"id": 1}` or `{"name": "...", "slug": "..."}`
- However, the error suggests NetBox is validating the tenant object as if creating a new tenant when only `{"id": X}` is provided
- The `SiteSerializer` uses `TenantSerializer(nested=True, required=False, allow_null=True)` which should accept `{"id": X}`
- **Conclusion**: Despite NetBox code supporting `{"id": X}`, the actual API behavior requires full objects for tenant references in site CREATE operations

**Fix Applied**: Changed `crates/netbox-client/src/dcim/site.rs` to use `add_tenant_for_create` which fetches the tenant and adds the full object `{"id": X, "name": "...", "slug": "..."}`.

**Status**: ✅ Fixed in commit

---

### 2. ⚠️ MISSING: NetBoxRIR CRD

**Location**: `config/examples/netbox-rir-example.yaml` exists, but no CRD/reconciler

**Impact**: 
- `NetBoxAggregate` resources reference RIR by name (e.g., "ARIN")
- No CRD exists to manage RIRs declaratively
- Aggregate creation fails if RIR doesn't exist in NetBox

**Current Workaround**: 
- RIR is optional in `create_aggregate` (fixed in previous session)
- Aggregates can be created without RIR, but status reflects missing dependency

**Required Work**:
- [ ] Create `NetBoxRIR` CRD in `crates/crds/src/ipam/netbox_rir.rs`
- [ ] Implement reconciler in `controllers/netbox/src/reconciler/ipam/rir.rs`
- [ ] Add NetBox client support for RIR operations
- [ ] Wire into controller and watcher
- [ ] Update `scripts/apply_example_crs.py` to include RIR example

**References**:
- `crates/crds/src/ipam/netbox_aggregate.rs:25` - `pub rir: Option<String>`
- `config/examples/netbox-rir-example.yaml` - Example CR exists

---

### 3. ⚠️ MISSING: NetBoxVLANGroup CRD

**Location**: Referenced in `crates/crds/src/ipam/netbox_vlan.rs:31`

**Impact**:
- `NetBoxVLAN` CRD has optional `group` field that references `NetBoxVLANGroup`
- Comment states: "not yet implemented"
- VLANs can be created without groups, but group functionality is unavailable

**Required Work**:
- [ ] Create `NetBoxVLANGroup` CRD
- [ ] Implement reconciler
- [ ] Add NetBox client support
- [ ] Wire into controller

**References**:
- `crates/crds/src/ipam/netbox_vlan.rs:31` - `/// VLAN group reference (references NetBoxVLANGroup CRD, optional - not yet implemented)`

---

### 4. ⚠️ MISSING: NetBoxTenantGroup CRD

**Location**: Referenced in `crates/crds/src/tenancy/netbox_tenant.rs:36`

**Impact**:
- `NetBoxTenant` CRD has optional `group` field that references `NetBoxTenantGroup`
- Tenants can be created without groups, but group functionality is unavailable

**Required Work**:
- [ ] Create `NetBoxTenantGroup` CRD
- [ ] Implement reconciler
- [ ] Add NetBox client support
- [ ] Wire into controller

**References**:
- `crates/crds/src/tenancy/netbox_tenant.rs:36` - `/// Tenant group reference (references NetBoxTenantGroup CRD, optional)`
- `controllers/netbox/src/reconciler/tenancy.rs:168` - Validation comment: "NetBoxTenantGroup CRD not yet implemented"

---

## Failed Reconciliations Analysis

### NetBoxSite (`default/datacenter-1`)

**Error Pattern**: 
```
Failed to create site in NetBox: 400 Bad Request - {"tenant":{"name":["This field is required."],"slug":["This field is required."]}}
```

**Attempts**: 180+ (continuously requeuing)

**Status**: ✅ Fixed - Site creation now uses `add_tenant_for_create` to send full tenant object

---

### NetBoxAggregate (`default/private-network-aggregate`)

**Error Pattern**: 
```
Failed to create NetBoxAggregate in NetBox: NetBox API error: RIR is required for aggregates but was not provided
```

**Status**: ✅ Fixed - RIR is now optional in `create_aggregate`

**Remaining Issue**: If RIR is specified in CRD but doesn't exist, aggregate creation succeeds but RIR dependency is missing. NetBoxRIR CRD would solve this.

---

## Recommendations

### Priority 1: NetBoxRIR CRD
- **Reason**: Already has example CR, referenced by aggregates, blocking full GitOps workflow
- **Effort**: Medium (CRD + reconciler + client support)

### Priority 2: NetBoxTenantGroup CRD
- **Reason**: Referenced by tenants, enables hierarchical tenant organization
- **Effort**: Medium (CRD + reconciler + client support)

### Priority 3: NetBoxVLANGroup CRD
- **Reason**: Referenced by VLANs, enables VLAN organization
- **Effort**: Medium (CRD + reconciler + client support)

---

## Next Steps

1. ✅ Fix NetBoxSite tenant reference (COMPLETED)
2. [ ] Create NetBoxRIR CRD and reconciler
3. [ ] Create NetBoxTenantGroup CRD and reconciler
4. [ ] Create NetBoxVLANGroup CRD and reconciler
5. [ ] Update `scripts/apply_example_crs.py` to include all new examples

