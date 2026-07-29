# CRD Reference Audit

**Date:** 2025-12-25  
**Purpose:** Audit all CRDs and their references to ensure compliance with Kubernetes standards

## Kubernetes Reference Standards

Kubernetes uses structured reference types:
- **`LocalObjectReference`**: `{ name: string }` - Same namespace references
- **`ObjectReference`**: `{ apiVersion, kind, name, namespace, uid }` - Cross-namespace/cluster
- **`TypedLocalObjectReference`**: `{ apiGroup, kind, name }` - Typed same-namespace references
- **`SecretKeySelector`**: `{ name, key }` - For secret references
- **`ConfigMapKeySelector`**: `{ name, key }` - For configmap references

## Current State: Custom `NetBoxResourceRef`

We currently use a custom `NetBoxResourceRef` type:
```rust
pub struct NetBoxResourceRef {
    pub name: String,
    pub namespace: Option<String>,
}
```

**Issues:**
1. ❌ Missing `kind` - Kubernetes can't validate the referenced resource type
2. ❌ Missing `apiGroup` - Can't distinguish between different API groups
3. ❌ Not using Kubernetes core types
4. ❌ No validation that the referenced resource exists

## Recommended: Use `TypedLocalObjectReference` Pattern

For CRD-to-CRD references, we should use a pattern similar to `TypedLocalObjectReference`:

```rust
pub struct NetBoxResourceReference {
    /// API group of the referenced resource (e.g., "dcops.microscaler.io")
    pub api_group: String,
    
    /// Kind of the referenced resource (e.g., "NetBoxSite")
    pub kind: String,
    
    /// Name of the referenced resource
    pub name: String,
    
    /// Namespace (optional, defaults to same namespace)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}
```

This allows:
- ✅ Kubernetes to validate the reference type
- ✅ Clear documentation of what resource type is expected
- ✅ Better error messages when references are invalid
- ✅ Alignment with Kubernetes patterns

## Complete Reference Audit

### 1. NetBoxPrefix (`netboxprefixes.dcops.microscaler.io`)

| Field | Current Type | References | Required | Notes |
|-------|-------------|------------|----------|-------|
| `site` | `Option<NetBoxResourceRef>` | `NetBoxSite` | No | Should include `kind: "NetBoxSite"` |
| `tenant` | `Option<NetBoxResourceRef>` | `NetBoxTenant` | No | Should include `kind: "NetBoxTenant"` |
| `vlan` | `Option<NetBoxResourceRef>` | `NetBoxVLAN` | No | Should include `kind: "NetBoxVLAN"` |
| `role` | `Option<NetBoxResourceRef>` | `NetBoxRole` | No | Should include `kind: "NetBoxRole"` |
| `aggregate` | `Option<NetBoxResourceRef>` | `NetBoxAggregate` | No | Should include `kind: "NetBoxAggregate"` |
| `tags` | `Option<Vec<NetBoxResourceRef>>` | `NetBoxTag[]` | No | Should include `kind: "NetBoxTag"` |

**Total References:** 6

### 2. NetBoxSite (`netboxsites.dcops.microscaler.io`)

| Field | Current Type | References | Required | Notes |
|-------|-------------|------------|----------|-------|
| `tenant` | `Option<NetBoxResourceRef>` | `NetBoxTenant` | No | Should include `kind: "NetBoxTenant"` |
| `region` | `Option<NetBoxResourceRef>` | `NetBoxRegion` | No | Should include `kind: "NetBoxRegion"` |
| `site_group` | `Option<NetBoxResourceRef>` | `NetBoxSiteGroup` | No | Should include `kind: "NetBoxSiteGroup"` |

**Total References:** 3

### 3. NetBoxVLAN (`netboxvlans.dcops.microscaler.io`)

| Field | Current Type | References | Required | Notes |
|-------|-------------|------------|----------|-------|
| `site` | `Option<NetBoxResourceRef>` | `NetBoxSite` | No | Should include `kind: "NetBoxSite"` |
| `tenant` | `Option<NetBoxResourceRef>` | `NetBoxTenant` | No | Should include `kind: "NetBoxTenant"` |
| `role` | `Option<NetBoxResourceRef>` | `NetBoxRole` | No | Should include `kind: "NetBoxRole"` |
| `group` | `Option<NetBoxResourceRef>` | `NetBoxVLANGroup` | No | **NOT IMPLEMENTED** - CRD doesn't exist yet |

**Total References:** 4 (3 implemented, 1 deferred)

### 4. NetBoxLocation (`netboxlocations.dcops.microscaler.io`)

| Field | Current Type | References | Required | Notes |
|-------|-------------|------------|----------|-------|
| `site` | `NetBoxResourceRef` | `NetBoxSite` | **Yes** | Should include `kind: "NetBoxSite"` |
| `parent` | `Option<NetBoxResourceRef>` | `NetBoxLocation` | No | Self-reference, should include `kind: "NetBoxLocation"` |

**Total References:** 2

### 5. NetBoxRegion (`netboxregions.dcops.microscaler.io`)

| Field | Current Type | References | Required | Notes |
|-------|-------------|------------|----------|-------|
| `parent` | `Option<NetBoxResourceRef>` | `NetBoxRegion` | No | Self-reference, should include `kind: "NetBoxRegion"` |

**Total References:** 1

### 6. NetBoxSiteGroup (`netboxsitegroups.dcops.microscaler.io`)

| Field | Current Type | References | Required | Notes |
|-------|-------------|------------|----------|-------|
| `parent` | `Option<NetBoxResourceRef>` | `NetBoxSiteGroup` | No | Self-reference, should include `kind: "NetBoxSiteGroup"` |

**Total References:** 1

### 7. NetBoxDevice (`netboxdevices.dcops.microscaler.io`)

