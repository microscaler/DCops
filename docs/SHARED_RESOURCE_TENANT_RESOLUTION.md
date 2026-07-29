# Shared Resource Tenant Resolution Strategy

## Problem Statement

Some NetBox resources don't have tenant fields because they are **shared reference data** used by multiple tenants:
- **Manufacturers**: Used by DeviceTypes, Platforms, Devices
- **Device Types**: Used by Devices
- **Platforms**: Used by Devices
- **Device Roles**: Used by Devices
- **Regions**: Used by Sites
- **Site Groups**: Used by Sites
- **Tags**: Used by many resources
- **Roles** (Extras): Used by many resources

These resources need NetBox API tokens for CRUD operations, but they don't have a direct tenant association.

## Solution: Multi-Strategy Tenant Resolution

### Strategy 1: Inherit from Referencing Resource (Primary)

When reconciling a shared resource, find a resource that references it and use that resource's tenant.

**Example: Manufacturer**
1. Query Devices that use this Manufacturer (via DeviceType)
2. Use the first Device's tenant
3. If no Devices found, try DeviceTypes that use this Manufacturer
4. Use the DeviceType's tenant (if DeviceTypes had tenant - they don't, so skip)
5. Fall back to Strategy 2

**Example: Device Type**
1. Query Devices that use this DeviceType
2. Use the first Device's tenant
3. Fall back to Strategy 2

**Example: Platform**
1. Query Devices that use this Platform
2. Use the first Device's tenant
3. Fall back to Strategy 2

**Example: Device Role**
1. Query Devices that use this DeviceRole
2. Use the first Device's tenant
3. Fall back to Strategy 2

**Example: Region**
1. Query Sites that use this Region
2. Use the first Site's tenant
3. Fall back to Strategy 2

**Example: Site Group**
1. Query Sites that use this SiteGroup
2. Use the first Site's tenant
3. Fall back to Strategy 2

### Strategy 2: System/Admin Tenant (Fallback)

If no referencing resource is found, use a **system tenant** with admin privileges.

**Configuration:**
- Environment variable: `NETBOX_SYSTEM_TENANT_NAME` (default: "system")
- System tenant must exist in NetBox with admin token
- System tenant's token stored in Secret: `netbox-system-tenant-token`

### Strategy 3: Contact-Based (Future Enhancement)

If a resource has a `contact` field and contacts have tenant associations:
1. Get the contact's tenant
2. Use that tenant's token

**Note:** This requires NetBox Contact model support, which is not yet implemented.

## Implementation

### TokenResolver Enhancement

```rust
impl TokenResolver {
    /// Resolve tenant for a shared resource by finding a referencing resource
    pub async fn resolve_tenant_for_shared_resource(
        &self,
        namespace: &str,
        resource_kind: &str,
        resource_name: &str,
    ) -> Result<NetBoxResourceReference, TokenResolutionError> {
        match resource_kind {
            "NetBoxManufacturer" => {
                // Find a Device that uses this Manufacturer
                // (via DeviceType -> Manufacturer relationship)
                self.find_tenant_from_referencing_devices(namespace, resource_name).await
            }
            "NetBoxDeviceType" => {
                self.find_tenant_from_referencing_devices(namespace, resource_name).await
            }
            "NetBoxPlatform" => {
                self.find_tenant_from_referencing_devices(namespace, resource_name).await
            }
            "NetBoxDeviceRole" => {
                self.find_tenant_from_referencing_devices(namespace, resource_name).await
            }
            "NetBoxRegion" => {
                self.find_tenant_from_referencing_sites(namespace, resource_name).await
            }
            "NetBoxSiteGroup" => {
                self.find_tenant_from_referencing_sites(namespace, resource_name).await
            }
            _ => {
                // Fall back to system tenant
                self.get_system_tenant_reference()
            }
        }
    }
    
    /// Get system tenant reference (fallback)
    fn get_system_tenant_reference(&self) -> Result<NetBoxResourceReference, TokenResolutionError> {
        let system_tenant_name = std::env::var("NETBOX_SYSTEM_TENANT_NAME")
            .unwrap_or_else(|_| "system".to_string());
        
        Ok(NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTenant".to_string(),
            name: system_tenant_name,
            namespace: None,
        })
    }
}
```

## Resource Classification

### Resources WITH Tenant Fields (Direct Resolution)
- ✅ NetBoxSite
- ✅ NetBoxDevice
- ✅ NetBoxLocation
- ✅ NetBoxPrefix
- ✅ NetBoxVLAN
- ✅ NetBoxTenant (special case - resolves own token)

### Resources WITHOUT Tenant Fields (Shared Resources)
- ⚠️ NetBoxManufacturer → Use Device tenant (via DeviceType)
- ⚠️ NetBoxDeviceType → Use Device tenant
- ⚠️ NetBoxPlatform → Use Device tenant
- ⚠️ NetBoxDeviceRole → Use Device tenant
- ⚠️ NetBoxRegion → Use Site tenant
- ⚠️ NetBoxSiteGroup → Use Site tenant
- ⚠️ NetBoxTag → Use any referencing resource's tenant
- ⚠️ NetBoxRole (Extras) → Use any referencing resource's tenant
- ⚠️ NetBoxAggregate → Use system tenant (no clear reference)
- ⚠️ NetBoxInterface → Use Device tenant (parent)
- ⚠️ NetBoxMACAddress → Use Device tenant (via Interface)

## Contact Field Consideration

**Question:** Do manufacturers have contacts, and do contacts have tenant associations?

**Answer:** 
- NetBox supports associating Contacts with Manufacturers
- Contacts can have tenant associations
- **However:** This is a future enhancement - we should first implement Strategy 1 (inherit from referencing resource) and Strategy 2 (system tenant)

## Next Steps

1. ✅ Document the strategy (this document)
2. ⏳ Implement `resolve_tenant_for_shared_resource` in TokenResolver
3. ✅ Update reconcilers for shared resources - All stubbed out, ready for implementation
4. ⏳ Add system tenant configuration
5. ⏳ Implement Strategy 1: Inherit from referencing resource
6. ⏳ Implement Strategy 2: System/Admin tenant fallback
7. ⏳ Test with real NetBox instances

## Current Status

All reconcilers have been updated:
- **Resources WITH tenant fields**: ✅ Use `TokenResolver.create_client_for_tenant()` directly
- **Resources WITHOUT tenant fields**: ✅ Stubbed out with clear error messages indicating need for shared resource resolution

The next phase is to implement the `resolve_tenant_for_shared_resource()` method in `TokenResolver` and update the stubbed reconcilers to use it.

