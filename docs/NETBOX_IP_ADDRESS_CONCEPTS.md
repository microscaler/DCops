# NetBox IP Address Concepts

## Overview

This document explains the key concepts related to NetBox IP addresses, including assignment, NAT, primary IP, and OOB IP.

## IP Address Fields

### Tenant
- **Field**: `tenant` (optional)
- **Type**: Tenant ID (integer) or Tenant reference
- **Purpose**: Associates the IP address with a tenant for multi-tenancy support
- **Status**: ✅ **FIXED** - Now properly populated in create/update requests

### Tags
- **Field**: `tags` (optional)
- **Type**: Array of tag IDs (integers) or tag dictionaries with `name`/`slug` keys
- **Purpose**: Categorizes and organizes IP addresses
- **Status**: ✅ **FIXED** - Now properly resolved from `NetBoxResourceReference` to tag IDs

### Assignment
- **Fields**: 
  - `assigned_object_type`: ContentType string (e.g., "dcim.interface", "virtualization.vminterface", "ipam.fhrpgroup")
  - `assigned_object_id`: Integer ID of the assigned object
  - `assigned_object`: Read-only field showing the assigned object details
- **Purpose**: Links an IP address to a specific interface, VM interface, or FHRP group
- **Status**: ⚠️ **NOT YET IMPLEMENTED** - Fields exist in the model but not in the CRD or reconciler

**Supported Assignment Types** (from NetBox source):
- `dcim.interface` - Physical or virtual network interface on a device
- `virtualization.vminterface` - Virtual machine interface
- `ipam.fhrpgroup` - First Hop Redundancy Protocol group (VRRP, HSRP, GLBP, CARP)

### NAT (Network Address Translation)

#### NAT Inside
- **Field**: `nat_inside` (optional)
- **Type**: Single IP address reference (`NestedIPAddress`)
- **Purpose**: Points to the "inside" (private/internal) IP address when this IP is the "outside" (public/external) address
- **Example**: If this IP is `203.0.113.1` (public), `nat_inside` might point to `192.168.1.10` (private)
- **Status**: ⚠️ **NOT YET IMPLEMENTED** - Field exists in the model but not in the CRD or reconciler

#### NAT Outside
- **Field**: `nat_outside` (read-only)
- **Type**: Array of IP address references (`Vec<NestedIPAddress>`)
- **Purpose**: Lists all "outside" (public/external) IP addresses that point to this IP as their "inside" address
- **Example**: If this IP is `192.168.1.10` (private), `nat_outside` might contain `[203.0.113.1, 203.0.113.2]` (public IPs)
- **Status**: ⚠️ **READ-ONLY** - Managed by NetBox automatically, cannot be set via API

### Primary IP (on Device)

**Note**: Primary IP is set on the **Device** model, not the IPAddress model.

- **Fields**:
  - `primary_ip4`: OneToOneField to IPAddress (IPv4)
  - `primary_ip6`: OneToOneField to IPAddress (IPv6)
- **Purpose**: Designates the primary management/access IP address for a device
- **Location**: `../netbox/netbox/dcim/models/devices.py` (lines 592-603)
- **Status**: ⚠️ **NOT YET IMPLEMENTED** - Would need to be set via `NetBoxDevice` CRD

**Usage**:
- When a device has multiple IP addresses, the primary IP is the one used for management/SSH/console access
- Typically the IP address assigned to the device's management interface

### OOB IP (Out-of-Band IP)

**Note**: OOB IP is also set on the **Device** model, not the IPAddress model.

- **Field**: `oob_ip`: OneToOneField to IPAddress
- **Purpose**: Designates the out-of-band management IP address (separate from the primary IP)
- **Location**: `../netbox/netbox/dcim/models/devices.py` (lines 608-611)
- **Status**: ⚠️ **NOT YET IMPLEMENTED** - Would need to be set via `NetBoxDevice` CRD

**Usage**:
- OOB IP is typically used for:
  - IPMI/iDRAC/iLO management interfaces
  - Serial console servers
  - Dedicated management networks
- Separate from the primary IP which is usually the in-band management IP

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Tenant | ✅ Fixed | Now properly populated in create/update requests |
| Tags | ✅ Fixed | Resolved from `NetBoxResourceReference` to tag IDs |
| Assignment | ⚠️ Not Implemented | Fields exist in model, need CRD fields and reconciler logic |
| NAT Inside | ⚠️ Not Implemented | Field exists in model, need CRD field and reconciler logic |
| NAT Outside | ⚠️ Read-Only | Managed automatically by NetBox |
| Primary IP | ⚠️ Not Implemented | Set on Device model, not IPAddress |
| OOB IP | ⚠️ Not Implemented | Set on Device model, not IPAddress |

## References

- NetBox IP Address Serializer: `../netbox/netbox/ipam/api/serializers_/ip.py`
- NetBox Device Model: `../netbox/netbox/dcim/models/devices.py`
- NetBox IP Address Model: `../netbox/netbox/ipam/models/ip.py` (referenced in serializer)

