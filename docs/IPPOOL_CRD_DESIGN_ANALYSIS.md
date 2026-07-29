# IPPool CRD Design Analysis

**Date:** 2025-12-25  
**Status:** 🔍 **ANALYSIS COMPLETE** - Design issue identified

## Problem Statement

The IPPool CRD currently allows NetBox prefix IDs in the spec:
```yaml
spec:
  netboxPrefixRef:
    id: "1"  # Direct NetBox ID - PROBLEMATIC
```

**Issue:** NetBox IDs are volatile and can change when resources are deleted/recreated. This violates GitOps principles where spec should contain stable, desired state.

## Kubernetes Spec vs Status Principles

### Spec (Desired State)
- **What you want** - stable, declarative, GitOps-friendly
- Should NOT contain volatile IDs that change in external systems
- Should contain stable references (CRD names, stable identifiers)

### Status (Observed State)
- **What actually exists** - can change, controller-managed
- Should contain resolved IDs from external systems
- Can be updated by controller without requiring spec changes

## Current IPPool Design

### Spec (`IPPoolSpec`)
```rust
pub struct IPPoolSpec {
    pub netbox_prefix_ref: NetBoxPrefixRef,  // Contains id: String
    pub role: String,
    pub allocation_strategy: AllocationStrategy,
}

pub struct NetBoxPrefixRef {
    pub id: String,  // ❌ Can be direct NetBox ID (volatile!)
    pub site: Option<String>,
}
```

### Status (`IPPoolStatus`)
```rust
pub struct IPPoolStatus {
    pub total_ips: u32,
    pub allocated_ips: u32,
    pub available_ips: u32,
    pub last_reconciled: Option<DateTime<Utc>>,
    // ❌ Missing: resolved NetBox prefix ID
}
```

## Comparison with Other CRDs

### NetBoxPrefix CRD (Correct Pattern)
- **Spec:** Contains stable identifiers (prefix CIDR, references to other CRDs)
- **Status:** Contains `netbox_id: Option<u64>` (observed state)

### NetBoxSite CRD (Correct Pattern)
- **Spec:** Contains stable identifiers (name, slug, references to other CRDs)
- **Status:** Contains `netbox_id: Option<u64>` (observed state)

### IPPool CRD (Current - Problematic)
- **Spec:** Contains `id: String` which can be volatile NetBox ID ❌
- **Status:** Missing resolved NetBox prefix ID ❌

## Design Issues

### Issue 1: Volatile ID in Spec
**Problem:**
- If prefix ID 1 is deleted and recreated in NetBox, it might get a new ID (e.g., 2)
- The IPPool spec would need manual update: `id: "1"` → `id: "2"`
- This violates GitOps - spec should be stable

**Example Scenario:**
1. IPPool spec has `id: "1"`
2. Prefix 1 is deleted in NetBox (accidentally or intentionally)
3. Prefix is recreated → gets new ID 2
4. IPPool now fails because ID 1 doesn't exist
5. **Manual intervention required** to update spec to `id: "2"` ❌

### Issue 2: Missing Resolved ID in Status
**Problem:**
- Controller resolves the prefix ID but doesn't store it in status
- If prefix is recreated, controller has to re-resolve every time
- No way to track which NetBox prefix ID is actually being used

**Impact:**
- Can't detect drift (if prefix ID changes)
- Can't track which NetBox resource is actually being used
- Harder to debug issues

## Recommended Design

### Option 1: CRD Reference Only (Recommended)

**Spec:**
```rust
pub struct IPPoolSpec {
    pub netbox_prefix_ref: NetBoxPrefixReference,  // Only CRD reference
    pub role: String,
    pub allocation_strategy: AllocationStrategy,
}

pub struct NetBoxPrefixReference {
    /// Reference to NetBoxPrefix CRD (stable, GitOps-friendly)
    pub prefix: NetBoxResourceReference,  // apiGroup, kind, name
    pub site: Option<String>,  // Optional site hint
}
```

**Status:**
```rust
pub struct IPPoolStatus {
    /// Resolved NetBox prefix ID (observed state)
    pub netbox_prefix_id: Option<u64>,
    
    /// NetBox prefix URL
    pub netbox_prefix_url: Option<String>,
    
    /// Pool statistics
    pub total_ips: u32,
    pub allocated_ips: u32,
    pub available_ips: u32,
    
    /// Last reconciliation timestamp
    pub last_reconciled: Option<DateTime<Utc>>,
}
```

**Benefits:**
- ✅ Spec is stable (only CRD references)
- ✅ Status tracks observed NetBox ID
- ✅ Follows Kubernetes best practices
- ✅ GitOps-friendly (no volatile IDs in Git)
- ✅ Can detect drift (status.netbox_prefix_id vs actual NetBox)

