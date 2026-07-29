# Fix Verification Plan

**Date:** 2025-12-25  
**Purpose:** Systematic verification of all fixes from FAILURE_AUDIT.md

## Issue #1: NetBoxSiteGroup Deserialization Fix

### Fix Applied
- **File:** `crates/netbox-client/src/models.rs`
- **Change:** Added `#[serde(default)]` to `prefix_count` field in `SiteGroup` struct
- **Line:** ~730

### Verification Steps
1. ✅ **Code Review:** Verified `#[serde(default)]` is present on `prefix_count` field
2. ✅ **Compilation:** Code compiles successfully
3. ⏳ **Runtime Test:** Deploy and verify fallback query succeeds
4. ⏳ **Expected Behavior:** 
   - Fallback query `query_site_groups(&[], true)` should deserialize successfully
   - Site group should be found and CR status updated with NetBox ID

### Test Case
```rust
// Simulated NetBox API response (missing prefix_count)
{
  "results": [{
    "id": 1,
    "name": "Production Sites",
    "slug": "production-sites",
    "site_count": 0,
    // prefix_count missing - should default to 0
    "_depth": 0
  }]
}
```

**Expected:** Deserialization succeeds, `prefix_count` defaults to `0`

---

## Issue #2: NetBoxRegion Deserialization Fix

### Fix Applied
- **File:** `crates/netbox-client/src/models.rs`
- **Change:** Added `#[serde(default)]` to `prefix_count` field in `Region` struct
- **Line:** ~698

### Verification Steps
1. ✅ **Code Review:** Verified `#[serde(default)]` is present on `prefix_count` field
2. ✅ **Compilation:** Code compiles successfully
3. ⏳ **Runtime Test:** Deploy and verify fallback query succeeds
4. ⏳ **Expected Behavior:**
   - Fallback query `query_regions(&[], true)` should deserialize successfully
   - Region should be found and CR status updated with NetBox ID

### Test Case
```rust
// Simulated NetBox API response (missing prefix_count)
{
  "results": [{
    "id": 1,
    "name": "US East",
    "slug": "us-east",
    "site_count": 0,
    // prefix_count missing - should default to 0
    "_depth": 0
  }]
}
```

**Expected:** Deserialization succeeds, `prefix_count` defaults to `0`

---

## Issue #3: NetBoxDevice Idempotency Fix

### Fix Applied
- **File:** `controllers/netbox/src/reconciler.rs`
- **Change:** Added idempotency handling in `reconcile_netbox_device` function
- **Lines:** 3231-3278

### Verification Steps
1. ✅ **Code Review:** Verified idempotency logic is present
2. ✅ **Compilation:** Code compiles successfully
3. ⏳ **Runtime Test:** Deploy and verify device reconciliation handles "already exists" errors
4. ⏳ **Expected Behavior:**
   - When device creation fails with "already exists" or "asset tag" error:
     - Query NetBox for existing device by `asset_tag` (if provided)
     - Fallback to query by `name` if asset_tag query fails
     - Update CR status with existing NetBox ID
     - Treat as successful reconciliation

### Test Scenarios

#### Scenario 1: Device exists with asset_tag
```
1. Device CR created with asset_tag="RPI-001"
2. Device already exists in NetBox with same asset_tag
3. Creation fails: "device with this asset tag already exists"
4. Controller queries by asset_tag
5. Finds existing device (ID: 5)
6. Updates CR status with netbox_id=5, state=Created
```

#### Scenario 2: Device exists with name only
```
1. Device CR created with name="talos-control-plane-01"
2. Device already exists in NetBox with same name
3. Creation fails: "device with this name already exists"
4. Controller queries by name
5. Finds existing device (ID: 3)
6. Updates CR status with netbox_id=3, state=Created
```

#### Scenario 3: Device not found after "already exists" error
```
1. Device CR created
2. Creation fails: "already exists"
3. Controller queries by asset_tag - not found
4. Controller queries by name - not found
5. Returns error: "Device already exists but could not retrieve it"
```

---

## Issue #4 & #5: Dependency Blocking (Auto-Resolve)

### Status
- **Issue #4:** NetBoxInterface - Blocked by Issue #3
- **Issue #5:** NetBoxMACAddress - Blocked by Issue #3

### Expected Resolution
Once Issue #3 is fixed and NetBoxDevice reconciles successfully:
- NetBoxInterface will automatically reconcile (device dependency satisfied)
- NetBoxMACAddress will automatically reconcile (device dependency satisfied)

### Verification Steps
1. ⏳ **Verify Issue #3 fix works**
2. ⏳ **Verify Device CR has netbox_id in status**
3. ⏳ **Verify Interface CR reconciles successfully**
4. ⏳ **Verify MACAddress CR reconciles successfully**

---

## Testing Checklist

### Pre-Deployment
- [x] All fixes compile successfully
- [x] Code review completed for all fixes
- [x] Fixes documented in FAILURE_AUDIT.md

### Post-Deployment
- [ ] Issue #1: NetBoxSiteGroup fallback query succeeds
- [ ] Issue #2: NetBoxRegion fallback query succeeds
- [ ] Issue #3: NetBoxDevice handles "already exists" gracefully
- [ ] Issue #4: NetBoxInterface reconciles (after #3)
- [ ] Issue #5: NetBoxMACAddress reconciles (after #3)

### Verification Commands
```bash
# Check SiteGroup CR status
kubectl get netboxsitegroup production-sites -o jsonpath='{.status.netboxId}'

# Check Region CR status
kubectl get netboxregion us-east -o jsonpath='{.status.netboxId}'

# Check Device CR status
kubectl get netboxdevice talos-control-plane-01 -o jsonpath='{.status.netboxId}'

# Check Interface CR status
kubectl get netboxinterface talos-control-plane-01-eth0 -o jsonpath='{.status.netboxId}'

# Check MACAddress CR status
kubectl get netboxmacaddress talos-control-plane-01-eth0-mac -o jsonpath='{.status.netboxId}'
```

---

## Next Steps

1. **Deploy updated controller** with all fixes
2. **Monitor logs** for successful reconciliation
3. **Verify CR statuses** have netbox_id populated
4. **Update FAILURE_AUDIT.md** with test results
5. **Mark issues as resolved** once verified

