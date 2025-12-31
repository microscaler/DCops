# CRD Reference

Complete reference for all 31 Custom Resource Definitions in DCops, organized by category.

## Overview

DCops provides Kubernetes CRDs for managing NetBox resources through GitOps. All CRDs use Kubernetes-compliant object references, making them GitOps-friendly and allowing you to reference other CRDs by name rather than IDs.

**API Group:** `dcops.microscaler.io`  
**Version:** `v1alpha1`  
**Namespace:** All CRDs are namespaced

## Resource Categories

- **IPAM (IP Address Management)** - 9 CRDs for IP address allocation and management
- **DCIM (Data Center Infrastructure Management)** - 11 CRDs for physical infrastructure
- **Boot Resources** - 2 CRDs for PXE boot control
- **Tenancy** - 1 CRD for tenant management
- **Extras** - 1 CRD for tags
- **IP Pool Management** - 2 CRDs for IP pool abstraction

---

## IPAM Resources (IP Address Management)

### NetBoxTenant

**Required for most resources** - Tenant is required by NetBox for proper resource organization and access control.

**Required Fields:**
- `name` - Tenant name
- `tokenSecret.name` - Kubernetes Secret containing NetBox API token

**Example:**
```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxTenant
metadata:
  name: datacenter-tenant
  namespace: default
spec:
  name: "Data Center Operations"
  slug: "datacenter-ops"
  description: "Primary tenant for datacenter operations"
  tokenSecret:
    name: netbox-token-datacenter-tenant
  reconcileInterval: 300  # Check for drift every 5 minutes
```

**See:** `config/examples/tenant-datacenter-tenant/netbox-tenant-example.yaml`

### NetBoxPrefix

Foundation for IP address management. Represents a CIDR block in NetBox.

**Required Fields:**
- `prefix` - CIDR notation (e.g., "192.168.1.0/24")
- `tenant` - Tenant reference (required)

**Optional Fields:**
- `site` - Site reference
- `aggregate` - Aggregate reference
- `vlan` - VLAN reference
- `role` - Role reference
- `tags` - Tag references
- `status` - Prefix status (active, reserved, deprecated, container)

**Example:**
```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxPrefix
metadata:
  name: control-plane-prefix
  namespace: default
spec:
  prefix: "192.168.1.0/24"
  description: "Control plane IP address pool for Talos clusters"
  status: active
  tenant:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxTenant"
    name: "datacenter-tenant"
  site:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxSite"
    name: "datacenter-1"
```

**See:** `config/examples/tenant-datacenter-tenant/netbox-prefix-example.yaml`

### NetBoxIPAddress

Represents an individual IP address in NetBox.

**Required Fields:**
- `address` - IP address with CIDR (e.g., "192.168.1.10/24")
- `tenant` - Tenant reference (required)

**Optional Fields:**
- `status` - IP status (active, reserved, deprecated, dhcp, slaac)
- `role` - IP role (loopback, secondary, anycast, vip, vrrp, hsrp, glbp, carp)
- `dnsName` - DNS name
- `description` - Description
- `tags` - Tag references

**Example:**
```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxIPAddress
metadata:
  name: control-plane-01-ip
  namespace: default
spec:
  address: "192.168.1.10/24"
  status: active
  tenant:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxTenant"
    name: "datacenter-tenant"
```

**See:** `config/examples/tenant-datacenter-tenant/netbox-ip-address-example.yaml`

### NetBoxIPRange

Represents a range of IP addresses within a prefix.

**Required Fields:**
- `startAddress` - Starting IP address
- `endAddress` - Ending IP address
- `tenant` - Tenant reference (required)

**Optional Fields:**
- `description` - Description
- `status` - Range status (active, reserved, deprecated)
- `role` - Role reference
- `tags` - Tag references

**See:** `config/examples/tenant-datacenter-tenant/netbox-ip-range-example.yaml`

### NetBoxAggregate

Represents a large IP address block (typically /8 or larger).

**Required Fields:**
- `prefix` - CIDR notation (e.g., "10.0.0.0/8")
- `rir` - RIR (Regional Internet Registry) reference
- `tenant` - Tenant reference (required)

**Optional Fields:**
- `description` - Description
- `tags` - Tag references

**See:** `config/examples/tenant-datacenter-tenant/netbox-aggregate-example.yaml`

### NetBoxVLAN

Represents a VLAN in NetBox.

