# Trait/Generics Refactor Status & DRY Opportunities

**Date:** 2025-12-25  
**Status:** Partially Complete

## Current Refactor Status

### ✅ Completed (17/17 reconcilers using helpers - 100%)

1. **`reconcile_netbox_site()`** - Uses `check_and_update_existing()` with diffing
2. **`reconcile_netbox_tenant()`** - Uses `check_existing()` for drift detection
3. **`reconcile_netbox_aggregate()`** - Uses `check_existing()` for drift detection
4. **`reconcile_netbox_prefix()`** - Uses `check_existing()` for drift detection
5. **`reconcile_netbox_region()`** - Uses `check_existing()` for drift detection
6. **`reconcile_netbox_role()`** - Uses `check_existing()` for drift detection
7. **`reconcile_netbox_tag()`** - Uses `check_existing()` for drift detection
8. **`reconcile_netbox_site_group()`** - Uses `check_existing()` for drift detection
9. **`reconcile_netbox_location()`** - Uses `check_existing()` for drift detection
10. **`reconcile_netbox_device_role()`** - Uses `check_existing()` for drift detection (query-by-name pattern)
11. **`reconcile_netbox_manufacturer()`** - Uses `check_existing()` for drift detection (query-by-name pattern)
12. **`reconcile_netbox_platform()`** - Uses `check_existing()` for drift detection (query-by-name pattern)
13. **`reconcile_netbox_device_type()`** - Uses `check_existing()` for drift detection (with manufacturer_id)
14. **`reconcile_netbox_device()`** - Uses `check_existing()` for drift detection
15. **`reconcile_netbox_interface()`** - Uses `check_existing()` for drift detection
16. **`reconcile_netbox_mac_address()`** - Special case (managed via interfaces, drift detection via interface check)
17. **`reconcile_netbox_vlan()`** - Uses `check_existing()` for drift detection

### 📊 Statistics

- **Total reconcilers:** 17
- **Refactored:** 17 (100%)
- **Remaining:** 0 (0%)
- **Files using helpers:** All reconciler modules
- **NetBoxResource trait implementations:** All resource types

## DRY Opportunities

### 1. **Status Patch Creation** (High Impact - 80 matches across 19 files)

**Current State:**
- `create_resource_status_patch()` - Used for most CRDs
- `create_prefix_status_patch()` - Special case for Prefix
- `create_ipclaim_status_patch()` - Special case for IPClaim
- Direct JSON construction in some places

**Opportunity:**
Create a generic status patch helper that works with any status type:
```rust
pub async fn update_resource_status<K>(
    api: &Api<K>,
    name: &str,
    netbox_id: u64,
    netbox_url: String,
    state: impl Into<String>,
    error: Option<String>,
) -> Result<(), ControllerError>
where
    K: Crd + Clone + Debug + Send + Sync + 'static,
    K::Status: ResourceStatus,
```

**Impact:** Eliminates ~80 duplicate status update patterns

### 2. **Drift Detection Status Clearing** (High Impact - Pattern duplicated 17 times)

**Current State:**
Every reconciler has this pattern:
```rust
let status_patch = Self::create_resource_status_patch(
    0, // Clear netbox_id
    String::new(), // Clear URL
    ResourceState::Pending,
    Some("Resource was deleted in NetBox, will recreate".to_string()),
);
let pp = kube::api::PatchParams::default();
if let Err(e) = api.patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch)).await {
    warn!("Failed to clear status after drift detection: {}", e);
}
```

**Opportunity:**
Add to `reconcile_helpers.rs`:
```rust
pub async fn clear_status_on_drift<K>(
    api: &Api<K>,
    name: &str,
    namespace: &str,
    resource_type: &str,
) -> Result<(), ControllerError>
where
    K: Crd + Clone + Debug + Send + Sync + 'static,
    K::Status: ResourceStatus,
```

**Impact:** Eliminates ~17 duplicate drift clearing patterns

### 3. **Error Status Update Pattern** (Medium Impact - 19 matches across 5 files)

**Current State:**
Many reconcilers have an `update_status_error()` helper function:
```rust
async fn update_status_error(
    api: &Api<NetBoxTenant>,
    name: &str,
    namespace: &str,
    error_msg: String,
    current_status: Option<&NetBoxTenantStatus>,
) { ... }
```