**Example CR:**
```yaml
spec:
  netboxPrefixRef:
    prefix:
      apiGroup: "dcops.microscaler.io"
      kind: "NetBoxPrefix"
      name: "control-plane-prefix"  # Stable CRD name
    site: "datacenter-1"
  role: "control-plane"
status:
  netboxPrefixId: 1  # Resolved by controller (can change)
  netboxPrefixUrl: "http://netbox/api/ipam/prefixes/1/"
  totalIps: 50
  allocatedIps: 0
  availableIps: 50
```

### Option 2: Support Both (Backward Compatible)

Keep current design but:
- **Spec:** Support both CRD reference and direct ID (for backward compatibility)
- **Status:** Add resolved NetBox prefix ID
- **Migration path:** Gradually move users to CRD references

**Spec:**
```rust
pub enum NetBoxPrefixRef {
    /// CRD reference (recommended, stable)
    CrdReference(NetBoxResourceReference),
    
    /// Direct NetBox ID (legacy, volatile)
    DirectId(String),
}
```

**Status:**
```rust
pub struct IPPoolStatus {
    pub netbox_prefix_id: Option<u64>,  // Resolved ID (always set)
    // ... rest
}
```

**Benefits:**
- ✅ Backward compatible
- ✅ Status tracks resolved ID
- ✅ Can migrate users gradually

**Drawbacks:**
- ❌ Still allows volatile IDs in spec
- ❌ More complex implementation

## Recommendation

**Use Option 1** - CRD reference only:
1. **Follows Kubernetes best practices** - spec = desired, status = observed
2. **GitOps-friendly** - no volatile IDs in Git
3. **Resilient** - if prefix is recreated, CRD's netbox_id updates automatically
4. **Consistent** - matches pattern used by all other NetBox CRDs
5. **Simpler** - one way to reference, less complexity

## Implementation Status

✅ **COMPLETED** - Option 1 has been implemented:

### Changes Made

1. **CRD Spec (`crates/crds/src/ip_pool.rs`)**:
   - Changed `netbox_prefix_ref: NetBoxPrefixRef` to `netbox_prefix_ref: NetBoxResourceReference`
   - Removed `NetBoxPrefixRef` struct (no longer needed)
   - Spec now only accepts CRD references (stable, GitOps-friendly)

2. **CRD Status (`crates/crds/src/ip_pool.rs`)**:
   - Added `netbox_prefix_id: Option<u64>` - stores resolved NetBox prefix ID
   - Added `netbox_prefix_url: Option<String>` - stores NetBox prefix URL
   - These are observed state, managed by the controller

3. **Reconciler (`controllers/netbox/src/reconciler/ipam/ip_pool.rs`)**:
   - Updated to resolve prefix ID from `NetBoxResourceReference` (CRD reference)
   - Validates that reference is to `NetBoxPrefix` CRD
   - Stores resolved ID and URL in status
   - Only updates status when values change (prevents reconciliation loops)

4. **IPClaim Reconciler (`controllers/netbox/src/reconciler/ipam/ip_claim.rs`)**:
   - Updated to use IPPool's `status.netbox_prefix_id` (fast path)
   - Falls back to resolving from IPPool's CRD reference if status not available
   - Added helper method `resolve_prefix_id_from_pool_spec()` for fallback resolution

5. **Example CR (`config/examples/ippool-example.yaml`)**:
   - Updated to use Kubernetes-compliant `NetBoxResourceReference` format
   - Removed direct ID reference

### Benefits Achieved

- ✅ **No volatile IDs in spec** - only stable CRD references
- ✅ **Resolved IDs in status** - controller tracks observed state
- ✅ **GitOps-compliant** - spec changes only when desired state changes
- ✅ **Resilient** - automatic recovery if prefix is recreated
- ✅ **Consistent** - matches pattern used by all other NetBox CRDs

## Migration Path

If we change the design:
1. **Update CRD schema** - change `NetBoxPrefixRef` to use `NetBoxResourceReference`
2. **Update reconciler** - always resolve from CRD, store ID in status
3. **Update examples** - use CRD references
4. **Documentation** - explain why direct IDs are not supported

## Impact Analysis

### Current Users
- If any IPPool CRs use direct IDs, they'll need to be updated
- Migration: Change `id: "1"` to CRD reference

### Controller Behavior
- Always resolves prefix from CRD reference
- Stores resolved ID in status
- Can detect drift if prefix ID changes

### Benefits
- ✅ No more "prefix ID not found" errors due to ID changes
- ✅ Automatic recovery if prefix is recreated
- ✅ Better observability (can see resolved ID in status)
- ✅ Follows Kubernetes and GitOps best practices

## Conclusion

The current design violates Kubernetes spec/status separation principles. The NetBox prefix ID is **observed state** (can change), not **desired state** (stable). It should be:

- **Spec:** CRD reference only (stable, GitOps-friendly)
- **Status:** Resolved NetBox prefix ID (observed, controller-managed)

This matches the pattern used by all other NetBox CRDs and follows Kubernetes best practices.

