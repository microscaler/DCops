# NetBox Controller Reconciler Completion Plan

This document analyzes the NetBox client API and identifies missing reconcilers to complete full NetBox support.

## Analysis Summary

### NetBox Client API Resources

The `netbox-client` crate provides client methods for the following NetBox API resources:

#### IPAM (IP Address Management)
- ✅ **Prefix** - `query_prefixes`, `get_prefix`, `create_prefix`, `update_prefix`
- ✅ **Aggregate** - `query_aggregates`, `get_aggregate`, `create_aggregate`
- ✅ **RIR** - `query_rirs`, `get_rir_by_name`, `create_rir`
- ✅ **VLAN** - `query_vlans`, `get_vlan`, `create_vlan`, `update_vlan`
- ❌ **IPAddress** - `query_ip_addresses`, `get_ip_address`, `create_ip_address`, `update_ip_address`, `delete_ip_address`, `allocate_ip`, `get_available_ips`

#### DCIM (Data Center Infrastructure Management)
- ✅ **Site** - `query_sites`, `get_site`, `create_site`, `update_site`
- ✅ **Region** - `query_regions`, `get_region`, `get_region_by_name`, `create_region`
- ✅ **SiteGroup** - `query_site_groups`, `get_site_group`, `get_site_group_by_name`, `create_site_group`
- ✅ **Location** - `query_locations`, `get_location`, `get_location_by_name`, `create_location`
- ✅ **DeviceRole** - `query_device_roles`, `get_device_role_by_name`, `create_device_role`
- ✅ **Manufacturer** - `query_manufacturers`, `get_manufacturer_by_name`, `create_manufacturer`
- ✅ **Platform** - `query_platforms`, `get_platform_by_name`, `create_platform`
- ✅ **DeviceType** - `query_device_types`, `get_device_type_by_model`, `create_device_type`
- ✅ **Device** - `query_devices`, `get_device`, `get_device_by_mac`, `create_device`, `update_device`
- ✅ **Interface** - `query_interfaces`, `get_interface`, `create_interface`, `update_interface`
- ✅ **MACAddress** - `query_mac_addresses`, `get_mac_address_by_address`, `create_mac_address`

#### Tenancy
- ✅ **Tenant** - `query_tenants`, `get_tenant`, `create_tenant`, `update_tenant`
- ❌ **TenantGroup** - `query_tenant_groups`, `get_tenant_group_by_name`, `create_tenant_group`

#### Extras
- ✅ **Role** (IPAM Role) - `query_roles`, `get_role`, `create_role`
- ✅ **Tag** - `query_tags`, `get_tag`, `create_tag`

### Existing Reconcilers

Current reconcilers implemented:

#### IPAM
- ✅ `NetBoxPrefix` - `controllers/netbox/src/reconciler/ipam/prefix.rs`
- ✅ `NetBoxAggregate` - `controllers/netbox/src/reconciler/ipam/aggregate.rs`
- ✅ `NetBoxRIR` - `controllers/netbox/src/reconciler/ipam/rir.rs`
- ✅ `NetBoxVLAN` - `controllers/netbox/src/reconciler/dcim/vlan.rs`
- ✅ `IPPool` (Custom) - `controllers/netbox/src/reconciler/ipam/ip_pool.rs`
- ✅ `IPClaim` (Custom) - `controllers/netbox/src/reconciler/ipam/ip_claim.rs`
- ❌ **NetBoxIPAddress** - Missing

#### DCIM
- ✅ `NetBoxSite` - `controllers/netbox/src/reconciler/dcim/site.rs`
- ✅ `NetBoxRegion` - `controllers/netbox/src/reconciler/dcim/region.rs`
- ✅ `NetBoxSiteGroup` - `controllers/netbox/src/reconciler/dcim/site_group.rs`
- ✅ `NetBoxLocation` - `controllers/netbox/src/reconciler/dcim/location.rs`
- ✅ `NetBoxDeviceRole` - `controllers/netbox/src/reconciler/dcim/device_role.rs`
- ✅ `NetBoxManufacturer` - `controllers/netbox/src/reconciler/dcim/manufacturer.rs`
- ✅ `NetBoxPlatform` - `controllers/netbox/src/reconciler/dcim/platform.rs`
- ✅ `NetBoxDeviceType` - `controllers/netbox/src/reconciler/dcim/device_type.rs`
- ✅ `NetBoxDevice` - `controllers/netbox/src/reconciler/dcim/device.rs`
- ✅ `NetBoxInterface` - `controllers/netbox/src/reconciler/dcim/interface.rs`
- ✅ `NetBoxMACAddress` - `controllers/netbox/src/reconciler/dcim/mac_address.rs`

#### Tenancy
- ✅ `NetBoxTenant` - `controllers/netbox/src/reconciler/tenancy.rs`
- ❌ **NetBoxTenantGroup** - Missing

#### Extras
- ✅ `NetBoxRole` (IPAM Role) - `controllers/netbox/src/reconciler/extras.rs`
- ✅ `NetBoxTag` - `controllers/netbox/src/reconciler/extras.rs`

## Missing Reconcilers