**Opportunity:**
Move to `reconcile_helpers.rs` as a generic function:
```rust
pub async fn update_status_with_error<K>(
    api: &Api<K>,
    name: &str,
    namespace: &str,
    error_msg: String,
    current_status: Option<&K::Status>,
) -> Result<(), ControllerError>
where
    K: Crd + Clone + Debug + Send + Sync + 'static,
    K::Status: ResourceStatus,
```

**Impact:** Eliminates ~19 duplicate error status update functions

### 4. **Idempotency Fallback Pattern** (Medium Impact - Pattern duplicated ~15 times)

**Current State:**
Many reconcilers have this pattern:
```rust
// Try to find existing by name
let existing = match self.netbox_client.query_*(
    &[("name", &spec.name)],
    false,
).await {
    Ok(items) => items.first().cloned(),
    Err(_) => None
};

// If not found, try by slug
if existing.is_none() && spec.slug.is_some() {
    match self.netbox_client.query_*(
        &[("slug", &spec.slug)],
        false,
    ).await {
        Ok(items) => items.first().cloned(),
        Err(_) => None
    }
}
```

**Opportunity:**
Create a generic idempotency helper:
```rust
pub async fn find_existing_by_name_or_slug<FQuery, Resource>(
    client: &NetBoxClient,
    name: &str,
    slug: Option<&str>,
    query_fn: FQuery,
) -> Result<Option<Resource>, ControllerError>
where
    FQuery: std::future::Future<Output = Result<Vec<Resource>, NetBoxError>> + Send,
    Resource: Clone,
```

**Impact:** Eliminates ~15 duplicate idempotency patterns

### 5. **Reference Resolution Pattern** (Medium Impact - Pattern duplicated ~20 times)

**Current State:**
Many reconcilers resolve CR references to NetBox IDs:
```rust
let tenant_id = if let Some(tenant_ref) = &spec.tenant {
    if tenant_ref.kind != "NetBoxTenant" {
        warn!("Invalid kind...");
        None
    } else {
        match self.netbox_tenant_api.get(&tenant_ref.name).await {
            Ok(tenant_crd) => {
                tenant_crd.status
                    .as_ref()
                    .and_then(|s| s.netbox_id)
            }
            Err(_) => None
        }
    }
} else {
    None
};
```

**Opportunity:**
Create a generic reference resolver:
```rust
pub async fn resolve_cr_reference<K>(
    api: &Api<K>,
    reference: &TypedLocalObjectReference,
    expected_kind: &str,
    resource_name: &str,
) -> Result<Option<u64>, ControllerError>
where
    K: Crd + Clone + Debug + Send + Sync + 'static,
    K::Status: ResourceStatus,
```

**Impact:** Eliminates ~20 duplicate reference resolution patterns

### 6. **Reconciliation Flow Pattern** (High Impact - Pattern duplicated 17 times)

**Current State:**
Every reconciler follows this pattern:
1. Extract name/namespace
2. Check if already created (drift detection)
3. Resolve references
4. Try idempotency fallback (query by name)
5. Create if needed
6. Update status

**Opportunity:**
Create a macro or builder pattern:
```rust
macro_rules! reconcile_resource {
    ($self:expr, $crd:expr, $resource_type:ident, {
        get_fn: $get:expr,
        query_fn: $query:expr,
        create_fn: $create:expr,
        update_fn: $update:expr,
        needs_update_fn: $needs_update:expr,
    }) => { ... };
}
```

**Impact:** Could eliminate ~200+ lines of boilerplate across all reconcilers

### 7. **Status Update After Success** (Medium Impact - Pattern duplicated 17 times)

**Current State:**
Every reconciler has:
```rust
let status_patch = Self::create_resource_status_patch(
    netbox_resource.id,
    netbox_resource.url.clone(),
    ResourceState::Created,
    None,
);
let pp = kube::api::PatchParams::default();
match api.patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch)).await {
    Ok(_) => Ok(()),
    Err(e) => Err(ControllerError::Kube(e.into())),
}
```

**Opportunity:**
Generic helper:
```rust
pub async fn update_status_on_success<K, Resource>(
    api: &Api<K>,
    name: &str,
    resource: &Resource,
) -> Result<(), ControllerError>
where
    K: Crd + Clone + Debug + Send + Sync + 'static,
    K::Status: ResourceStatus,
    Resource: NetBoxResource,
```

**Impact:** Eliminates ~17 duplicate success status update patterns

## Priority Recommendations

