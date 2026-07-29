# Volatile ID Audit - Spec vs Status

**Date:** 2025-12-25  
**Status:** ✅ **ALL ISSUES FIXED - GITOPS COMPLIANT**

## Problem Statement

The IPPool CRD had a design flaw where volatile NetBox IDs were stored in the spec instead of status. This violates Kubernetes spec/status separation principles:
- **Spec** = Desired state (stable, GitOps-friendly)
- **Status** = Observed state (can change, controller-managed)

This audit identifies all CRDs that might have similar issues.

## Audit Results

### ✅ **IPPool** - FIXED
**Issue:** `netbox_prefix_ref.id: String` could contain volatile NetBox prefix ID  
**Status:** ✅ **FIXED** - Now uses `NetBoxResourceReference` (CRD reference) in spec, resolved ID in status

### ✅ **IPClaim** - FIXED
**Field:** `netbox_device_ref: Option<String>` in spec  
**Location:** `crates/crds/src/ip_claim.rs:53` (removed)  
**Analysis:**
- This field was **not used** by the IPClaim reconciler
- It was described as "NetBox device reference (optional)" but was unimplemented
- The reconciler didn't reference this field anywhere

**Status:** ✅ **FIXED** - Field removed from `DeviceRef` struct

### ✅ **NetBoxDevice** - IMPROVED
**Field:** `primary_ip4: Option<PrimaryIPReference>` in spec  
**Location:** `crates/crds/src/dcim/netbox_device.rs:63`  
**Previous:** `primary_ip4: Option<String>`  
**Analysis:**
- Previously accepted IPClaim CRD name as string or IP address
- Now uses `PrimaryIPReference` enum that supports:
  - `IPClaimRef(NetBoxResourceReference)` - GitOps-friendly CRD reference ✅
  - `IPAddress(String)` - Direct IP address as fallback ✅
- The reconciler properly resolves IPClaim CRD references to NetBox IP IDs
- IP addresses are **stable identifiers** (they don't change when resources are recreated)

**Status:** ✅ **IMPROVED** - Now uses proper CRD references for GitOps compliance
**Benefits:**
- Type-safe references using `NetBoxResourceReference`
- Consistent with other CRD references in the codebase
- Still supports IP addresses as fallback for backward compatibility

### ✅ **All NetBox CRDs** - CORRECT
All NetBox CRDs (NetBoxPrefix, NetBoxSite, NetBoxDevice, etc.) correctly:
- Store `netbox_id: Option<u64>` in **status** (not spec) ✅
- Use `NetBoxResourceReference` for cross-CRD references in spec ✅
- Follow Kubernetes best practices ✅

### ✅ **BootIntent** - CORRECT
- Uses CRD references (`BootProfileRef`) in spec ✅
- No volatile IDs ✅

### ✅ **IPClaim.pool_ref** - CORRECT
- Uses CRD reference (`IPPoolRef` with `name` and `namespace`) ✅
- No volatile IDs ✅

## Summary

| CRD | Field | Location | Issue | Status |
|-----|-------|----------|-------|--------|
| IPPool | `netbox_prefix_ref.id` | spec | Volatile NetBox ID | ✅ **FIXED** |
| IPClaim | `netbox_device_ref` | spec | Unused field | ✅ **FIXED** - Removed |
| NetBoxDevice | `primary_ip4` | spec | Accepts IP addresses (stable) | ✅ **IMPROVED** - Now supports IPClaim CRD references |
| NetBoxDevice | `primary_ip6` | spec | Accepts IP addresses (stable) | ✅ **IMPROVED** - Now supports IPClaim CRD references |

## Implementation Status

### ✅ All Issues Fixed

1. **IPClaim.netbox_device_ref**: ✅ **REMOVED** - Unused field removed from `DeviceRef` struct

2. **NetBoxDevice.primary_ip4/primary_ip6**: ✅ **IMPROVED** - Now uses `PrimaryIPReference` enum:
   - Supports `IPClaimRef(NetBoxResourceReference)` for GitOps-friendly CRD references
   - Supports `IPAddress(String)` as fallback for direct IP addresses
   - Reconciler properly resolves IPClaim CRD references to NetBox IP IDs

3. **Documentation**: ✅ **UPDATED** - All changes documented

## GitOps Compliance

All CRDs are now GitOps-compliant:
- ✅ No volatile IDs in spec
- ✅ All external system IDs stored in status
- ✅ All cross-CRD references use `NetBoxResourceReference`
- ✅ Stable, declarative spec fields only
- ✅ Observed state in status only

## Design Principles

When designing CRDs that reference external systems:

1. **Spec should contain:**
   - Stable identifiers (CRD names, stable resource names)
   - Desired state only
   - GitOps-friendly references

2. **Status should contain:**
   - Volatile IDs from external systems
   - Observed state
   - Resolved references

3. **Use `NetBoxResourceReference` for:**
   - All cross-CRD references
   - Consistent Kubernetes-style references
   - Type safety and validation