**Required Fields:**
- `vid` - VLAN ID (1-4094)
- `name` - VLAN name
- `tenant` - Tenant reference (required)

**Optional Fields:**
- `site` - Site reference
- `group` - VLAN group reference
- `status` - VLAN status (active, reserved, deprecated)
- `role` - Role reference
- `tags` - Tag references

**See:** `config/examples/tenant-datacenter-tenant/netbox-vlan-example.yaml`

### NetBoxRole

Represents a functional role for IPAM resources.

**Required Fields:**
- `name` - Role name

**Optional Fields:**
- `slug` - Role slug
- `description` - Description
- `weight` - Display weight
- `tags` - Tag references

**See:** `config/examples/tenant-datacenter-tenant/netbox-role-example.yaml`

### NetBoxRIR

Represents a Regional Internet Registry (ARIN, RIPE, etc.).

**Required Fields:**
- `name` - RIR name

**Optional Fields:**
- `slug` - RIR slug
- `description` - Description

**See:** `config/examples/tenant-datacenter-tenant/netbox-rir-example.yaml`

---

## IP Pool Management

### IPPool

High-level abstraction for IP address pools. References a NetBoxPrefix CRD.

**Required Fields:**
- `netboxPrefixRef` - Reference to NetBoxPrefix CRD

**Optional Fields:**
- `role` - Pool scope/role (e.g., "control-plane", "worker")
- `allocationStrategy` - Allocation strategy (sequential, random)

**Example:**
```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPPool
metadata:
  name: control-plane-pool
  namespace: default
spec:
  netboxPrefixRef:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxPrefix"
    name: "control-plane-prefix"
  role: "control-plane"
  allocationStrategy: sequential
```

**See:** `config/examples/tenant-datacenter-tenant/ippool-example.yaml`

### IPClaim

Requests an IP address from an IPPool.

**Required Fields:**
- `poolRef` - Reference to IPPool
- `deviceRef.name` - Device name or identifier

**Optional Fields:**
- `deviceRef.interface` - Interface name
- `preferredIp` - Preferred IP hint (CIDR notation)

**Example:**
```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPClaim
metadata:
  name: talos-control-plane-01
  namespace: default
spec:
  poolRef:
    name: control-plane-pool
  deviceRef:
    name: "talos-control-plane-01"
    interface: "eth0"
  preferredIp: "192.168.1.10/24"  # Optional hint
```

**See:** `config/examples/tenant-datacenter-tenant/ipclaim-example.yaml`

---

## DCIM Resources (Data Center Infrastructure Management)

### NetBoxSite

Represents a physical location (datacenter, colocation facility, etc.).

**Required Fields:**
- `name` - Site name
- `tenant` - Tenant reference (required)

**Optional Fields:**
- `slug` - Site slug
- `description` - Description
- `status` - Site status (active, planned, retired, staging)
- `region` - Region reference
- `siteGroup` - Site group reference
- `physicalAddress` - Physical address
- `facility` - Facility code
- `timeZone` - Time zone
- `latitude` / `longitude` - Coordinates
- `tags` - Tag references

**Example:**
```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxSite
metadata:
  name: datacenter-1
  namespace: default
spec:
  name: "Data Center 1"
  slug: "datacenter-1"
  description: "Primary datacenter facility"
  status: active
  tenant:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxTenant"
    name: "datacenter-tenant"
  region:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxRegion"
    name: "us-east"
```

**See:** `config/examples/tenant-datacenter-tenant/netbox-site-example.yaml`

### NetBoxRegion

Represents a geographic region for organizing sites.

**Required Fields:**
- `name` - Region name

**Optional Fields:**
- `slug` - Region slug
- `description` - Description
- `parent` - Parent region reference (for hierarchical regions)
- `tags` - Tag references

**See:** `config/examples/tenant-datacenter-tenant/netbox-region-example.yaml`

### NetBoxSiteGroup

Alternative to regions for organizing sites.

**Required Fields:**
- `name` - Site group name

**Optional Fields:**
- `slug` - Site group slug
- `description` - Description
- `tags` - Tag references

**See:** `config/examples/tenant-datacenter-tenant/netbox-site-group-example.yaml`

### NetBoxLocation

Represents a nested location within a site (e.g., building, floor, room, rack).

**Required Fields:**
- `name` - Location name
- `site` - Site reference
- `tenant` - Tenant reference (required)