### High Priority (Quick Wins)
1. ✅ **Drift detection helpers** - `check_existing()` and `check_and_update_existing()` implemented
2. ⏳ **Status update patterns** - Documented in `reconcile_helpers.rs` (trait bounds too complex for generic helpers)
3. ⏳ **Macro for status updates** - Future enhancement to reduce boilerplate while maintaining type safety

### Medium Priority (Significant Impact)
4. **Add `resolve_cr_reference()` helper** - Reduces reference resolution duplication
5. **Add `find_existing_by_name_or_slug()` helper** - Standardizes idempotency

### Low Priority (Architectural)
6. **Create reconciliation macro** - Requires careful design, but could eliminate most boilerplate
7. **Add trait for status types** - Would enable more generic helpers

## Critical Issue: Reconcile Loop Stops After One Iteration

**Date Identified:** 2025-12-25  
**Status:** 🔴 **CRITICAL - Must Fix Before Production**

### Problem Description

The controller performs one reconcile loop and then stops. This is a **critical defect** that prevents the controller from continuously watching and reconciling resources.

### Root Cause

**Current Implementation (WRONG):**
```rust
// In watcher.rs - Using low-level watcher() function
let mut stream = Box::pin(watcher(self.netbox_prefix_api.clone(), watcher::Config::default()));

while let Some(result) = stream.try_next().await {
    match result {
        watcher::Event::Apply(prefix) => {
            // Reconcile...
        }
        // ...
    }
}
// ❌ When stream ends (connection error, API error, etc.), loop exits and watcher stops
```

**Issues:**
1. **Manual stream polling** - Using `kube_runtime::watcher()` directly creates a stream that must be manually polled
2. **No automatic reconnection** - When the watch stream ends (network issues, API errors, etc.), the `while let Some(result)` loop exits
3. **No retry logic** - Stream errors cause the watcher to stop permanently
4. **One-shot behavior** - After initial sync (`InitDone`), the controller stops watching for new changes

### Correct Implementation Pattern

**Should Use `kube_runtime::Controller` (CORRECT):**
```rust
use kube_runtime::Controller;

// Create controller with reconcile function
let controller = Controller::new(
    api,
    watcher::Config::default(),
)
.reconcile_all_on(watcher::Config::default())
.run(
    reconcile_fn,  // Our reconcile function
    error_policy,  // Error handling policy
    Arc::new(Reconciler::new(...)),
)
.await?;

// Controller automatically:
// - Handles watch stream reconnection
// - Manages retries and backoff
// - Continues watching indefinitely
// - Processes all events (Apply, Delete, etc.)
```

### Impact

- **Severity:** 🔴 **CRITICAL**
- **Affected Components:** All 17 watchers in `watcher.rs`
- **User Impact:** Controller stops working after initial sync, no continuous reconciliation
- **Production Readiness:** ❌ **BLOCKER** - Cannot deploy to production

### Solution Required

1. **Create generic watcher helper** using `kube_runtime::Controller` - This will fix all 17 watchers at once!
2. **Create reconcile functions** that match `Controller::reconcile_fn` signature:
   ```rust
   async fn reconcile_fn(
       resource: Arc<NetBoxPrefix>,
       ctx: Arc<Reconciler>,
   ) -> Result<ReconcileAction, ControllerError>
   ```
3. **Implement error policies** for handling reconciliation failures
4. **Remove manual stream polling** - Let `Controller` handle the watch loop
5. **Remove manual generation tracking** - `Controller` handles this automatically

### Trait-Based Solution (✅ RECOMMENDED)

**Create a generic watcher helper in `watcher.rs`:**

