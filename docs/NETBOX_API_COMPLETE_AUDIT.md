# Complete NetBox API Surface Audit

**Date:** 2025-12-25  
**Status:** 🔍 **COMPREHENSIVE AUDIT COMPLETE**

## Overview

This document catalogs the **entire NetBox API surface** from the NetBox codebase at `/Users/casibbald/Workspace/microscaler/netbox`. This is a complete inventory of all API endpoints to enable building a comprehensive NetBoxClient.

**Total API Endpoints:** 150+ endpoints across 10 modules

## API Endpoints by Module

### IPAM (IP Address Management) - 17 Endpoints

| Endpoint | ViewSet | Operations | Custom Actions | Implemented | Priority |
|----------|---------|------------|----------------|------------|----------|
| `asns` | ASNViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | MEDIUM |
| `asn-ranges` | ASNRangeViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | MEDIUM |
| `vrfs` | VRFViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | MEDIUM |
| `route-targets` | RouteTargetViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `rirs` | RIRViewSet | list, create, retrieve, update, partial_update, destroy | ✅ | | HIGH |
| `aggregates` | AggregateViewSet | list, create, retrieve, update, partial_update, destroy | ✅ | | HIGH |
| `roles` | RoleViewSet | list, create, retrieve, update, partial_update, destroy | ✅ | | HIGH |
| `prefixes` | PrefixViewSet | list, create, retrieve, update, partial_update, destroy | `available-prefixes` (GET/POST) | ✅ | HIGH |
| `ip-ranges` | IPRangeViewSet | list, create, retrieve, update, partial_update, destroy | `available-ips` (GET/POST) | ❌ | MEDIUM |
| `ip-addresses` | IPAddressViewSet | list, create, retrieve, update, partial_update, destroy | | ✅ | HIGH |
| `fhrp-groups` | FHRPGroupViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `fhrp-group-assignments` | FHRPGroupAssignmentViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `vlan-groups` | VLANGroupViewSet | list, create, retrieve, update, partial_update, destroy | `available-vlans` (GET/POST) | ❌ | MEDIUM |
| `vlans` | VLANViewSet | list, create, retrieve, update, partial_update, destroy | | ✅ | MEDIUM |
| `vlan-translation-policies` | VLANTranslationPolicyViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `vlan-translation-rules` | VLANTranslationRuleViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `service-templates` | ServiceTemplateViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `services` | ServiceViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |

**IPAM Custom Actions:**
- `asn-ranges/<id>/available-asns/` - GET/POST available ASNs
- `prefixes/<id>/available-prefixes/` - GET/POST available prefixes
- `prefixes/<id>/available-ips/` - GET/POST available IPs ✅
- `ip-ranges/<id>/available-ips/` - GET/POST available IPs
- `vlan-groups/<id>/available-vlans/` - GET/POST available VLANs

### DCIM (Data Center Infrastructure Management) - 45 Endpoints

