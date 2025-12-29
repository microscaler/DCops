# WET (Write Everything Twice) Patterns Analysis

## Patterns Found Across Reconcilers

### 1. **Required Dependency Resolution** (HIGH PRIORITY)
**Pattern**: Resolve a required dependency's netbox_id from CRD status
**Occurrences**: ~15+ times across site, device, prefix, vlan, location, etc.

**Current Pattern**:
```rust
let tenant_id = match self.netbox_tenant_api.get(&resource.spec.tenant.name).await {
    Ok(tenant_crd) => {
        tenant_crd.status
            .as_ref()
            .and_then(|s| s.netbox_id)
            .ok_or_else(|| ControllerError::InvalidConfig(
                format!("Tenant '{}' has not been created in NetBox yet (no netbox_id in status)", resource.spec.tenant.name)
            ))?
    }
    Err(_) => {
        return Err(ControllerError::InvalidConfig(
            format!("Tenant CRD '{}' not found for {}", resource.spec.tenant.name, name)
        ));
    }
};
```

**Proposed Helper**:
```rust
async fn resolve_required_dependency_id<API, CRD>(
    api: &API,
    resource_name: &str,
    dependency_name: &str,
    dependency_kind: &str,
    current_resource_name: &str,
) -> Result<u64, ControllerError>
where
    API: KubeApiTrait<CRD>,
    CRD: HasStatus<Status: NetBoxStatusCheck>,
```

**Locations to Refactor**:
- [x] `dcim/site.rs:170` - Tenant ID resolution ✅
- [x] `dcim/device.rs:120` - DeviceType ID resolution ✅
- [x] `dcim/device.rs:141` - DeviceRole ID resolution ✅
- [x] `dcim/device.rs:162` - Site ID resolution ✅
- [x] `dcim/device.rs:184` - Tenant ID resolution ✅
- [x] `dcim/vlan.rs:137` - Tenant ID resolution ✅
- [x] `dcim/device_type.rs:30` - Manufacturer ID resolution ✅
- [x] `dcim/location.rs:118` - Site ID resolution ✅
- [x] `dcim/location.rs:162` - Tenant ID resolution ✅
- [x] `ipam/prefix.rs:179` - Tenant ID resolution (first occurrence) ✅
- [x] `ipam/prefix.rs:417` - Tenant ID resolution (second occurrence in update path) ✅

---

### 2. **Optional Dependency Resolution** (HIGH PRIORITY)
**Pattern**: Resolve an optional dependency's netbox_id from CRD status
**Occurrences**: ~20+ times across site, prefix, device, etc.

**Current Pattern**:
```rust
let region_id = if let Some(region_ref) = &site_crd.spec.region {
    if region_ref.kind != "NetBoxRegion" {
        warn!("Invalid kind '{}' for region reference in site {}, expected 'NetBoxRegion'", region_ref.kind, name);
        None
    } else {
        match self.netbox_region_api.get(&region_ref.name).await {
            Ok(region_crd) => {
                region_crd.status
                    .as_ref()
                    .and_then(|s| s.netbox_id)
            }
            Err(_) => {
                warn!("Region CRD '{}' not found for site {}, skipping region reference", region_ref.name, name);
                None
            }
        }
    }
} else {
    None
};
```

**Proposed Helper**:
```rust
async fn resolve_optional_dependency_id<API, CRD>(
    api: &API,
    reference: Option<&NetBoxResourceReference>,
    expected_kind: &str,
    dependency_name: &str,
    current_resource_name: &str,
) -> Option<u64>
where
    API: KubeApiTrait<CRD>,
    CRD: HasStatus<Status: NetBoxStatusCheck>,
```

**Locations to Refactor**:
- [x] `dcim/site.rs:187` - Region ID resolution (optional) ✅
- [x] `dcim/site.rs:209` - SiteGroup ID resolution (optional) ✅
- [x] `dcim/device.rs:201` - Platform ID resolution (optional) ✅
- [x] `dcim/device.rs:218` - Location ID resolution (optional) ✅
- [x] `dcim/vlan.rs:118` - Site ID resolution (optional) ✅
- [x] `dcim/vlan.rs:155` - Role ID resolution (optional) ✅
- [x] `dcim/platform.rs:30` - Manufacturer ID resolution (optional) ✅
- [x] `dcim/site_group.rs:26` - Parent SiteGroup ID resolution (optional) ✅
- [x] `dcim/location.rs:136` - Parent Location ID resolution (optional) ✅
- [x] `dcim/region.rs:26` - Parent Region ID resolution (optional) ✅
- [ ] `tenancy.rs:168` - TenantGroup ID resolution (optional)
- [x] `ipam/prefix.rs:134` - Site ID resolution (optional, first occurrence) ✅
- [x] `ipam/prefix.rs:156` - VLAN ID resolution (optional, first occurrence) ✅
- [x] `ipam/prefix.rs:201` - Role ID resolution (optional, first occurrence) ✅
- [x] `ipam/prefix.rs:395` - Site ID resolution (optional, second occurrence in update path) ✅
- [x] `ipam/prefix.rs:366` - VLAN ID resolution (optional, second occurrence in update path) ✅
- [x] `ipam/prefix.rs:436` - Role ID resolution (optional, second occurrence in update path) ✅

