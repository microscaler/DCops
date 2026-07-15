# Tenant Reference CR Update Guide

**Date:** 2025-12-25  
**Status:** ✅ **GUIDE COMPLETE**

## Overview

This guide helps determine which CRs need tenant references and how to add them.

## Tenant Support by CRD

| CRD | Tenant Field | Required? | Example CR Status |
|-----|--------------|-----------|-------------------|
| **NetBoxPrefix** | ✅ `tenant: Option<NetBoxResourceReference>` | ❌ Optional | ✅ Has tenant in example |
| **NetBoxDevice** | ✅ `tenant: Option<NetBoxResourceReference>` | ❌ Optional | ✅ Has tenant in example |
| **NetBoxSite** | ✅ `tenant: Option<NetBoxResourceReference>` | ❌ Optional | ✅ Has tenant in example |
| **NetBoxVLAN** | ✅ `tenant: Option<NetBoxTenant>` | ❌ Optional | ✅ Has tenant in example |

## Current Example CR Status

### ✅ Already Have Tenant References

All example CRs that support tenant **already have tenant references**:

1. **`config/examples/netbox-prefix-example.yaml`**
   ```yaml
   tenant:
     apiGroup: "dcops.microscaler.io"
     kind: "NetBoxTenant"
     name: "datacenter-tenant"
   ```

2. **`config/examples/netbox-device-example.yaml`**
   ```yaml
   tenant:
     apiGroup: "dcops.microscaler.io"
     kind: "NetBoxTenant"
     name: "datacenter-tenant"
   ```

3. **`config/examples/netbox-site-example.yaml`**
   ```yaml
   tenant:
     apiGroup: "dcops.microscaler.io"
     kind: "NetBoxTenant"
     name: "datacenter-tenant"
   ```

4. **`config/examples/netbox-vlan-example.yaml`**
   ```yaml
   tenant:
     apiGroup: "dcops.microscaler.io"
     kind: "NetBoxTenant"
     name: "datacenter-tenant"
   ```

## Do Your Existing CRs Need Updates?

### ✅ **No Updates Needed If:**

1. **Your CRs already have tenant references** - They will work correctly now that the API methods are fixed
2. **You don't want tenant assignment** - Tenant is optional, so CRs without tenant will work fine

### 🔄 **Updates Recommended If:**

1. **Your CRs don't have tenant but you want tenant assignment**
   - Add tenant reference to existing CRs
   - Tenant will be set in NetBox after reconciliation

2. **You want consistent tenant assignment across resources**
   - Ensure all Prefix, Device, Site, and VLAN CRs reference the same tenant
   - This provides better organization in NetBox

## How to Add Tenant Reference to Existing CRs

### Step 1: Ensure NetBoxTenant CR Exists

First, make sure you have a NetBoxTenant CR:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxTenant
metadata:
  name: datacenter-tenant
  namespace: default
spec:
  name: "Datacenter Tenant"
  slug: "datacenter-tenant"
  description: "Primary datacenter tenant"
```

### Step 2: Add Tenant Reference to Your CR

For **NetBoxPrefix**, **NetBoxDevice**, **NetBoxSite**, or **NetBoxVLAN**:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxPrefix  # or NetBoxDevice, NetBoxSite, NetBoxVLAN
metadata:
  name: your-resource
  namespace: default
spec:
  # ... existing spec fields ...
  tenant:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxTenant"
    name: "datacenter-tenant"  # Must match NetBoxTenant CR name
    # namespace is optional, defaults to same namespace
```

### Step 3: Apply the Updated CR

```bash
kubectl apply -f your-resource.yaml
```

The controller will:
1. Resolve the tenant reference to NetBox tenant ID
2. Create/update the resource in NetBox with tenant assigned
3. Update the CR status with NetBox ID

## Verification

After updating CRs with tenant references:

1. **Check CR status:**
   ```bash
   kubectl get netboxprefix -o yaml
   kubectl get netboxdevice -o yaml
   kubectl get netboxsite -o yaml
   kubectl get netboxvlan -o yaml
   ```

2. **Check NetBox UI:**
   - Navigate to the resource in NetBox
   - Verify "Tenant" field is set correctly

3. **Check controller logs:**
   ```bash
   kubectl logs -n dcops-system deployment/netbox-controller | grep tenant
   ```

## Common Scenarios

### Scenario 1: New CRs Created After Fix

**Status:** ✅ **No action needed**
- New CRs with tenant references will work correctly
- Controller will set tenant in NetBox automatically

### Scenario 2: Existing CRs Without Tenant

**Status:** ⚠️ **Optional update**
- CRs will continue to work without tenant
- If you want tenant assignment, add tenant reference and reapply

### Scenario 3: Existing CRs With Tenant (But Not Working)

**Status:** ✅ **Should work now**
- After the fixes, existing CRs with tenant references should work
- Controller will reconcile and set tenant in NetBox
- If tenant still not set, check:
  - NetBoxTenant CR exists and is reconciled
  - Tenant reference name matches NetBoxTenant CR name
  - Controller logs for errors

## Troubleshooting

### Tenant Not Set in NetBox

1. **Check NetBoxTenant CR exists:**
   ```bash
   kubectl get netboxtenant datacenter-tenant -o yaml
   ```

2. **Check NetBoxTenant has netbox_id in status:**
   ```bash
   kubectl get netboxtenant datacenter-tenant -o jsonpath='{.status.netboxId}'
   ```
   Should return a number (NetBox tenant ID)

3. **Check controller logs:**
   ```bash
   kubectl logs -n dcops-system deployment/netbox-controller | grep -i tenant
   ```

4. **Verify tenant reference name matches:**
   - CR tenant reference `name` must match NetBoxTenant CR `metadata.name`
   - Case-sensitive!

### Tenant Reference Not Resolved

**Error:** `Tenant CRD 'xxx' not found`

**Solution:**
- Ensure NetBoxTenant CR exists with matching name
- Check namespace (tenant reference defaults to same namespace)
- Verify NetBoxTenant CR is reconciled (has `netbox_id` in status)

## Summary

- ✅ **Example CRs already have tenant references** - No changes needed
- ✅ **Tenant is optional** - CRs work without tenant
- 🔄 **Add tenant if desired** - Follow the guide above
- ✅ **Fixes are complete** - Tenant will be set correctly when CRs are reconciled