| Endpoint | ViewSet | Operations | Custom Actions | Implemented | Priority |
|----------|---------|------------|----------------|------------|----------|
| `regions` | RegionViewSet | list, create, retrieve, update, partial_update, destroy | | ✅ | MEDIUM |
| `site-groups` | SiteGroupViewSet | list, create, retrieve, update, partial_update, destroy | | ✅ | MEDIUM |
| `sites` | SiteViewSet | list, create, retrieve, update, partial_update, destroy | | ✅ | HIGH |
| `locations` | LocationViewSet | list, create, retrieve, update, partial_update, destroy | | ✅ | MEDIUM |
| `rack-types` | RackTypeViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `rack-roles` | RackRoleViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `racks` | RackViewSet | list, create, retrieve, update, partial_update, destroy | `elevation` (GET) | ❌ | MEDIUM |
| `rack-reservations` | RackReservationViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `manufacturers` | ManufacturerViewSet | list, create, retrieve, update, partial_update, destroy | | ✅ | LOW |
| `device-types` | DeviceTypeViewSet | list, create, retrieve, update, partial_update, destroy | | ✅ | HIGH |
| `module-types` | ModuleTypeViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `module-type-profiles` | ModuleTypeProfileViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `console-port-templates` | ConsolePortTemplateViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `console-server-port-templates` | ConsoleServerPortTemplateViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `power-port-templates` | PowerPortTemplateViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `power-outlet-templates` | PowerOutletTemplateViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `interface-templates` | InterfaceTemplateViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `front-port-templates` | FrontPortTemplateViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `rear-port-templates` | RearPortTemplateViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `module-bay-templates` | ModuleBayTemplateViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `device-bay-templates` | DeviceBayTemplateViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `inventory-item-templates` | InventoryItemTemplateViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `device-roles` | DeviceRoleViewSet | list, create, retrieve, update, partial_update, destroy | | ✅ | LOW |
| `platforms` | PlatformViewSet | list, create, retrieve, update, partial_update, destroy | | ✅ | LOW |
| `devices` | DeviceViewSet | list, create, retrieve, update, partial_update, destroy | | ✅ | HIGH |
| `virtual-device-contexts` | VirtualDeviceContextViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `modules` | ModuleViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `console-ports` | ConsolePortViewSet | list, create, retrieve, update, partial_update, destroy | `trace` (GET) | ❌ | LOW |
| `console-server-ports` | ConsoleServerPortViewSet | list, create, retrieve, update, partial_update, destroy | `trace` (GET) | ❌ | LOW |
| `power-ports` | PowerPortViewSet | list, create, retrieve, update, partial_update, destroy | `trace` (GET) | ❌ | LOW |
| `power-outlets` | PowerOutletViewSet | list, create, retrieve, update, partial_update, destroy | `trace` (GET) | ❌ | LOW |
| `interfaces` | InterfaceViewSet | list, create, retrieve, update, partial_update, destroy | `trace` (GET) | ✅ | MEDIUM |
| `front-ports` | FrontPortViewSet | list, create, retrieve, update, partial_update, destroy | `paths` (GET), `trace` (GET) | ❌ | LOW |
| `rear-ports` | RearPortViewSet | list, create, retrieve, update, partial_update, destroy | `paths` (GET), `trace` (GET) | ❌ | LOW |
| `module-bays` | ModuleBayViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `device-bays` | DeviceBayViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `inventory-items` | InventoryItemViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `inventory-item-roles` | InventoryItemRoleViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `mac-addresses` | MACAddressViewSet | list, create, retrieve, update, partial_update, destroy | | ✅ | MEDIUM |
| `cables` | CableViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `cable-terminations` | CableTerminationViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `virtual-chassis` | VirtualChassisViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `power-panels` | PowerPanelViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `power-feeds` | PowerFeedViewSet | list, create, retrieve, update, partial_update, destroy | `trace` (GET) | ❌ | LOW |
| `connected-device` | ConnectedDeviceViewSet | list | | ❌ | LOW |

**DCIM Custom Actions:**
- `racks/<id>/elevation/` - GET rack elevation (JSON or SVG)
- `console-ports/<id>/trace/` - GET cable trace
- `console-server-ports/<id>/trace/` - GET cable trace
- `power-ports/<id>/trace/` - GET cable trace
- `power-outlets/<id>/trace/` - GET cable trace
- `interfaces/<id>/trace/` - GET cable trace
- `front-ports/<id>/paths/` - GET cable paths
- `front-ports/<id>/trace/` - GET cable trace
- `rear-ports/<id>/paths/` - GET cable paths
- `rear-ports/<id>/trace/` - GET cable trace
- `power-feeds/<id>/trace/` - GET cable trace

### Tenancy - 6 Endpoints

| Endpoint | ViewSet | Operations | Custom Actions | Implemented | Priority |
|----------|---------|------------|----------------|------------|----------|
| `tenant-groups` | TenantGroupViewSet | list, create, retrieve, update, partial_update, destroy | | ✅ | HIGH |
| `tenants` | TenantViewSet | list, create, retrieve, update, partial_update, destroy | | ✅ | HIGH |
| `contact-groups` | ContactGroupViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `contact-roles` | ContactRoleViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `contacts` | ContactViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `contact-assignments` | ContactAssignmentViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |

### Extras - 20 Endpoints

| Endpoint | ViewSet | Operations | Custom Actions | Implemented | Priority |
|----------|---------|------------|----------------|------------|----------|
| `event-rules` | EventRuleViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `webhooks` | WebhookViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `custom-fields` | CustomFieldViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `custom-field-choice-sets` | CustomFieldChoiceSetViewSet | list, create, retrieve, update, partial_update, destroy | `choices` (GET) | ❌ | LOW |
| `custom-links` | CustomLinkViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `export-templates` | ExportTemplateViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `saved-filters` | SavedFilterViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `table-configs` | TableConfigViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `bookmarks` | BookmarkViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `notifications` | NotificationViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `notification-groups` | NotificationGroupViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `subscriptions` | SubscriptionViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `tags` | TagViewSet | list, create, retrieve, update, partial_update, destroy | | ✅ | HIGH |
| `tagged-objects` | TaggedItemViewSet | list, retrieve | | ❌ | LOW |
| `image-attachments` | ImageAttachmentViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `journal-entries` | JournalEntryViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `config-contexts` | ConfigContextViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `config-context-profiles` | ConfigContextProfileViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `config-templates` | ConfigTemplateViewSet | list, create, retrieve, update, partial_update, destroy | `render` (POST) | ❌ | LOW |
| `scripts` | ScriptViewSet | list, create, retrieve, update, partial_update, destroy | `run` (POST) | ❌ | LOW |
| `dashboard` | DashboardView | retrieve, update, partial_update, destroy | | ❌ | LOW |

