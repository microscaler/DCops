# Code Organization

This document describes the code organization structure, which mirrors the NetBox API organization.

## Directory Structure

```
crates/crds/src/
├── lib.rs                    # Main library file
├── bin/
│   └── crdgen.rs            # CRD generation binary
│
├── boot_profile.rs          # Boot profile CRD
├── boot_intent.rs          # Boot intent CRD
├── ip_pool.rs              # IP pool CRD (custom)
├── ip_claim.rs             # IP claim CRD (custom)
│
├── dcim/                   # Data Center Infrastructure Management
│   ├── mod.rs
│   ├── netbox_site.rs
│   ├── netbox_device_role.rs
│   ├── netbox_manufacturer.rs
│   ├── netbox_platform.rs
│   ├── netbox_device_type.rs
│   ├── netbox_device.rs
│   ├── netbox_interface.rs
│   └── netbox_mac_address.rs
│
├── ipam/                   # IP Address Management
│   ├── mod.rs
│   ├── netbox_prefix.rs
│   ├── netbox_aggregate.rs
│   ├── netbox_role.rs
│   └── netbox_vlan.rs
│
├── tenancy/                # Tenancy
│   ├── mod.rs
│   └── netbox_tenant.rs
│
└── extras/                 # Extras (metadata)
    ├── mod.rs
    └── netbox_tag.rs
```

## NetBox API Mapping

The code organization directly maps to NetBox API endpoints:

### DCIM (`/api/dcim/`)
- **Sites** - `dcim/netbox_site.rs` → `/api/dcim/sites/`
- **Device Roles** - `dcim/netbox_device_role.rs` → `/api/dcim/device-roles/`
- **Manufacturers** - `dcim/netbox_manufacturer.rs` → `/api/dcim/manufacturers/`
- **Platforms** - `dcim/netbox_platform.rs` → `/api/dcim/platforms/`
- **Device Types** - `dcim/netbox_device_type.rs` → `/api/dcim/device-types/`
- **Devices** - `dcim/netbox_device.rs` → `/api/dcim/devices/`
- **Interfaces** - `dcim/netbox_interface.rs` → `/api/dcim/interfaces/`
- **MAC Addresses** - `dcim/netbox_mac_address.rs` → `/api/dcim/mac-addresses/`

### IPAM (`/api/ipam/`)
- **Prefixes** - `ipam/netbox_prefix.rs` → `/api/ipam/prefixes/`
- **Aggregates** - `ipam/netbox_aggregate.rs` → `/api/ipam/aggregates/`
- **Roles** - `ipam/netbox_role.rs` → `/api/ipam/roles/`
- **VLANs** - `ipam/netbox_vlan.rs` → `/api/ipam/vlans/`

### Tenancy (`/api/tenancy/`)
- **Tenants** - `tenancy/netbox_tenant.rs` → `/api/tenancy/tenants/`

### Extras (`/api/extras/`)
- **Tags** - `extras/netbox_tag.rs` → `/api/extras/tags/`

## Module Structure

Each module follows this pattern:

```rust
//! Module documentation
//! Maps to NetBox API: /api/{section}/{resource}/

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, ...)]
pub struct NetBoxResourceSpec { ... }

pub struct NetBoxResourceStatus {
    pub state: crate::tenancy::netbox_tenant::ResourceState,
    ...
}
```

## Benefits of This Organization

1. **Clear API Mapping** - Easy to find code for a specific NetBox API endpoint
2. **Logical Grouping** - Related resources are grouped together
3. **Scalability** - Easy to add new resources to the appropriate section
4. **Maintainability** - Clear separation of concerns
5. **Documentation** - Structure itself documents the API organization

## Adding New Resources

When adding a new NetBox resource:

1. **Identify the API section** (dcim, ipam, tenancy, extras, etc.)
2. **Create the file** in the appropriate directory: `{section}/netbox_{resource}.rs`
3. **Add to module** - Update `{section}/mod.rs` to include the new resource
4. **Update lib.rs** - The module is already re-exported, so no changes needed
5. **Update crdgen.rs** - Add the CRD to the generation list

## Example: Adding a New DCIM Resource

```rust
// crates/crds/src/dcim/netbox_rack.rs
//! NetBoxRack Custom Resource Definition
//! Maps to NetBox API: /api/dcim/racks/

// ... CRD definition ...

// crates/crds/src/dcim/mod.rs
pub mod netbox_rack;
pub use netbox_rack::*;

// crates/crds/src/bin/crdgen.rs
use crds::dcim::NetBoxRack;
// ...
crds.push(NetBoxRack::crd());
```

## Status

✅ **Complete** - All existing CRDs have been reorganized according to this structure.