---

### 3. **Name/Namespace Extraction** (MEDIUM PRIORITY)
**Pattern**: Extract name and namespace from CRD metadata
**Occurrences**: 60+ times across ALL reconcilers

**Current Pattern**:
```rust
let name = resource.metadata.name.as_ref()
    .ok_or_else(|| ControllerError::InvalidConfig("NetBoxSite missing name".to_string()))?;
let namespace = resource.metadata.namespace.as_deref()
    .unwrap_or("default");
```

**Proposed Helper**:
```rust
fn extract_name_and_namespace<CRD>(
    crd: &CRD,
    resource_kind: &str,
) -> Result<(&str, &str), ControllerError>
where
    CRD: kube::Resource,
```

**Locations to Refactor** (60+ occurrences across all reconcilers):
- [x] `dcim/mac_address.rs:11-14` - Name/namespace extraction ✅
- [x] `dcim/interface.rs:11-14` - Name/namespace extraction ✅
- [x] `dcim/site.rs:150-153` - Name/namespace extraction ✅
- [x] `dcim/device.rs:20-21` - Name/namespace extraction ✅
- [x] `dcim/vlan.rs` - Name/namespace extraction ✅
- [x] `dcim/device_type.rs` - Name/namespace extraction ✅
- [x] `dcim/manufacturer.rs` - Name/namespace extraction ✅
- [x] `dcim/platform.rs` - Name/namespace extraction ✅
- [x] `dcim/site_group.rs` - Name/namespace extraction ✅
- [x] `dcim/location.rs` - Name/namespace extraction ✅
- [x] `dcim/device_role.rs` - Name/namespace extraction ✅
- [x] `dcim/region.rs` - Name/namespace extraction ✅
- [x] `tenancy.rs:45-48` - Name/namespace extraction ✅
- [x] `ipam/prefix.rs:71-74` - Name/namespace extraction ✅
- [x] `ipam/aggregate.rs:11-14` - Name/namespace extraction ✅
- [x] `ipam/ip_claim.rs` - Name/namespace extraction ✅
- [x] `ipam/ip_pool.rs` - Name/namespace extraction ✅
- [x] `extras.rs` - Name/namespace extraction (NetBoxRole) ✅
- [x] `extras.rs` - Name/namespace extraction (NetBoxTag) ✅

---

### 4. **Kind Validation** (MEDIUM PRIORITY)
**Pattern**: Validate resource reference kind matches expected
**Occurrences**: ~15+ times across site, prefix, device, etc.

**Current Pattern**:
```rust
if resource.spec.tenant.kind != "NetBoxTenant" {
    return Err(ControllerError::InvalidConfig(
        format!("Invalid kind '{}' for tenant reference in {}, expected 'NetBoxTenant'", resource.spec.tenant.kind, name)
    ));
}
```

**Proposed Helper**:
```rust
fn validate_reference_kind(
    reference: &NetBoxResourceReference,
    expected_kind: &str,
    reference_name: &str,
    current_resource_name: &str,
) -> Result<(), ControllerError>
```

**Locations to Refactor** (Required - return error):
- [x] `dcim/site.rs:158` - Tenant kind validation (required) ✅
- [x] `dcim/device.rs:116` - DeviceType kind validation (required) ✅
- [x] `dcim/device.rs:137` - DeviceRole kind validation (required) ✅
- [x] `dcim/device.rs:157` - Site kind validation (required) ✅
- [x] `dcim/device.rs:179` - Tenant kind validation (required) ✅
- [x] `dcim/vlan.rs:133` - Tenant kind validation (required) ✅
- [x] `dcim/device_type.rs:26` - Manufacturer kind validation (required) ✅
- [x] `dcim/location.rs:114` - Site kind validation (required) ✅
- [x] `dcim/location.rs:158` - Tenant kind validation (required) ✅
- [x] `ipam/prefix.rs:175` - Tenant kind validation (required, first occurrence) ✅
- [x] `ipam/prefix.rs:413` - Tenant kind validation (required, second occurrence in update path) ✅