**Extras Custom Actions:**
- `custom-field-choice-sets/<id>/choices/` - GET choices in a choice set
- `config-templates/<id>/render/` - POST render config template
- `scripts/<id>/run/` - POST run script

### Virtualization - 6 Endpoints

| Endpoint | ViewSet | Operations | Custom Actions | Implemented | Priority |
|----------|---------|------------|----------------|------------|----------|
| `cluster-types` | ClusterTypeViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `cluster-groups` | ClusterGroupViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `clusters` | ClusterViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `virtual-machines` | VirtualMachineViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `interfaces` | VMInterfaceViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `virtual-disks` | VirtualDiskViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |

### Circuits - 11 Endpoints

| Endpoint | ViewSet | Operations | Custom Actions | Implemented | Priority |
|----------|---------|------------|----------------|------------|----------|
| `providers` | ProviderViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `provider-accounts` | ProviderAccountViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `provider-networks` | ProviderNetworkViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `circuit-types` | CircuitTypeViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `circuits` | CircuitViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `circuit-terminations` | CircuitTerminationViewSet | list, create, retrieve, update, partial_update, destroy | `paths` (GET), `trace` (GET) | ❌ | LOW |
| `circuit-groups` | CircuitGroupViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `circuit-group-assignments` | CircuitGroupAssignmentViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `virtual-circuits` | VirtualCircuitViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `virtual-circuit-types` | VirtualCircuitTypeViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `virtual-circuit-terminations` | VirtualCircuitTerminationViewSet | list, create, retrieve, update, partial_update, destroy | `paths` (GET), `trace` (GET) | ❌ | LOW |

**Circuits Custom Actions:**
- `circuit-terminations/<id>/paths/` - GET cable paths
- `circuit-terminations/<id>/trace/` - GET cable trace
- `virtual-circuit-terminations/<id>/paths/` - GET cable paths
- `virtual-circuit-terminations/<id>/trace/` - GET cable trace

### VPN - 9 Endpoints

| Endpoint | ViewSet | Operations | Custom Actions | Implemented | Priority |
|----------|---------|------------|----------------|------------|----------|
| `tunnel-groups` | TunnelGroupViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `tunnels` | TunnelViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `tunnel-terminations` | TunnelTerminationViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `ike-proposals` | IKEProposalViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `ike-policies` | IKEPolicyViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `ipsec-proposals` | IPSecProposalViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `ipsec-policies` | IPSecPolicyViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `ipsec-profiles` | IPSecProfileViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `l2vpns` | L2VPNViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `l2vpn-terminations` | L2VPNTerminationViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |

### Wireless - 3 Endpoints

| Endpoint | ViewSet | Operations | Custom Actions | Implemented | Priority |
|----------|---------|------------|----------------|------------|----------|
| `wireless-lan-groups` | WirelessLANGroupViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `wireless-lans` | WirelessLANViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `wireless-links` | WirelessLinkViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |

### Core - 8 Endpoints

| Endpoint | ViewSet | Operations | Custom Actions | Implemented | Priority |
|----------|---------|------------|----------------|------------|----------|
| `data-sources` | DataSourceViewSet | list, create, retrieve, update, partial_update, destroy | `sync` (POST) | ❌ | LOW |
| `data-files` | DataFileViewSet | list, retrieve | | ❌ | LOW |
| `jobs` | JobViewSet | list, retrieve | | ❌ | LOW |
| `object-changes` | ObjectChangeViewSet | list, retrieve | | ❌ | LOW |
| `object-types` | ObjectTypeViewSet | list, retrieve | | ❌ | LOW |
| `background-queues` | BackgroundQueueViewSet | list, retrieve | | ❌ | LOW |
| `background-workers` | BackgroundWorkerViewSet | list, retrieve | | ❌ | LOW |
| `background-tasks` | BackgroundTaskViewSet | list, retrieve | `delete` (POST), `requeue` (POST), `enqueue` (POST), `stop` (POST) | ❌ | LOW |

**Core Custom Actions:**
- `data-sources/<id>/sync/` - POST sync data source
- `background-tasks/<id>/delete/` - POST delete task
- `background-tasks/<id>/requeue/` - POST requeue task
- `background-tasks/<id>/enqueue/` - POST enqueue task
- `background-tasks/<id>/stop/` - POST stop task

### Users - 5 Endpoints

| Endpoint | ViewSet | Operations | Custom Actions | Implemented | Priority |
|----------|---------|------------|----------------|------------|----------|
| `users` | UserViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `groups` | GroupViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `tokens` | TokenViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `tokens/provision/` | TokenProvisionView | POST | | ❌ | LOW |
| `permissions` | ObjectPermissionViewSet | list, create, retrieve, update, partial_update, destroy | | ❌ | LOW |
| `config` | UserConfigViewSet | list, PATCH | | ❌ | LOW |