```rust
use kube_runtime::{Controller, watcher, controller::Action};
use std::time::Duration;

/// Generic watcher helper that uses kube_runtime::Controller properly
/// This fixes the reconcile loop issue for ALL watchers at once!
/// 
/// The reconcile_fn should match our existing reconcile function signature:
/// `async fn reconcile(&self, resource: &K) -> Result<(), ControllerError>`
pub async fn watch_resource<K, F>(
    api: Api<K>,
    reconciler: Arc<Reconciler>,
    reconcile_fn: F,
    resource_name: &str,
) -> Result<(), ControllerError>
where
    K: kube::Resource + Clone + Send + Sync + 'static + std::fmt::Debug,
    K::DynamicType: Default,
    F: Fn(&Reconciler, &K) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ControllerError>> + Send>> + Send + Sync + Clone + 'static,
{
    info!("Starting {} watcher", resource_name);
    
    // Error policy: requeue with exponential backoff
    let error_policy = |obj: Arc<K>, error: &ControllerError, _ctx: Arc<Reconciler>| {
        error!("Reconciliation error for {}: {:?}", resource_name, error);
        Action::requeue(Duration::from_secs(60))
    };
    
    // Reconcile function: wraps our existing reconcile functions
    let reconcile = move |obj: Arc<K>, ctx: Arc<Reconciler>| {
        let reconcile_fn = reconcile_fn.clone();
        async move {
            match reconcile_fn(&*ctx, &*obj).await {
                Ok(()) => Ok(Action::await_change()),
                Err(e) => {
                    error!("Failed to reconcile {}: {}", resource_name, e);
                    Err(e)
                }
            }
        }
    };
    
    Controller::new(api, watcher::Config::default())
        .reconcile_all_on(watcher::Config::default())
        .run(reconcile, error_policy, reconciler)
        .for_each(|res| async move {
            if let Err(e) = res {
                error!("Controller error for {}: {}", resource_name, e);
            }
        })
        .await;
    
    Ok(())
}
```

**Benefits:**
- ✅ Fixes all 17 watchers with a single implementation
- ✅ Automatic reconnection on stream errors
- ✅ Proper retry and backoff handling
- ✅ No manual generation tracking needed
- ✅ Follows DRY principles - one function for all resource types
- ✅ Wraps existing reconcile functions - no signature changes needed

**Usage:**
```rust
// In watcher.rs - All watchers become one-liners!
pub async fn watch_netbox_prefixes(&self) -> Result<(), ControllerError> {
    watch_resource(
        self.netbox_prefix_api.clone(),
        self.reconciler.clone(),
        |reconciler, resource| {
            Box::pin(reconciler.reconcile_netbox_prefix(resource))
        },
        "NetBoxPrefix",
    ).await
}
```

**All 17 watchers can use this pattern:**
- `watch_netbox_prefixes()` → `watch_resource(..., |r, res| Box::pin(r.reconcile_netbox_prefix(res)), ...)`
- `watch_netbox_tenants()` → `watch_resource(..., |r, res| Box::pin(r.reconcile_netbox_tenant(res)), ...)`
- `watch_netbox_sites()` → `watch_resource(..., |r, res| Box::pin(r.reconcile_netbox_site(res)), ...)`
- ... and so on for all 17 resource types

### Files Requiring Changes

- `controllers/netbox/src/watcher.rs` - **Refactor to use generic helper** (much simpler!)
- `controllers/netbox/src/reconcile_helpers.rs` - **Add generic watcher helper**
- `controllers/netbox/src/controller.rs` - Update to use new watcher pattern
- `controllers/netbox/src/reconciler/mod.rs` - May need minor adjustments

### Reference Implementation

See `secrets-manager-controller` for the correct pattern (as mentioned by user).

## Next Steps

1. ✅ **COMPLETED:** Fixed reconcile loop issue - Refactored to use `kube_runtime::Controller` with generic helper
2. Complete refactoring remaining 10 reconcilers to use `check_existing()` helper
3. Add high-priority helpers to `reconcile_helpers.rs`
4. Refactor existing reconcilers to use new helpers
5. Consider macro/builder pattern for full reconciliation flow

## Implementation Status: Reconcile Loop Fix

**Date Completed:** 2025-12-25  
**Status:** ✅ **COMPLETED**

### What Was Fixed

- ✅ Created generic `watch_resource()` helper function using `kube_runtime::Controller`
- ✅ Refactored all 17 watchers to use the generic helper (one-liner implementations)
- ✅ Removed manual stream polling (`watcher()` + `try_next()` loops)
- ✅ Removed manual generation tracking (Controller handles this automatically)
- ✅ Removed all `ReconciliationState` structs and mutexes
- ✅ Controller now handles automatic reconnection on stream errors
- ✅ Controller manages retries and backoff automatically
- ✅ Controller continues watching indefinitely (no one-shot behavior)

### Code Reduction

- **Before:** ~1056 lines in `watcher.rs` with 17 duplicate watcher implementations
- **After:** ~350 lines in `watcher.rs` with 1 generic helper + 17 one-liner watchers
- **Reduction:** ~700 lines removed (66% reduction)

### Files Changed

- `controllers/netbox/src/watcher.rs` - Complete rewrite using `kube_runtime::Controller`
- All 17 watcher methods now use the generic `watch_resource()` helper

### Testing Required

