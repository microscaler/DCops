# NetBox Client Modularization Plan

## Problem Statement

The `crates/netbox-client/src/client.rs` file has grown to **3,503 lines**, violating modularity principles and making the codebase difficult to maintain. This violates:
- **AGENT.md Rule**: Modules should be 200-300 lines, max 500 lines
- **Single Responsibility Principle**: One file handling all resource types
- **Maintainability**: Hard to navigate, test, and extend

## Current Structure

The client has clear resource boundaries already identified:
- **IPAM**: Prefixes, IP addresses, aggregates, RIRs, VLANs
- **DCIM**: Sites, regions, site groups, locations, devices, interfaces, MAC addresses, device roles, manufacturers, platforms, device types
- **Tenancy**: Tenants, tenant groups
- **Extras**: Roles, tags

The mock implementation (`mock/`) is already modular:
- `mock/ipam.rs` - IPAM operations
- `mock/dcim.rs` - DCIM operations
- `mock/tenancy.rs` - Tenancy operations
- `mock/extras.rs` - Extras operations

## Proposed Structure

```
crates/netbox-client/src/
├── client.rs              # Main NetBoxClient (composes modules, ~200 lines)
├── core/
│   ├── mod.rs             # Core client struct (~300 lines)
│   └── helpers.rs         # Shared helper functions (~200 lines)
├── ipam/
│   ├── mod.rs             # IPAM module re-exports (~50 lines)
│   ├── prefix.rs          # Prefix operations (~200 lines)
│   ├── ip_address.rs      # IP address operations (~200 lines)
│   ├── aggregate.rs       # Aggregate operations (~100 lines)
│   ├── rir.rs             # RIR operations (~100 lines)
│   └── vlan.rs            # VLAN operations (~200 lines)
├── dcim/
│   ├── mod.rs             # DCIM module re-exports (~50 lines)
│   ├── site.rs            # Site operations (~300 lines)
│   ├── region.rs          # Region operations (~150 lines)
│   ├── site_group.rs      # Site group operations (~150 lines)
│   ├── location.rs        # Location operations (~200 lines)
│   ├── device.rs          # Device operations (~300 lines)
│   ├── interface.rs       # Interface operations (~200 lines)
│   ├── mac_address.rs     # MAC address operations (~150 lines)
│   ├── device_role.rs     # Device role operations (~150 lines)
│   ├── manufacturer.rs    # Manufacturer operations (~150 lines)
│   ├── platform.rs        # Platform operations (~200 lines)
│   └── device_type.rs     # Device type operations (~200 lines)
├── tenancy/
│   ├── mod.rs             # Tenancy module re-exports (~50 lines)
│   ├── tenant.rs          # Tenant operations (~200 lines)
│   └── tenant_group.rs    # Tenant group operations (~150 lines)
└── extras/
    ├── mod.rs             # Extras module re-exports (~50 lines)
    ├── role.rs            # Role operations (~150 lines)
    └── tag.rs             # Tag operations (~150 lines)
```

## Implementation Strategy

### Phase 1: Extract Core and Helpers
1. Create `core/mod.rs` with `NetBoxClient` struct (client, base_url, token)
2. Create `core/helpers.rs` with all helper functions (generate_slug, add_nested_reference, etc.)
3. Update `client.rs` to use `core::NetBoxClient`

### Phase 2: Extract IPAM Module
1. Create `ipam/mod.rs` with re-exports
2. Create `ipam/prefix.rs` - Extract prefix methods (~200 lines)
3. Create `ipam/ip_address.rs` - Extract IP address methods (~200 lines)
4. Create `ipam/aggregate.rs` - Extract aggregate methods (~100 lines)
5. Create `ipam/rir.rs` - Extract RIR methods (~100 lines)
6. Create `ipam/vlan.rs` - Extract VLAN methods (~200 lines)
7. Update `NetBoxClient` to compose IPAM modules