**Locations to Refactor** (Optional - warn and return None):
- [x] `dcim/site.rs:188` - Region kind validation (optional) ✅
- [x] `dcim/site.rs:210` - SiteGroup kind validation (optional) ✅
- [x] `dcim/device.rs:201` - Platform kind validation (optional) ✅
- [x] `dcim/device.rs:218` - Location kind validation (optional) ✅
- [x] `dcim/device.rs:238` - IPClaim kind validation (optional, primary_ip4) ✅
- [x] `dcim/device.rs:306` - IPClaim kind validation (optional, primary_ip6) ✅
- [x] `dcim/vlan.rs:114` - Site kind validation (optional) ✅
- [x] `dcim/vlan.rs:155` - Role kind validation (optional) ✅
- [x] `dcim/platform.rs:26` - Manufacturer kind validation (optional) ✅
- [x] `dcim/site_group.rs:26` - Parent SiteGroup kind validation (optional) ✅
- [x] `dcim/location.rs:136` - Parent Location kind validation (optional) ✅
- [x] `dcim/region.rs:26` - Parent Region kind validation (optional) ✅
- [x] `tenancy.rs:168` - TenantGroup kind validation (optional) ✅
- [x] `ipam/prefix.rs:130` - Site kind validation (optional) ✅
- [x] `ipam/prefix.rs:152` - VLAN kind validation (optional) ✅
- [x] `ipam/prefix.rs:197` - Role kind validation (optional) ✅
- [x] `ipam/prefix.rs:366` - VLAN kind validation (optional, second occurrence) ✅
- [x] `ipam/prefix.rs:390` - Site kind validation (optional, second occurrence) ✅
- [x] `ipam/prefix.rs:436` - Role kind validation (optional, second occurrence) ✅
- [x] `ipam/ip_claim.rs:355` - NetBoxPrefix kind validation ✅
- [x] `ipam/ip_pool.rs:24` - NetBoxPrefix kind validation ✅

---

### 5. **Status Update with Error Handling** (LOW PRIORITY)
**Pattern**: Update status, handle errors consistently
**Occurrences**: ~10+ times across multiple reconcilers

**Current Pattern**:
```rust
let status_patch = Self::create_resource_status_patch(...);
let pp = kube::api::PatchParams::default();
match api.patch_status(name, &pp, &kube::api::Patch::Merge(status_patch)).await {
    Ok(_) => {
        debug!("Updated {} {}/{} status: NetBox ID {}", resource_name, namespace, name, netbox_id);
        Ok(())
    }
    Err(e) => {
        error!("Failed to update {} status: {}", resource_name, e);
        Err(ControllerError::Kube(e.into()))
    }
}
```

**Proposed Helper**:
```rust
async fn update_resource_status<API, CRD>(
    api: &API,
    name: &str,
    namespace: &str,
    netbox_id: u64,
    netbox_url: String,
    state: ResourceState,
    error: Option<String>,
    resource_name: &str,
) -> Result<(), ControllerError>
where
    API: KubeApiTrait<CRD>,
```

**Locations to Refactor**:
- [x] `dcim/mac_address.rs` - Status update error handling ✅
- [x] `dcim/interface.rs` - Status update error handling ✅
- [x] `dcim/site.rs:368` - Status update error handling ✅
- [x] `dcim/device.rs` - Status update error handling (2 locations) ✅
- [x] `dcim/vlan.rs` - Status update error handling (2 locations) ✅
- [x] `dcim/device_type.rs` - Status update error handling (2 locations) ✅
- [x] `dcim/manufacturer.rs` - Status update error handling (2 locations) ✅
- [x] `dcim/platform.rs` - Status update error handling (2 locations) ✅
- [x] `dcim/site_group.rs` - Status update error handling (2 locations) ✅
- [x] `dcim/location.rs` - Status update error handling (2 locations) ✅
- [x] `dcim/device_role.rs` - Status update error handling (2 locations) ✅
- [x] `dcim/region.rs` - Status update error handling (2 locations) ✅
- [x] `extras.rs` - Status update error handling (NetBoxRole - 2 locations) ✅
- [x] `extras.rs` - Status update error handling (NetBoxTag - 2 locations) ✅
- [x] `ipam/prefix.rs:490` - Status update error handling ✅
- [x] `ipam/aggregate.rs` - Status update error handling ✅
- [x] `tenancy.rs` - Status update error handling ✅

**Note**: Error status updates (e.g., `dcim/site.rs:143`, `ipam/prefix.rs:113`) use a different pattern where errors are logged but not returned. These are intentionally left as-is since they handle error state updates differently.

---

## Summary

### High Priority (Most Duplicated)
1. **Required Dependency Resolution** - ~15+ occurrences
2. **Optional Dependency Resolution** - ~20+ occurrences

### Medium Priority (Frequently Used)
3. **Name/Namespace Extraction** - 60+ occurrences
4. **Kind Validation** - ~15+ occurrences

### Low Priority (Less Critical)
5. **Status Update Error Handling** - ~10+ occurrences

---

## Recommended Implementation Order

1. **Required Dependency Resolution Helper** - Highest impact, most duplicated
2. **Optional Dependency Resolution Helper** - Second highest impact
3. **Name/Namespace Extraction Helper** - Very common, easy to implement
4. **Kind Validation Helper** - Simple, reduces boilerplate
5. **Status Update Helper** - Nice to have, but less critical