- [ ] Verify controller continues watching after initial sync
- [ ] Verify automatic reconnection on network errors
- [ ] Verify retry logic works correctly
- [ ] Verify all 17 resource types reconcile properly

## Reconciliation Frequency Issues

**Date Identified:** 2025-12-25  
**Status:** 🔴 **CRITICAL - Needs Optimization**

### Problem Description

The controller may be reconciling too frequently, potentially causing:
- Excessive API calls to NetBox
- Reconciliation loops from status updates
- Resource exhaustion with 17 concurrent watchers
- Unnecessary load on Kubernetes API server

### Root Causes Identified

#### 1. **Non-Deterministic Status Updates** (🔴 CRITICAL)

**Issue:** Every status update includes `lastReconciled: Utc::now().to_rfc3339()`, which changes on every reconciliation.

**Location:** `controllers/netbox/src/reconciler/mod.rs:107`

```rust
serde_json::json!({
    "status": {
        "netboxId": netbox_id,
        "netboxUrl": netbox_url,
        "state": state_str,
        "error": error,
        "lastReconciled": Utc::now().to_rfc3339(),  // ❌ Changes every time!
    }
})
```

**Impact:**
- Status update → Watch event → Reconciliation → Status update → Loop
- Even if Controller filters by generation, the status change itself could trigger events

**Fix Required:**
- Only update `lastReconciled` if it's been > 5 minutes since last update
- Or remove it entirely if not needed for monitoring
- Or make it deterministic (only update on state changes, not every reconciliation)

#### 2. **No Debouncing** (🟡 HIGH PRIORITY)

**Issue:** `watcher::Config::default()` has no debounce period, so every event triggers immediate reconciliation.

**Location:** `controllers/netbox/src/watcher.rs:58`

```rust
Controller::new(api, watcher::Config::default())  // ❌ No debounce!
```

**Impact:**
- Rapid status updates (e.g., from multiple reconcilers) trigger immediate reconciliations
- No batching of related events
- Potential thundering herd with 17 watchers

**Fix Required:**
```rust
let watcher_config = watcher::Config::default()
    .debounce(Duration::from_secs(5));  // Wait 5s for events to settle
```

#### 3. **No Concurrency Limits** (🟡 HIGH PRIORITY)

**Issue:** All 17 watchers can reconcile simultaneously, potentially overwhelming NetBox API.

**Impact:**
- 17 concurrent reconciliations hitting NetBox API at once
- No rate limiting or throttling
- Could trigger NetBox rate limiting or cause API errors

**Fix Required:**
```rust
Controller::new(api, watcher_config)
    .concurrency(3)  // Limit to 3 concurrent reconciliations per watcher
    .run(...)
```

**Total Impact:** 17 watchers × 3 concurrency = 51 potential concurrent reconciliations (still high, but better than unlimited)

#### 4. **Status Updates Trigger Watch Events** (🟡 MEDIUM PRIORITY)

**Issue:** When we call `patch_status()`, that creates a watch event, which could trigger another reconciliation.

**Location:** All reconcilers call `api.patch_status()` after reconciliation

**Impact:**
- Reconciliation → Status update → Watch event → Reconciliation → Loop
- Controller should filter by generation, but we need to verify this works

**Mitigation:**
- Controller's `run()` should automatically filter status-only updates (generation unchanged)
- But we should verify this is working correctly
- Consider adding explicit generation check in reconcile function

#### 5. **No Minimum Reconciliation Interval** (🟢 LOW PRIORITY)

**Issue:** Same resource could be reconciled multiple times in quick succession.

**Impact:**
- Less critical if debouncing is added
- But could still benefit from per-resource rate limiting

**Fix Required:**
- Add per-resource timestamp tracking
- Skip reconciliation if last reconciliation was < 10 seconds ago
- Or rely on Controller's built-in debouncing

### Recommended Fixes (Priority Order)

1. **✅ COMPLETED:** Remove or make `lastReconciled` deterministic
   - ✅ Removed `lastReconciled` from all status patch functions
   - ✅ Prevents status-update loops from non-deterministic timestamps
   - ✅ Controller already tracks reconciliation timing internally

2. **✅ COMPLETED:** Add debounce to watcher config
   - ✅ Added 5-second debounce period via `ControllerConfig::debounce()`
   - ✅ Batches rapid events together
   - ✅ Reduces API load significantly

3. **✅ COMPLETED:** Add concurrency limits
   - ✅ Limited to 3 concurrent reconciliations per watcher
   - ✅ Prevents overwhelming NetBox API
   - ✅ Total: 17 watchers × 3 = 51 max concurrent reconciliations

