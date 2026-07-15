# PXE Boot & Pi Cluster Implementation Plan

> **PXE HTTP server:** see [`PXE_SERVER_IMPLEMENTATION.md`](PXE_SERVER_IMPLEMENTATION.md) (M0–M2: axum HTTP + BootIntent lookup).

**Status:** CRDs Created ✅ | API Client Methods 🚧 | Reconciliation Logic 📋

## Overview

This document tracks the implementation of NetBox CRDs required for PXE boot and Raspberry Pi cluster management.

## Completed CRDs ✅

All essential CRDs for PXE boot have been created:

1. **NetBoxDeviceRole** - Device role categorization (control-plane, worker, etc.)
2. **NetBoxManufacturer** - Hardware manufacturers (Raspberry Pi Foundation)
3. **NetBoxPlatform** - OS/platform information (Talos Linux, etc.)
4. **NetBoxDeviceType** - Device type templates (Raspberry Pi 4 Model B, etc.)
5. **NetBoxDevice** - Core device management (Pi hardware instances)
6. **NetBoxInterface** - Network interfaces on devices
7. **NetBoxMACAddress** - MAC address management (critical for PXE boot)
8. **NetBoxVLAN** - VLAN management for network segmentation

## Implementation Status

### Phase 1: CRD Definitions ✅ COMPLETE
- [x] NetBoxDeviceRole CRD
- [x] NetBoxManufacturer CRD
- [x] NetBoxPlatform CRD
- [x] NetBoxDeviceType CRD
- [x] NetBoxDevice CRD
- [x] NetBoxInterface CRD
- [x] NetBoxMACAddress CRD
- [x] NetBoxVLAN CRD
- [x] Updated crdgen.rs to include all new CRDs

### Phase 2: NetBox API Client Methods 🚧 IN PROGRESS
- [ ] Add DeviceRole API methods (create, get, query, update, delete)
- [ ] Add Manufacturer API methods
- [ ] Add Platform API methods
- [ ] Add DeviceType API methods
- [ ] Add Device API methods
- [ ] Add Interface API methods
- [ ] Add MACAddress API methods
- [ ] Add VLAN API methods

### Phase 3: Reconciliation Logic 📋 PLANNED
- [ ] Implement reconcile_netbox_device_role
- [ ] Implement reconcile_netbox_manufacturer
- [ ] Implement reconcile_netbox_platform
- [ ] Implement reconcile_netbox_device_type
- [ ] Implement reconcile_netbox_device
- [ ] Implement reconcile_netbox_interface
- [ ] Implement reconcile_netbox_mac_address
- [ ] Implement reconcile_netbox_vlan

### Phase 4: Controller Integration 📋 PLANNED
- [ ] Add watchers for all new CRDs
- [ ] Update controller to spawn watchers
- [ ] Add RBAC permissions for new CRDs
- [ ] Update startup reconciliation

### Phase 5: Example Manifests 📋 PLANNED
- [ ] Create example manifests for all new CRDs
- [ ] Create complete Pi cluster example
- [ ] Document PXE boot workflow

## PXE Boot Workflow

The typical workflow for PXE booting a Pi cluster:

1. **Create Manufacturer** - "Raspberry Pi Foundation"
2. **Create DeviceType** - "Raspberry Pi 4 Model B" (references Manufacturer)
3. **Create Platform** - "Talos Linux" (optional, references Manufacturer)
4. **Create DeviceRole** - "control-plane" or "worker"
5. **Create Site** - Already exists ✅
6. **Create VLAN** - For network segmentation (optional)
7. **Create Device** - Physical Pi instance (references DeviceType, DeviceRole, Site)
8. **Create Interface** - Network interface on device (references Device)
9. **Create MACAddress** - MAC address for interface (references Interface) - **Critical for PXE**
10. **Create IPClaim** - Allocate IP address for device (references IPPool)
11. **Update Device** - Set primary_ip4/primary_ip6 from IPClaim

## Next Steps

1. **Add API Client Methods** - Implement all CRUD operations for new resources
2. **Add Reconciliation Logic** - Implement reconciliation for each CRD
3. **Add Controller Integration** - Wire up watchers and reconciliation
4. **Create Example Manifests** - Provide working examples for Pi cluster setup
5. **Test End-to-End** - Verify complete PXE boot workflow

## Dependencies Graph

```
NetBoxManufacturer
    └── NetBoxDeviceType
    └── NetBoxPlatform (optional)
        └── NetBoxDevice
            └── NetBoxInterface
                └── NetBoxMACAddress

NetBoxDeviceRole
    └── NetBoxDevice

NetBoxSite
    └── NetBoxDevice
    └── NetBoxVLAN

NetBoxTenant
    └── NetBoxDevice
    └── NetBoxVLAN

NetBoxRole (IPAM)
    └── NetBoxVLAN

IPPool
    └── IPClaim
        └── NetBoxDevice (primary_ip4/primary_ip6)
```

## Notes

- **MAC Address Management**: Critical for PXE boot - the PXE server uses MAC addresses to identify devices
- **Device Naming**: Devices can be unnamed in NetBox, but we recommend naming them for easier management
- **Interface Types**: For Pi devices, typically "1000base-t" for Ethernet or "virtual" for virtual interfaces
- **VLAN Support**: Optional but recommended for network segmentation in multi-tenant scenarios