**Optional Fields:**
- `slug` - Location slug
- `description` - Description
- `parent` - Parent location reference (for nested locations)
- `status` - Location status (active, planned, retired)
- `tags` - Tag references

**See:** `config/examples/tenant-datacenter-tenant/netbox-location-example.yaml`

### NetBoxDevice

Represents a physical device (server, switch, router, etc.).

**Required Fields:**
- `deviceType` - Device type reference
- `deviceRole` - Device role reference
- `site` - Site reference
- `tenant` - Tenant reference (required)

**Optional Fields:**
- `name` - Device name
- `location` - Location reference
- `platform` - Platform reference
- `status` - Device status (active, offline, planned, staged, failed, inventory)
- `serial` - Serial number
- `assetTag` - Asset tag
- `primaryIp4` - Primary IPv4 (IPClaim reference or IP address)
- `primaryIp6` - Primary IPv6 (IPClaim reference or IP address)
- `description` - Description
- `comments` - Comments
- `tags` - Tag references

**Example:**
```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxDevice
metadata:
  name: talos-control-plane-01
  namespace: default
spec:
  name: "talos-control-plane-01"
  deviceType:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxDeviceType"
    name: "raspberry-pi-4-model-b"
  deviceRole:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxDeviceRole"
    name: "kubernetes-control-plane"
  site:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxSite"
    name: "datacenter-1"
  tenant:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxTenant"
    name: "datacenter-tenant"
  primaryIp4:
    ipClaimRef:
      apiGroup: "dcops.microscaler.io"
      kind: "IPClaim"
      name: "talos-control-plane-01"
  status: active
```

**See:** `config/examples/tenant-datacenter-tenant/netbox-device-example.yaml`

### NetBoxDeviceType

Represents a device model/type (e.g., "Raspberry Pi 4 Model B").

**Required Fields:**
- `manufacturer` - Manufacturer reference
- `model` - Device model name

**Optional Fields:**
- `slug` - Device type slug
- `partNumber` - Part number
- `uHeight` - Rack units (U height)
- `isFullDepth` - Full depth flag
- `description` - Description
- `comments` - Comments
- `tags` - Tag references

**See:** `config/examples/platform/netbox-device-type-example.yaml`

### NetBoxDeviceRole

Represents a functional role for devices (e.g., "Kubernetes Control Plane", "Worker Node").

**Required Fields:**
- `name` - Role name

**Optional Fields:**
- `slug` - Role slug
- `color` - Display color (hex code)
- `description` - Description
- `tags` - Tag references

**See:** `config/examples/platform/netbox-device-role-example.yaml`

### NetBoxManufacturer

Represents a hardware manufacturer (e.g., "Raspberry Pi Foundation", "Dell").

**Required Fields:**
- `name` - Manufacturer name

**Optional Fields:**
- `slug` - Manufacturer slug
- `description` - Description
- `tags` - Tag references

**See:** `config/examples/platform/netbox-manufacturer-example.yaml`

### NetBoxPlatform

Represents an operating system or software platform (e.g., "Talos Linux", "Ubuntu").

**Required Fields:**
- `name` - Platform name

**Optional Fields:**
- `slug` - Platform slug
- `manufacturer` - Manufacturer reference
- `napalmDriver` - NAPALM driver for network automation
- `description` - Description
- `comments` - Comments
- `tags` - Tag references

**Example:**
```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxPlatform
metadata:
  name: talos-linux
  namespace: default
spec:
  name: "Talos Linux"
  slug: "talos-linux"
  manufacturer:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxManufacturer"
    name: "raspberry-pi"
  description: "Talos Linux for Kubernetes on Raspberry Pi"
```

**See:** `config/examples/platform/netbox-platform-example.yaml`

### NetBoxInterface

Represents a network interface on a device.

**Required Fields:**
- `name` - Interface name
- `device` - Device reference
- `type` - Interface type (virtual, bridge, lag, etc.)

**Optional Fields:**
- `description` - Description
- `enabled` - Enabled flag
- `macAddress` - MAC address reference
- `mtu` - MTU size
- `tags` - Tag references

**See:** `config/examples/tenant-datacenter-tenant/netbox-interface-example.yaml`

### NetBoxMACAddress

Represents a MAC address.

**Required Fields:**
- `address` - MAC address (format: XX:XX:XX:XX:XX:XX)

**Optional Fields:**
- `description` - Description
- `tags` - Tag references

**See:** `config/examples/tenant-datacenter-tenant/netbox-mac-address-example.yaml`