### Phase 3: Extract DCIM Module
1. Create `dcim/mod.rs` with re-exports
2. Create `dcim/site.rs` - Extract site methods (~300 lines)
3. Create `dcim/region.rs` - Extract region methods (~150 lines)
4. Create `dcim/site_group.rs` - Extract site group methods (~150 lines)
5. Create `dcim/location.rs` - Extract location methods (~200 lines)
6. Create `dcim/device.rs` - Extract device methods (~300 lines)
7. Create `dcim/interface.rs` - Extract interface methods (~200 lines)
8. Create `dcim/mac_address.rs` - Extract MAC address methods (~150 lines)
9. Create `dcim/device_role.rs` - Extract device role methods (~150 lines)
10. Create `dcim/manufacturer.rs` - Extract manufacturer methods (~150 lines)
11. Create `dcim/platform.rs` - Extract platform methods (~200 lines)
12. Create `dcim/device_type.rs` - Extract device type methods (~200 lines)
13. Update `NetBoxClient` to compose DCIM modules

### Phase 4: Extract Tenancy Module
1. Create `tenancy/mod.rs` with re-exports
2. Create `tenancy/tenant.rs` - Extract tenant methods (~200 lines)
3. Create `tenancy/tenant_group.rs` - Extract tenant group methods (~150 lines)
4. Update `NetBoxClient` to compose Tenancy modules

### Phase 5: Extract Extras Module
1. Create `extras/mod.rs` with re-exports
2. Create `extras/role.rs` - Extract role methods (~150 lines)
3. Create `extras/tag.rs` - Extract tag methods (~150 lines)
4. Update `NetBoxClient` to compose Extras modules

### Phase 6: Update Trait and Main Client
1. Update `NetBoxClientTrait` to delegate to composed modules
2. Update `client.rs` to compose all modules
3. Ensure all tests pass

## Benefits

1. **Modularity**: Each file is 100-300 lines (well within guidelines)
2. **Maintainability**: Easy to find and modify resource-specific code
3. **Testability**: Can test each resource type independently
4. **Extensibility**: Easy to add new resource types or methods
5. **Consistency**: Matches existing mock structure
6. **Single Responsibility**: Each file handles one resource type
7. **No Code Smell**: No files over 500 lines

## Migration Strategy

1. **Incremental**: Extract one module at a time
2. **Test-driven**: Verify compilation and tests after each extraction
3. **Backward compatible**: Keep `NetBoxClientTrait` interface unchanged
4. **Documentation**: Update module docs as we go

## Estimated Impact

- **client.rs**: 3,503 lines → ~200 lines (94% reduction)
- **core/mod.rs**: ~300 lines
- **core/helpers.rs**: ~200 lines
- **ipam/**: 5 files, ~750 lines total (avg 150 lines/file)
  - `mod.rs`: ~50 lines
  - `prefix.rs`: ~200 lines
  - `ip_address.rs`: ~200 lines
  - `aggregate.rs`: ~100 lines
  - `rir.rs`: ~100 lines
  - `vlan.rs`: ~200 lines
- **dcim/**: 11 files, ~2,200 lines total (avg 200 lines/file)
  - `mod.rs`: ~50 lines
  - `site.rs`: ~300 lines
  - `region.rs`: ~150 lines
  - `site_group.rs`: ~150 lines
  - `location.rs`: ~200 lines
  - `device.rs`: ~300 lines
  - `interface.rs`: ~200 lines
  - `mac_address.rs`: ~150 lines
  - `device_role.rs`: ~150 lines
  - `manufacturer.rs`: ~150 lines
  - `platform.rs`: ~200 lines
  - `device_type.rs`: ~200 lines
- **tenancy/**: 3 files, ~400 lines total (avg 133 lines/file)
  - `mod.rs`: ~50 lines
  - `tenant.rs`: ~200 lines
  - `tenant_group.rs`: ~150 lines
- **extras/**: 3 files, ~350 lines total (avg 117 lines/file)
  - `mod.rs`: ~50 lines
  - `role.rs`: ~150 lines
  - `tag.rs`: ~150 lines

**Total**: ~3,000 lines across 25 files (same functionality, properly organized)
**Max file size**: ~300 lines (well within 500 line guideline)