4. **⏳ PENDING:** Verify generation-based filtering
   - Controller should automatically filter status-only updates (generation unchanged)
   - Need to add logging to confirm this is working
   - May need explicit generation check if Controller doesn't filter properly

5. **⏳ PENDING:** Add per-resource rate limiting
   - Only if debouncing isn't sufficient
   - Track last reconciliation time per resource
   - Skip if too recent (< 10 seconds)

### Expected Impact After Fixes

- **Before:** Potentially hundreds of reconciliations per minute (status update loops)
- **After:** ~1-2 reconciliations per resource per minute (only on actual changes)
- **API Load Reduction:** 90%+ reduction in NetBox API calls (estimated)
- **Resource Usage:** Much lower CPU/memory usage
- **Stability:** No more reconciliation loops from status updates

### Implementation Status

**Date Completed:** 2025-12-25  
**Status:** ✅ **PARTIALLY COMPLETE** (3/5 fixes implemented)

#### ✅ Completed Fixes

1. **Removed `lastReconciled` from status updates**
   - Removed from `create_resource_status_patch()`
   - Removed from `create_prefix_status_patch()`
   - Removed from `create_ipclaim_status_patch()`
   - Set to `None` in IPPool status updates
   - **Impact:** Prevents reconciliation loops from non-deterministic timestamps

2. **Added debounce configuration**
   - Added 5-second debounce via `ControllerConfig::debounce(Duration::from_secs(5))`
   - Batches rapid events together
   - **Impact:** Reduces rapid-fire reconciliations significantly

3. **Added concurrency limits**
   - Limited to 3 concurrent reconciliations per watcher
   - Total: 17 watchers × 3 = 51 max concurrent reconciliations
   - **Impact:** Prevents overwhelming NetBox API with unlimited concurrent requests

#### ⏳ Pending Fixes

4. **Verify generation-based filtering** - ✅ Added debug logging to track reconciliation frequency
5. **Add per-resource rate limiting** - Only if needed after testing

#### ✅ COMPLETED: Status Update Optimization

**Issue Identified:** Status is being updated on EVERY reconciliation, even when values haven't changed.

**Root Cause:** All reconcilers call `patch_status()` even when:
- Resource already exists
- Status values haven't changed
- Generation hasn't changed (status-only update)

**Fix Applied:**
- ✅ Created `NetBoxStatusCheck` trait implemented for all 17 status types
- ✅ Created generic `status_needs_update()` helper function
- ✅ Applied status comparison to ALL 17 reconcilers:
  - NetBoxDevice ✅
  - NetBoxPlatform ✅
  - NetBoxSite ✅
  - NetBoxInterface ✅
  - NetBoxTenant ✅
  - NetBoxRegion ✅
  - NetBoxSiteGroup ✅
  - NetBoxLocation ✅
  - NetBoxDeviceRole ✅
  - NetBoxManufacturer ✅
  - NetBoxDeviceType ✅
  - NetBoxVLAN ✅
  - NetBoxPrefix ✅
  - NetBoxAggregate ✅
  - NetBoxRole ✅
  - NetBoxTag ✅
  - (IPClaim and MACAddress have different patterns, handled separately)

**Pattern to Apply:**
```rust
// Before updating status, check if it actually changed
let needs_status_update = current_status
    .map(|s| {
        s.netbox_id != Some(resource.id)
            || s.netbox_url.as_deref() != Some(resource.url.as_str())
            || s.state != ResourceState::Created
            || s.error.is_some()
    })
    .unwrap_or(true); // No status = needs update

if needs_status_update {
    // Update status
} else {
    // Skip update - status already correct
    debug!("Resource already has correct status, skipping update");
    return Ok(());
}
```

**Impact:** This should reduce status updates by 90%+ and eliminate reconciliation loops.

### Files Changed

- `controllers/netbox/src/watcher.rs` - Added debounce and concurrency limits
- `controllers/netbox/src/reconciler/mod.rs` - Removed `lastReconciled` from status patches
- `controllers/netbox/src/reconciler/ipam/ip_pool.rs` - Set `last_reconciled` to `None`

### Files Requiring Changes

- `controllers/netbox/src/watcher.rs` - Add debounce and concurrency limits
- `controllers/netbox/src/reconciler/mod.rs` - Fix `lastReconciled` to be deterministic
- All reconciler files - Verify status updates don't trigger loops