---

## Boot Resources

### BootProfile

Defines a PXE boot profile configuration.

**Status:** ⚠️ Not yet implemented (stub only)

**See:** PXE Boot Control concept documentation

### BootIntent

Declares the desired boot behavior for a device.

**Status:** ⚠️ Not yet implemented (stub only)

**See:** PXE Boot Control concept documentation

---

## Extras Resources

### NetBoxTag

Represents a tag for organizing and filtering resources.

**Required Fields:**
- `name` - Tag name

**Optional Fields:**
- `slug` - Tag slug
- `color` - Display color (hex code)
- `description` - Description

**Example:**
```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxTag
metadata:
  name: managed-by-dcops
  namespace: default
spec:
  name: "managed-by-dcops"
  slug: "managed-by-dcops"
  color: "00ff00"
  description: "Resource managed by DCops controller"
```

**See:** `config/examples/tenant-datacenter-tenant/netbox-tag-example.yaml`

---

## Common Patterns

### Object References

All CRDs use Kubernetes-compliant object references:

```yaml
reference:
  apiGroup: "dcops.microscaler.io"
  kind: "NetBoxTenant"
  name: "datacenter-tenant"
  namespace: default  # Optional, defaults to same namespace
```

### Status Fields

All CRDs include status fields:
- `netboxId` - NetBox resource ID (set after creation)
- `netboxUrl` - NetBox API URL
- `state` - Resource state (Pending, Created, Updated, Failed)
- `error` - Error message if reconciliation failed
- `lastReconciled` - Last reconciliation timestamp

### Tenant Requirement

Most NetBox resources require a tenant reference. Always create a `NetBoxTenant` first:

1. Create `NetBoxTenant` CRD
2. Create Kubernetes Secret with NetBox API token
3. Reference tenant in other resources

### Example Files

All example files are available in:
- `config/examples/platform/` - Platform-level resources
- `config/examples/tenant-datacenter-tenant/` - Tenant-specific resources

---

## Quick Reference Table

| CRD | Category | Required Tenant | Example File |
|-----|----------|----------------|--------------|
| NetBoxTenant | Tenancy | No | `netbox-tenant-example.yaml` |
| NetBoxPrefix | IPAM | Yes | `netbox-prefix-example.yaml` |
| NetBoxIPAddress | IPAM | Yes | `netbox-ip-address-example.yaml` |
| NetBoxIPRange | IPAM | Yes | `netbox-ip-range-example.yaml` |
| NetBoxAggregate | IPAM | Yes | `netbox-aggregate-example.yaml` |
| NetBoxVLAN | IPAM | Yes | `netbox-vlan-example.yaml` |
| NetBoxRole | IPAM | No | `netbox-role-example.yaml` |
| NetBoxRIR | IPAM | No | `netbox-rir-example.yaml` |
| IPPool | IP Pool | No | `ippool-example.yaml` |
| IPClaim | IP Pool | No | `ipclaim-example.yaml` |
| NetBoxSite | DCIM | Yes | `netbox-site-example.yaml` |
| NetBoxRegion | DCIM | No | `netbox-region-example.yaml` |
| NetBoxSiteGroup | DCIM | No | `netbox-site-group-example.yaml` |
| NetBoxLocation | DCIM | Yes | `netbox-location-example.yaml` |
| NetBoxDevice | DCIM | Yes | `netbox-device-example.yaml` |
| NetBoxDeviceType | DCIM | No | `netbox-device-type-example.yaml` (platform/) |
| NetBoxDeviceRole | DCIM | No | `netbox-device-role-example.yaml` (platform/) |
| NetBoxManufacturer | DCIM | No | `netbox-manufacturer-example.yaml` (platform/) |
| NetBoxPlatform | DCIM | No | `netbox-platform-example.yaml` (platform/) |
| NetBoxInterface | DCIM | No | `netbox-interface-example.yaml` |
| NetBoxMACAddress | DCIM | No | `netbox-mac-address-example.yaml` |
| NetBoxTag | Extras | No | `netbox-tag-example.yaml` |
| BootProfile | Boot | No | Not implemented |
| BootIntent | Boot | No | Not implemented |

---

## Next Steps

- [Installation Guide](../getting-started/installation.md) - Deploy DCops controllers
- [Quick Start](../getting-started/quick-start.md) - Create your first resources
- [NetBox Controller](./netbox-controller.md) - Understand controller behavior