Based on the analysis, the following reconcilers are missing to complete full NetBox support:

### 1. NetBoxIPAddress (IPAM)

**Status:** ❌ Missing

**Client Methods Available:**
- `query_ip_addresses(filters, fetch_all) -> Result<Vec<IPAddress>>`
- `get_ip_address(id) -> Result<IPAddress>`
- `create_ip_address(address, request) -> Result<IPAddress>`
- `update_ip_address(id, request) -> Result<IPAddress>`
- `delete_ip_address(id) -> Result<()>`
- `allocate_ip(prefix_id, request) -> Result<IPAddress>`
- `get_available_ips(prefix_id, limit) -> Result<Vec<AvailableIP>>`

**NetBox Model:** `IPAddress` (in `crates/netbox-client/src/models.rs`)

**Complexity:** Medium
- Has dependencies: `tenant`, `vrf`, `vlan`, `role`, `tags`
- Supports both explicit IP creation and allocation from prefix
- Has update and delete operations

**Priority:** Medium
- IPAddresses are typically managed via `IPClaim` CRD (which allocates from prefixes)
- Direct IPAddress CRD would be useful for static IP management
- Supports update and delete operations (unlike most other resources)

### 2. NetBoxTenantGroup (Tenancy)

**Status:** ❌ Missing

**Client Methods Available:**
- `query_tenant_groups(filters, fetch_all) -> Result<Vec<TenantGroup>>`
- `get_tenant_group_by_name(name) -> Result<Option<TenantGroup>>`
- `create_tenant_group(name, slug, description, comments, parent_id) -> Result<TenantGroup>`

**NetBox Model:** `TenantGroup` (in `crates/netbox-client/src/models.rs`)

**Complexity:** Low
- Simple resource with hierarchical support (parent_id)
- No update method (NetBox API limitation)
- Shared resource (not tenant-specific)

**Priority:** Low
- TenantGroups are organizational structures
- Not critical for core functionality
- Can be created manually in NetBox if needed

## Implementation Plan

### Phase 1: NetBoxIPAddress Reconciler (Priority: Medium)

#### 1.1 CRD Definition
- [ ] Create `crates/crds/src/ipam/netbox_ip_address.rs`
- [ ] Define `NetBoxIPAddressSpec` with:
  - `address: IpNet` (required) - IP address with CIDR
  - `tenant: Option<NetBoxResourceReference>` (optional)
  - `vrf: Option<NetBoxResourceReference>` (optional)
  - `vlan: Option<NetBoxResourceReference>` (optional)
  - `role: Option<NetBoxResourceReference>` (optional)
  - `status: IPAddressStatus` (default: Active)
  - `dns_name: Option<String>`
  - `description: Option<String>`
  - `tags: Option<Vec<String>>`
- [ ] Define `NetBoxIPAddressStatus` with:
  - `netbox_id: Option<u64>`
  - `netbox_url: Option<String>`
  - `state: ResourceState`
  - `error: Option<String>`
- [ ] Update `crates/crds/src/ipam/mod.rs` to export
- [ ] Add to `crates/crds/src/bin/crdgen.rs`

#### 1.2 NetBox Client
- [x] Client methods already exist in `crates/netbox-client/src/ipam/ip_address.rs`
- [x] Model already exists: `IPAddress` in `crates/netbox-client/src/models.rs`
- [x] Trait methods already exist in `crates/netbox-client/src/trait.rs`

#### 1.3 Reconciliation Logic
- [ ] Create `controllers/netbox/src/reconciler/ipam/ip_address.rs`
- [ ] Implement `reconcile_netbox_ip_address()`:
  - Extract name and namespace
  - Resolve optional dependencies (tenant, vrf, vlan, role)
  - Validate status and check for drift
  - Create or update IP address
  - Handle delete operation (if CR is deleted)
  - Update status
  - Emit events
- [ ] Add API client to `Reconciler` struct
- [ ] Update `Reconciler::new()` to accept API client
- [ ] Update `startup_reconciliation()` to map existing IP addresses

#### 1.4 Watcher Setup
- [ ] Add API client to `Watcher` struct
- [ ] Create `watch_netbox_ip_addresses()` method
- [ ] Use `watch_resource()` helper

#### 1.5 Controller Integration
- [ ] Add API client to `Controller::new()`
- [ ] Add watcher `JoinHandle` to `Controller` struct
- [ ] Spawn watcher task
- [ ] Add branch to `tokio::select!` in `Controller::run()`

#### 1.6 RBAC
- [ ] Add permissions for `netboxipaddresses` CRD
- [ ] Add permissions for status subresource

#### 1.7 Example CR
- [ ] Create `config/examples/netbox-ip-address-example.yaml`

#### 1.8 Tests
- [ ] Create `controllers/netbox/src/reconciler/ipam/ip_address_test.rs`
- [ ] Test create path
- [ ] Test update path
- [ ] Test delete path
- [ ] Test drift detection
- [ ] Test dependency resolution
- [ ] Test event emission
- [ ] Verify status updates