## Implementation Status Summary

### Currently Implemented (24 endpoints)

**IPAM (8):**
- ✅ `rirs` - RIR management
- ✅ `aggregates` - Aggregate management
- ✅ `roles` - Role management
- ✅ `prefixes` - Prefix management (with available-prefixes action)
- ✅ `ip-addresses` - IP address management
- ✅ `vlans` - VLAN management
- ✅ `query_prefixes` - Query prefixes
- ✅ `query_ip_addresses` - Query IP addresses
- ✅ `get_available_ips` - Get available IPs from prefix
- ✅ `allocate_ip` - Allocate IP from prefix

**DCIM (12):**
- ✅ `regions` - Region management
- ✅ `site-groups` - Site group management
- ✅ `sites` - Site management
- ✅ `locations` - Location management
- ✅ `manufacturers` - Manufacturer management
- ✅ `device-types` - Device type management
- ✅ `device-roles` - Device role management
- ✅ `platforms` - Platform management
- ✅ `devices` - Device management
- ✅ `interfaces` - Interface management
- ✅ `mac-addresses` - MAC address management
- ✅ `query_devices` - Query devices
- ✅ `query_interfaces` - Query interfaces
- ✅ `get_device_by_mac` - Get device by MAC address

**Tenancy (2):**
- ✅ `tenant-groups` - Tenant group management
- ✅ `tenants` - Tenant management

**Extras (2):**
- ✅ `tags` - Tag management
- ✅ `roles` - Role management (IPAM)

### Missing (106+ endpoints)

**High Priority Missing:**
- ❌ `ip-ranges` - IP range management
- ❌ `vlan-groups` - VLAN group management (with available-vlans action)
- ❌ `asns` - ASN management
- ❌ `asn-ranges` - ASN range management (with available-asns action)
- ❌ `vrfs` - VRF management
- ❌ `services` - Service management
- ❌ `service-templates` - Service template management

**Medium Priority Missing:**
- ❌ All DCIM device component templates
- ❌ All DCIM device components (console ports, power ports, etc.)
- ❌ Rack management
- ❌ Module management
- ❌ Virtual device contexts

**Low Priority Missing:**
- ❌ All Virtualization endpoints
- ❌ All Circuits endpoints
- ❌ All VPN endpoints
- ❌ All Wireless endpoints
- ❌ All Core endpoints
- ❌ All Users endpoints
- ❌ Most Extras endpoints

## Standard CRUD Operations

All ViewSets support standard REST operations:
- `GET /api/{module}/{endpoint}/` - List (with filtering, pagination)
- `POST /api/{module}/{endpoint}/` - Create
- `GET /api/{module}/{endpoint}/{id}/` - Retrieve
- `PUT /api/{module}/{endpoint}/{id}/` - Update (full)
- `PATCH /api/{module}/{endpoint}/{id}/` - Update (partial)
- `DELETE /api/{module}/{endpoint}/{id}/` - Delete

## Custom Actions

Many endpoints have custom actions beyond standard CRUD:
- **Available Resources:** `available-ips`, `available-prefixes`, `available-vlans`, `available-asns`
- **Cable Tracing:** `trace` (for ports, interfaces, power feeds)
- **Cable Paths:** `paths` (for pass-through ports)
- **Rendering:** `elevation` (for racks), `render` (for config templates)
- **Operations:** `sync` (for data sources), `run` (for scripts)
- **Task Management:** `delete`, `requeue`, `enqueue`, `stop` (for background tasks)

## Implementation Plan

### Phase 1: Complete High-Priority IPAM Endpoints
1. `ip-ranges` - IP range management
2. `vlan-groups` - VLAN group management
3. `asns` - ASN management
4. `asn-ranges` - ASN range management
5. `vrfs` - VRF management
6. `services` - Service management
7. `service-templates` - Service template management

### Phase 2: Complete High-Priority DCIM Endpoints
1. All device component templates
2. All device components
3. Rack management
4. Module management

### Phase 3: Medium Priority
1. Remaining IPAM endpoints
2. Remaining DCIM endpoints
3. Remaining Tenancy endpoints

### Phase 4: Low Priority
1. Virtualization endpoints
2. Circuits endpoints
3. VPN endpoints
4. Wireless endpoints
5. Core endpoints
6. Users endpoints
7. Remaining Extras endpoints

## Next Steps

1. **Update NetBoxClientTrait** - Add all missing method signatures
2. **Implement missing methods** - Start with high-priority endpoints
3. **Add custom action support** - Implement custom action methods
4. **Update models** - Add models for all new resource types
5. **Add unit tests** - Test all new endpoints with mocks