| Field | Current Type | References | Required | Notes |
|-------|-------------|------------|----------|-------|
| `device_type` | `NetBoxResourceRef` | `NetBoxDeviceType` | **Yes** | Should include `kind: "NetBoxDeviceType"` |
| `device_role` | `NetBoxResourceRef` | `NetBoxDeviceRole` | **Yes** | Should include `kind: "NetBoxDeviceRole"` |
| `site` | `NetBoxResourceRef` | `NetBoxSite` | **Yes** | Should include `kind: "NetBoxSite"` |
| `location` | `Option<NetBoxResourceRef>` | `NetBoxLocation` | No | Should include `kind: "NetBoxLocation"` |
| `tenant` | `Option<NetBoxResourceRef>` | `NetBoxTenant` | No | Should include `kind: "NetBoxTenant"` |
| `platform` | `Option<NetBoxResourceRef>` | `NetBoxPlatform` | No | Should include `kind: "NetBoxPlatform"` |

**Total References:** 6

### 8. NetBoxDeviceType (`netboxdevicetypes.dcops.microscaler.io`)

| Field | Current Type | References | Required | Notes |
|-------|-------------|------------|----------|-------|
| `manufacturer` | `NetBoxResourceRef` | `NetBoxManufacturer` | **Yes** | Should include `kind: "NetBoxManufacturer"` |

**Total References:** 1

### 9. NetBoxPlatform (`netboxplatforms.dcops.microscaler.io`)

| Field | Current Type | References | Required | Notes |
|-------|-------------|------------|----------|-------|
| `manufacturer` | `Option<NetBoxResourceRef>` | `NetBoxManufacturer` | No | Should include `kind: "NetBoxManufacturer"` |

**Total References:** 1

### 10. NetBoxTenant (`netboxtenants.dcops.microscaler.io`)

| Field | Current Type | References | Required | Notes |
|-------|-------------|------------|----------|-------|
| `group` | `Option<String>` | `NetBoxTenantGroup` | No | **NOT UPDATED** - Still uses plain string! Should be `NetBoxResourceReference` |

**Total References:** 1 (needs update)

### 11. IPClaim (`ipclaims.dcops.microscaler.io`)

| Field | Current Type | References | Required | Notes |
|-------|-------------|------------|----------|-------|
| `pool_ref` | `IPPoolRef` | `IPPool` | **Yes** | Custom type, should use standard pattern |
| `device_ref` | `DeviceRef` | Various | **Yes** | Custom type, not a CRD reference |

**Total References:** 1 (custom types, may be acceptable)

### 12. IPPool (`ippools.dcops.microscaler.io`)

| Field | Current Type | References | Required | Notes |
|-------|-------------|------------|----------|-------|
| `netbox_prefix_ref` | `NetBoxPrefixRef` | `NetBoxPrefix` | **Yes** | Custom type, should use standard pattern |

**Total References:** 1 (custom type, may be acceptable)

### 13. BootIntent (`bootintents.dcops.microscaler.io`)

| Field | Current Type | References | Required | Notes |
|-------|-------------|------------|----------|-------|
| `profile_ref` | `BootProfileRef` | `BootProfile` | **Yes** | Custom type, should use standard pattern |

**Total References:** 1 (custom type, may be acceptable)

## Summary Statistics

| Category | Count |
|----------|-------|
| **Total CRDs with References** | 13 |
| **Total Reference Fields** | 30 |
| **Using `NetBoxResourceRef`** | 25 |
| **Using Custom Types** | 4 |
| **Using Plain Strings** | 1 (NetBoxTenant.group) |
| **Missing `kind` Information** | 25 |
| **Missing `apiGroup` Information** | 25 |

## Recommended Actions

### High Priority

1. **Replace `NetBoxResourceRef` with `NetBoxResourceReference`**
   - Add `api_group: String` field
   - Add `kind: String` field
   - Keep `name: String` and `namespace: Option<String>`
   - This enables Kubernetes validation

2. **Update NetBoxTenant.group**
   - Change from `Option<String>` to `Option<NetBoxResourceReference>`
   - Add `kind: "NetBoxTenantGroup"`

3. **Update all 25 reference fields**
   - Add appropriate `kind` values
   - Add `api_group: "dcops.microscaler.io"` (all our CRDs use same group)

### Medium Priority

4. **Consider standardizing custom reference types**
   - `IPPoolRef`, `NetBoxPrefixRef`, `BootProfileRef` could use the same pattern
   - Or document why they need custom types

5. **Add validation**
   - Controller should validate that referenced resources exist
   - Use Kubernetes admission webhooks if needed

### Low Priority

6. **Consider using Kubernetes core types**
   - Evaluate if `k8s_openapi::api::core::v1::TypedLocalObjectReference` can be used
   - May need custom implementation for our use case

## Implementation Plan

1. ✅ Create new `NetBoxResourceReference` type with `api_group`, `kind`, `name`, `namespace`
2. ✅ Update all CRDs to use new type (25 fields updated)
3. ✅ Update reconciliation logic to extract `kind` and validate
4. ✅ Update example CRs (8 example files updated)
5. ✅ Regenerate CRDs
6. ✅ Test compilation - **COMPLETE**

## Implementation Status: ✅ COMPLETE

**Date Completed:** 2025-12-25

All CRDs now use Kubernetes-compliant `NetBoxResourceReference` type with:
- `apiGroup`: "dcops.microscaler.io" (for all NetBox CRDs)
- `kind`: Resource type (e.g., "NetBoxSite", "NetBoxTenant")
- `name`: Resource name
- `namespace`: Optional (defaults to same namespace)

All reconciliation logic validates `kind` before resolving references, ensuring type safety and better error messages.