#### 1.9 Verification
- [ ] Compilation: `python3 scripts/host_aware_build.py --release -p netbox-controller`
- [ ] CRD Generation: `cargo run -p crds --bin crdgen`
- [ ] Tests pass: `cargo test --package netbox-controller`
- [ ] Coverage meets minimum: `cargo llvm-cov --package netbox-controller --bin netbox-controller`

**Estimated Effort:** 4-6 hours

### Phase 2: NetBoxTenantGroup Reconciler (Priority: Low)

#### 2.1 CRD Definition
- [ ] Create `crates/crds/src/tenancy/netbox_tenant_group.rs`
- [ ] Define `NetBoxTenantGroupSpec` with:
  - `name: String` (required)
  - `slug: Option<String>`
  - `description: Option<String>`
  - `comments: Option<String>`
  - `parent: Option<NetBoxResourceReference>` (optional - hierarchical)
- [ ] Define `NetBoxTenantGroupStatus` with:
  - `netbox_id: Option<u64>`
  - `netbox_url: Option<String>`
  - `state: ResourceState`
  - `error: Option<String>`
- [ ] Update `crates/crds/src/tenancy/mod.rs` to export
- [ ] Add to `crates/crds/src/bin/crdgen.rs`

#### 2.2 NetBox Client
- [x] Client methods already exist in `crates/netbox-client/src/tenancy/tenant_group.rs`
- [x] Model already exists: `TenantGroup` in `crates/netbox-client/src/models.rs`
- [x] Trait methods already exist in `crates/netbox-client/src/trait.rs`

#### 2.3 Reconciliation Logic
- [ ] Create `controllers/netbox/src/reconciler/tenancy/tenant_group.rs`
- [ ] Implement `reconcile_netbox_tenant_group()`:
  - Extract name and namespace
  - Resolve optional parent dependency
  - Validate status and check for drift
  - Create tenant group (no update method in NetBox API)
  - Update status
  - Emit events
- [ ] Add API client to `Reconciler` struct
- [ ] Update `Reconciler::new()` to accept API client
- [ ] Update `startup_reconciliation()` to map existing tenant groups

#### 2.4 Watcher Setup
- [ ] Add API client to `Watcher` struct
- [ ] Create `watch_netbox_tenant_groups()` method
- [ ] Use `watch_resource()` helper

#### 2.5 Controller Integration
- [ ] Add API client to `Controller::new()`
- [ ] Add watcher `JoinHandle` to `Controller` struct
- [ ] Spawn watcher task
- [ ] Add branch to `tokio::select!` in `Controller::run()`

#### 2.6 RBAC
- [ ] Add permissions for `netboxtenantgroups` CRD
- [ ] Add permissions for status subresource

#### 2.7 Example CR
- [ ] Create `config/examples/netbox-tenant-group-example.yaml`

#### 2.8 Tests
- [ ] Create `controllers/netbox/src/reconciler/tenancy/tenant_group_test.rs`
- [ ] Test create path
- [ ] Test hierarchical parent relationship
- [ ] Test drift detection
- [ ] Test event emission
- [ ] Verify status updates

#### 2.9 Verification
- [ ] Compilation: `python3 scripts/host_aware_build.py --release -p netbox-controller`
- [ ] CRD Generation: `cargo run -p crds --bin crdgen`
- [ ] Tests pass: `cargo test --package netbox-controller`
- [ ] Coverage meets minimum: `cargo llvm-cov --package netbox-controller --bin netbox-controller`

**Estimated Effort:** 3-4 hours

## Summary Table

| Resource | Category | Client Methods | CRD Exists | Reconciler Exists | Priority | Complexity | Estimated Effort |
|----------|----------|----------------|------------|-------------------|----------|------------|-----------------|
| **NetBoxIPAddress** | IPAM | ✅ Complete | ❌ Missing | ❌ Missing | Medium | Medium | 4-6 hours |
| **NetBoxTenantGroup** | Tenancy | ✅ Complete | ❌ Missing | ❌ Missing | Low | Low | 3-4 hours |

## Implementation Guidelines

All implementations must follow the patterns established in `CONTRIBUTING.md`:

1. **Modular Structure**: Create proper module files from the start
2. **TDD**: Write tests before implementation
3. **DRY**: Use existing helpers (`reconcile_helpers`, `validate_status_and_drift`, etc.)
4. **Event Emission**: Emit events for all operations
5. **Error Handling**: Update status with error messages
6. **GitOps Compliance**: Handle conflicts by querying for existing resources
7. **Test Coverage**: Minimum 65%, target 80%
8. **Documentation**: All public items must be documented

## Next Steps

1. **Start with NetBoxIPAddress** (higher priority, more complex)
2. **Follow complete checklist** from CONTRIBUTING.md section "Adding New Reconcilers"
3. **Verify each step** before moving to the next
4. **Commit frequently** with descriptive messages
5. **Run full verification** before marking complete

## Notes

- **IPAddress** is unique in that it supports both create and delete operations (most resources only support create/update)
- **TenantGroup** has no update method in NetBox API (create-only resource)
- Both resources are relatively straightforward compared to complex resources like `Device` or `Interface`
- All client methods already exist, so implementation is primarily CRD + Reconciler + Watcher + Controller integration

