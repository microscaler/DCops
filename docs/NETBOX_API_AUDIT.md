# NetBox API Audit & CRD Mapping

This document provides a comprehensive audit of all NetBox API endpoints and their mapping to Kubernetes CRDs for GitOps management.

**Last Updated:** 2025-12-24  
**Status:** Initial audit complete, implementation in progress

## Implementation Status Legend

- ✅ **Implemented** - CRD exists and is fully functional
- 🚧 **In Progress** - CRD exists but needs completion
- 📋 **Planned** - CRD planned but not yet implemented
- ⏸️ **Deferred** - Lower priority, may not implement
- ❌ **Not Applicable** - Not suitable for GitOps/CRD management

---

## 1. ORGANIZATION - DCIM (Data Center Infrastructure Management)

### 1.1 Sites

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/dcim/regions/` | `NetBoxRegion` | 📋 Planned | name, slug, parent (Region), description | None | Hierarchical site organization |
| `/api/dcim/site-groups/` | `NetBoxSiteGroup` | 📋 Planned | name, slug, parent (SiteGroup), description | None | Alternative to regions |
| `/api/dcim/sites/` | `NetBoxSite` | ✅ Implemented | name, slug, status, region, site_group, tenant, facility, physical_address, shipping_address, lat/long, time_zone, description, comments | NetBoxRegion, NetBoxSiteGroup, NetBoxTenant | **Currently implemented** |

### 1.2 Racks

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/dcim/locations/` | `NetBoxLocation` | 📋 Planned | name, slug, site, parent (Location), description | NetBoxSite | Nested locations within sites |
| `/api/dcim/rack-types/` | `NetBoxRackType` | 📋 Planned | manufacturer, model, u_height, width, depth, weight, max_weight, description | NetBoxManufacturer | Standard rack configurations |
| `/api/dcim/rack-roles/` | `NetBoxRackRole` | 📋 Planned | name, slug, color, description | None | Rack categorization |
| `/api/dcim/racks/` | `NetBoxRack` | 📋 Planned | name, site, location, status, facility_id, tenant, role, serial, asset_tag, type, width, u_height, starting_unit, weight, max_weight, description, comments | NetBoxSite, NetBoxLocation, NetBoxRackType, NetBoxRackRole, NetBoxTenant | Physical rack management |
| `/api/dcim/rack-reservations/` | `NetBoxRackReservation` | 📋 Planned | rack, units, tenant, user, description | NetBoxRack, NetBoxTenant | Reserve rack units |

### 1.3 Device/Module Types

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/dcim/manufacturers/` | `NetBoxManufacturer` | 📋 Planned | name, slug, description | None | Hardware manufacturers |
| `/api/dcim/device-types/` | `NetBoxDeviceType` | 📋 Planned | manufacturer, model, slug, part_number, u_height, is_full_depth, subdevice_role, airflow, weight, weight_unit, description | NetBoxManufacturer | Device type templates |
| `/api/dcim/module-types/` | `NetBoxModuleType` | 📋 Planned | manufacturer, model, part_number, weight, weight_unit, description | NetBoxManufacturer | Module type templates |
| `/api/dcim/module-type-profiles/` | `NetBoxModuleTypeProfile` | 📋 Planned | name, module_type, description | NetBoxModuleType | Module profiles |

### 1.4 Device Type Components (Templates)

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/dcim/console-port-templates/` | `NetBoxConsolePortTemplate` | ⏸️ Deferred | device_type, name, type, label, description | NetBoxDeviceType | Template for console ports |
| `/api/dcim/console-server-port-templates/` | `NetBoxConsoleServerPortTemplate` | ⏸️ Deferred | device_type, name, type, label, description | NetBoxDeviceType | Template for console server ports |
| `/api/dcim/power-port-templates/` | `NetBoxPowerPortTemplate` | ⏸️ Deferred | device_type, name, type, maximum_draw, allocated_draw, label, description | NetBoxDeviceType | Template for power ports |
| `/api/dcim/power-outlet-templates/` | `NetBoxPowerOutletTemplate` | ⏸️ Deferred | device_type, power_port, name, type, label, description | NetBoxDeviceType | Template for power outlets |
| `/api/dcim/interface-templates/` | `NetBoxInterfaceTemplate` | ⏸️ Deferred | device_type, name, type, mgmt_only, description | NetBoxDeviceType | Template for interfaces |
| `/api/dcim/front-port-templates/` | `NetBoxFrontPortTemplate` | ⏸️ Deferred | device_type, name, type, rear_port, rear_port_position, label, description | NetBoxDeviceType | Template for front ports |
| `/api/dcim/rear-port-templates/` | `NetBoxRearPortTemplate` | ⏸️ Deferred | device_type, name, type, positions, label, description | NetBoxDeviceType | Template for rear ports |
| `/api/dcim/module-bay-templates/` | `NetBoxModuleBayTemplate` | ⏸️ Deferred | device_type, name, label, position, description | NetBoxDeviceType | Template for module bays |
| `/api/dcim/device-bay-templates/` | `NetBoxDeviceBayTemplate` | ⏸️ Deferred | device_type, name, label, description | NetBoxDeviceType | Template for device bays |
| `/api/dcim/inventory-item-templates/` | `NetBoxInventoryItemTemplate` | ⏸️ Deferred | device_type, parent, name, label, role, manufacturer, part_id, description | NetBoxDeviceType, NetBoxInventoryItemRole | Template for inventory items |

### 1.5 Devices/Modules

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/dcim/device-roles/` | `NetBoxDeviceRole` | 📋 Planned | name, slug, color, vm_role, description | None | Device role categorization |
| `/api/dcim/platforms/` | `NetBoxPlatform` | 📋 Planned | name, slug, manufacturer, napalm_driver, napalm_args, description | NetBoxManufacturer | Network OS platforms |
| `/api/dcim/devices/` | `NetBoxDevice` | 📋 Planned | name, device_type, role, tenant, platform, serial, asset_tag, site, location, rack, position, face, status, primary_ip4, primary_ip6, cluster, virtual_chassis, vc_position, vc_priority, description, comments | NetBoxDeviceType, NetBoxDeviceRole, NetBoxSite, NetBoxLocation, NetBoxRack, NetBoxPlatform, NetBoxTenant | **High Priority** - Core device management |
| `/api/dcim/virtual-device-contexts/` | `NetBoxVirtualDeviceContext` | 📋 Planned | device, name, identifier, primary_ip4, primary_ip6, status, description | NetBoxDevice | Virtual device contexts |
| `/api/dcim/modules/` | `NetBoxModule` | 📋 Planned | device, module_bay, module_type, status, serial, asset_tag, description | NetBoxDevice, NetBoxModuleType | Physical modules in devices |

### 1.6 Device Components

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/dcim/console-ports/` | `NetBoxConsolePort` | ⏸️ Deferred | device, name, label, type, speed, description | NetBoxDevice | Console ports on devices |
| `/api/dcim/console-server-ports/` | `NetBoxConsoleServerPort` | ⏸️ Deferred | device, name, label, type, speed, description | NetBoxDevice | Console server ports |
| `/api/dcim/power-ports/` | `NetBoxPowerPort` | ⏸️ Deferred | device, name, label, type, maximum_draw, allocated_draw, description | NetBoxDevice | Power ports |
| `/api/dcim/power-outlets/` | `NetBoxPowerOutlet` | ⏸️ Deferred | device, power_port, name, label, type, feed_leg, description | NetBoxDevice | Power outlets |
| `/api/dcim/interfaces/` | `NetBoxInterface` | 📋 Planned | device, name, type, enabled, parent, bridge, lag, mtu, mac_address, speed, duplex, description | NetBoxDevice | **High Priority** - Network interfaces |
| `/api/dcim/front-ports/` | `NetBoxFrontPort` | ⏸️ Deferred | device, name, type, rear_port, rear_port_position, label, description | NetBoxDevice | Front ports |
| `/api/dcim/rear-ports/` | `NetBoxRearPort` | ⏸️ Deferred | device, name, type, positions, label, description | NetBoxDevice | Rear ports |
| `/api/dcim/module-bays/` | `NetBoxModuleBay` | ⏸️ Deferred | device, name, label, position, description | NetBoxDevice | Module bays |
| `/api/dcim/device-bays/` | `NetBoxDeviceBay` | ⏸️ Deferred | device, name, label, description | NetBoxDevice | Device bays |
| `/api/dcim/inventory-items/` | `NetBoxInventoryItem` | ⏸️ Deferred | device, parent, name, label, role, manufacturer, part_id, serial, asset_tag, discovered, description | NetBoxDevice, NetBoxInventoryItemRole | Inventory items |

### 1.7 Device Component Roles

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/dcim/inventory-item-roles/` | `NetBoxInventoryItemRole` | ⏸️ Deferred | name, slug, color, description | None | Inventory item categorization |

### 1.8 Addressing

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/dcim/mac-addresses/` | `NetBoxMACAddress` | 📋 Planned | interface, address, vdc, description | NetBoxInterface | MAC address management |

### 1.9 Cables

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/dcim/cables/` | `NetBoxCable` | ⏸️ Deferred | type, status, tenant, a_terminations, b_terminations, length, length_unit, label, color, description | NetBoxTenant | Physical cable connections |
| `/api/dcim/cable-terminations/` | `NetBoxCableTermination` | ⏸️ Deferred | cable, termination_type, termination_id, cable_end, description | NetBoxCable | Cable terminations |

### 1.10 Virtual Chassis

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/dcim/virtual-chassis/` | `NetBoxVirtualChassis` | ⏸️ Deferred | name, domain, master, description | NetBoxDevice | Virtual chassis grouping |

### 1.11 Power

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/dcim/power-panels/` | `NetBoxPowerPanel` | ⏸️ Deferred | site, location, name, description | NetBoxSite, NetBoxLocation | Power distribution panels |
| `/api/dcim/power-feeds/` | `NetBoxPowerFeed` | ⏸️ Deferred | power_panel, rack, name, status, type, supply, phase, voltage, amperage, max_utilization, description | NetBoxPowerPanel, NetBoxRack | Power feeds |

---

## 2. ORGANIZATION - Tenancy

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/tenancy/tenant-groups/` | `NetBoxTenantGroup` | 📋 Planned | name, slug, parent (TenantGroup), description | None | Hierarchical tenant organization |
| `/api/tenancy/tenants/` | `NetBoxTenant` | ✅ Implemented | name, slug, group, description, comments | NetBoxTenantGroup | **Currently implemented** |

### 2.1 Contacts

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/tenancy/contact-groups/` | `NetBoxContactGroup` | ⏸️ Deferred | name, slug, parent (ContactGroup), description | None | Contact organization |
| `/api/tenancy/contact-roles/` | `NetBoxContactRole` | ⏸️ Deferred | name, slug, description | None | Contact role categorization |
| `/api/tenancy/contacts/` | `NetBoxContact` | ⏸️ Deferred | name, title, phone, email, address, link, group, description | NetBoxContactGroup | Contact information |
| `/api/tenancy/contact-assignments/` | `NetBoxContactAssignment` | ⏸️ Deferred | content_type, object_id, contact, role, priority, description | NetBoxContact, NetBoxContactRole | Contact assignments to objects |

---

## 3. IPAM (IP Address Management)

### 3.1 ASNs

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/ipam/asns/` | `NetBoxASN` | 📋 Planned | asn, rir, tenant, site, description, comments | NetBoxRIR, NetBoxTenant, NetBoxSite | Autonomous System Numbers |
| `/api/ipam/asn-ranges/` | `NetBoxASNRange` | 📋 Planned | name, slug, rir, start, end, tenant, description | NetBoxRIR, NetBoxTenant | ASN ranges |

### 3.2 VRFs

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/ipam/vrfs/` | `NetBoxVRF` | 📋 Planned | name, rd, enforce_unique, description, tenant, import_targets, export_targets, tags, comments | NetBoxTenant, NetBoxRouteTarget | Virtual Routing and Forwarding |
| `/api/ipam/route-targets/` | `NetBoxRouteTarget` | 📋 Planned | name, tenant, description | NetBoxTenant | BGP route targets |

### 3.3 RIRs & Aggregates

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/ipam/rirs/` | `NetBoxRIR` | 📋 Planned | name, slug, is_private, description | None | Regional Internet Registries |
| `/api/ipam/aggregates/` | `NetBoxAggregate` | ✅ Implemented | prefix, rir, date_allocated, tenant, description, comments | NetBoxRIR, NetBoxTenant | **Currently implemented** |

### 3.4 Roles

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/ipam/roles/` | `NetBoxRole` | ✅ Implemented | name, slug, weight, description, comments | None | **Currently implemented** - IPAM role categorization |

### 3.5 Prefixes & IP Ranges

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/ipam/prefixes/` | `NetBoxPrefix` | ✅ Implemented | prefix, vrf, tenant, site, vlan, status, role, is_pool, mark_utilized, description, tags, comments | NetBoxVRF, NetBoxTenant, NetBoxSite, NetBoxVLAN, NetBoxRole | **Currently implemented** |
| `/api/ipam/ip-ranges/` | `NetBoxIPRange` | 📋 Planned | start_address, end_address, vrf, tenant, status, role, description, tags, comments | NetBoxVRF, NetBoxTenant, NetBoxRole | IP address ranges |

### 3.6 IP Addresses

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/ipam/ip-addresses/` | `NetBoxIPAddress` | 🚧 In Progress | address, vrf, tenant, status, role, assigned_object_type, assigned_object_id, nat_inside, dns_name, description, tags, comments | NetBoxVRF, NetBoxTenant, NetBoxRole | **Partially implemented** via IPClaim/IPPool |

### 3.7 FHRP Groups

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/ipam/fhrp-groups/` | `NetBoxFHRPGroup` | 📋 Planned | protocol, group_id, name, auth_type, auth_key, description, comments | None | First Hop Redundancy Protocol groups |
| `/api/ipam/fhrp-group-assignments/` | `NetBoxFHRPGroupAssignment` | 📋 Planned | group, interface_type, interface_id, priority | NetBoxFHRPGroup, NetBoxInterface | FHRP group assignments |

### 3.8 VLANs

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/ipam/vlan-groups/` | `NetBoxVLANGroup` | 📋 Planned | name, slug, scope_type, scope_id, min_vid, max_vid, description | NetBoxSite | VLAN group organization |
| `/api/ipam/vlans/` | `NetBoxVLAN` | 📋 Planned | site, group, vid, name, status, tenant, role, description, tags, comments | NetBoxSite, NetBoxVLANGroup, NetBoxTenant, NetBoxRole | **High Priority** - VLAN management |
| `/api/ipam/vlan-translation-policies/` | `NetBoxVLANTranslationPolicy` | ⏸️ Deferred | name, slug, description | None | VLAN translation policies |
| `/api/ipam/vlan-translation-rules/` | `NetBoxVLANTranslationRule` | ⏸️ Deferred | policy, inside_vlan, outside_vlan, description | NetBoxVLANTranslationPolicy | VLAN translation rules |

### 3.9 Services

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/ipam/service-templates/` | `NetBoxServiceTemplate` | 📋 Planned | name, protocol, ports, description | None | Service templates |
| `/api/ipam/services/` | `NetBoxService` | 📋 Planned | device, virtual_machine, name, protocol, ports, ipaddresses, description, tags, comments | NetBoxDevice, NetBoxVirtualMachine, NetBoxIPAddress | Network services |

---

## 4. CIRCUITS

### 4.1 Providers

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/circuits/providers/` | `NetBoxProvider` | ⏸️ Deferred | name, slug, asn, account, portal_url, noc_contact, admin_contact, description, comments | NetBoxASN | Circuit providers |
| `/api/circuits/provider-accounts/` | `NetBoxProviderAccount` | ⏸️ Deferred | provider, account, name, description | NetBoxProvider | Provider accounts |
| `/api/circuits/provider-networks/` | `NetBoxProviderNetwork` | ⏸️ Deferred | provider, name, service_id, description, comments | NetBoxProvider | Provider networks |

### 4.2 Circuits

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/circuits/circuit-types/` | `NetBoxCircuitType` | ⏸️ Deferred | name, slug, description | None | Circuit type categorization |
| `/api/circuits/circuits/` | `NetBoxCircuit` | ⏸️ Deferred | cid, provider, type, status, tenant, install_date, termination_date, commit_rate, description, comments | NetBoxProvider, NetBoxCircuitType, NetBoxTenant | WAN circuits |
| `/api/circuits/circuit-terminations/` | `NetBoxCircuitTermination` | ⏸️ Deferred | circuit, term_side, site, provider_network, port_speed, upstream_speed, xconnect_id, pp_info, description | NetBoxCircuit, NetBoxSite | Circuit terminations |
| `/api/circuits/circuit-groups/` | `NetBoxCircuitGroup` | ⏸️ Deferred | name, slug, description | None | Circuit grouping |
| `/api/circuits/circuit-group-assignments/` | `NetBoxCircuitGroupAssignment` | ⏸️ Deferred | circuit, group, description | NetBoxCircuit, NetBoxCircuitGroup | Circuit group assignments |

### 4.3 Virtual Circuits

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/circuits/virtual-circuits/` | `NetBoxVirtualCircuit` | ⏸️ Deferred | name, vcid, status, provider, type, description, comments | NetBoxProvider, NetBoxVirtualCircuitType | Virtual circuits |
| `/api/circuits/virtual-circuit-types/` | `NetBoxVirtualCircuitType` | ⏸️ Deferred | name, slug, description | None | Virtual circuit types |
| `/api/circuits/virtual-circuit-terminations/` | `NetBoxVirtualCircuitTermination` | ⏸️ Deferred | virtual_circuit, term_side, vrf, description | NetBoxVirtualCircuit, NetBoxVRF | Virtual circuit terminations |

---

## 5. VIRTUALIZATION

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/virtualization/cluster-types/` | `NetBoxClusterType` | ⏸️ Deferred | name, slug, description | None | Cluster type categorization |
| `/api/virtualization/cluster-groups/` | `NetBoxClusterGroup` | ⏸️ Deferred | name, slug, description | None | Cluster grouping |
| `/api/virtualization/clusters/` | `NetBoxCluster` | ⏸️ Deferred | name, type, group, status, tenant, site, description, comments | NetBoxClusterType, NetBoxClusterGroup, NetBoxTenant, NetBoxSite | Compute clusters |
| `/api/virtualization/virtual-machines/` | `NetBoxVirtualMachine` | ⏸️ Deferred | name, status, site, cluster, device, role, tenant, platform, primary_ip4, primary_ip6, vcpus, memory, disk, description, comments | NetBoxSite, NetBoxCluster, NetBoxDevice, NetBoxDeviceRole, NetBoxTenant, NetBoxPlatform | Virtual machines |
| `/api/virtualization/interfaces/` | `NetBoxVMInterface` | ⏸️ Deferred | virtual_machine, name, enabled, parent, bridge, mtu, mac_address, description, tags | NetBoxVirtualMachine | VM interfaces |
| `/api/virtualization/virtual-disks/` | `NetBoxVirtualDisk` | ⏸️ Deferred | virtual_machine, name, size, description | NetBoxVirtualMachine | VM virtual disks |

---

## 6. VPN

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/vpn/ike-policies/` | `NetBoxIKEPolicy` | ⏸️ Deferred | name, description, version, mode, proposal, preshared_key | NetBoxIKEProposal | IKE policies |
| `/api/vpn/ike-proposals/` | `NetBoxIKEProposal` | ⏸️ Deferred | name, description, authentication_method, encryption_algorithm, authentication_algorithm, group | None | IKE proposals |
| `/api/vpn/ipsec-policies/` | `NetBoxIPSecPolicy` | ⏸️ Deferred | name, description, proposal, pfs_group | NetBoxIPSecProposal | IPSec policies |
| `/api/vpn/ipsec-proposals/` | `NetBoxIPSecProposal` | ⏸️ Deferred | name, description, encryption_algorithm, authentication_algorithm, sa_lifetime_seconds, sa_lifetime_data | None | IPSec proposals |
| `/api/vpn/ipsec-profiles/` | `NetBoxIPSecProfile` | ⏸️ Deferred | name, description, ike_policy, ipsec_policy | NetBoxIKEPolicy, NetBoxIPSecPolicy | IPSec profiles |
| `/api/vpn/tunnel-groups/` | `NetBoxTunnelGroup` | ⏸️ Deferred | name, slug, description | None | Tunnel grouping |
| `/api/vpn/tunnels/` | `NetBoxTunnel` | ⏸️ Deferred | name, status, group, encapsulation, ipsec_profile, tenant, tunnel_id, description, comments | NetBoxTunnelGroup, NetBoxIPSecProfile, NetBoxTenant | VPN tunnels |
| `/api/vpn/tunnel-terminations/` | `NetBoxTunnelTermination` | ⏸️ Deferred | tunnel, role, termination_type, termination_id, outside_ip, description | NetBoxTunnel | Tunnel terminations |
| `/api/vpn/l2vpns/` | `NetBoxL2VPN` | ⏸️ Deferred | name, slug, type, identifier, import_targets, export_targets, description, comments | NetBoxRouteTarget | L2VPNs |
| `/api/vpn/l2vpn-terminations/` | `NetBoxL2VPNTermination` | ⏸️ Deferred | l2vpn, assigned_object_type, assigned_object_id, description | NetBoxL2VPN | L2VPN terminations |

---

## 7. WIRELESS

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/wireless/wireless-lan-groups/` | `NetBoxWirelessLANGroup` | ⏸️ Deferred | name, slug, parent (WirelessLANGroup), description | None | Wireless LAN grouping |
| `/api/wireless/wireless-lans/` | `NetBoxWirelessLAN` | ⏸️ Deferred | ssid, group, status, vlan, tenant, auth_type, auth_cipher, auth_psk, description, tags, comments | NetBoxWirelessLANGroup, NetBoxVLAN, NetBoxTenant | Wireless LANs |
| `/api/wireless/wireless-links/` | `NetBoxWirelessLink` | ⏸️ Deferred | interface_a, interface_b, ssid, status, tenant, auth_type, auth_cipher, auth_psk, description, tags, comments | NetBoxInterface, NetBoxTenant | Wireless point-to-point links |

---

## 8. EXTRAS (Configuration & Metadata)

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/extras/event-rules/` | `NetBoxEventRule` | ❌ Not Applicable | N/A | N/A | Event processing rules - not suitable for GitOps |
| `/api/extras/webhooks/` | `NetBoxWebhook` | ❌ Not Applicable | N/A | N/A | Webhook configuration - not suitable for GitOps |
| `/api/extras/custom-fields/` | `NetBoxCustomField` | ⏸️ Deferred | type, name, label, description, required, filter_logic, default, weight, validation_minimum, validation_maximum, validation_regex, choices | None | Custom field definitions |
| `/api/extras/custom-field-choice-sets/` | `NetBoxCustomFieldChoiceSet` | ⏸️ Deferred | name, description, extra_choices | None | Custom field choice sets |
| `/api/extras/custom-links/` | `NetBoxCustomLink` | ❌ Not Applicable | N/A | N/A | UI customization - not suitable for GitOps |
| `/api/extras/export-templates/` | `NetBoxExportTemplate` | ❌ Not Applicable | N/A | N/A | Export templates - not suitable for GitOps |
| `/api/extras/saved-filters/` | `NetBoxSavedFilter` | ❌ Not Applicable | N/A | N/A | User-specific filters - not suitable for GitOps |
| `/api/extras/table-configs/` | `NetBoxTableConfig` | ❌ Not Applicable | N/A | N/A | User-specific table configs - not suitable for GitOps |
| `/api/extras/bookmarks/` | `NetBoxBookmark` | ❌ Not Applicable | N/A | N/A | User-specific bookmarks - not suitable for GitOps |
| `/api/extras/notifications/` | `NetBoxNotification` | ❌ Not Applicable | N/A | N/A | User notifications - not suitable for GitOps |
| `/api/extras/notification-groups/` | `NetBoxNotificationGroup` | ❌ Not Applicable | N/A | N/A | User notification groups - not suitable for GitOps |
| `/api/extras/subscriptions/` | `NetBoxSubscription` | ❌ Not Applicable | N/A | N/A | User subscriptions - not suitable for GitOps |
| `/api/extras/tags/` | `NetBoxTag` | ✅ Implemented | name, slug, color, description, comments | None | **Currently implemented** |
| `/api/extras/tagged-objects/` | `NetBoxTaggedObject` | ❌ Not Applicable | N/A | N/A | Tag assignments - handled via parent objects |
| `/api/extras/image-attachments/` | `NetBoxImageAttachment` | ⏸️ Deferred | content_type, object_id, image, name | None | Image attachments |
| `/api/extras/journal-entries/` | `NetBoxJournalEntry` | ❌ Not Applicable | N/A | N/A | Change logs - read-only, not suitable for GitOps |
| `/api/extras/config-contexts/` | `NetBoxConfigContext` | 📋 Planned | name, weight, description, data, is_active | None | Configuration contexts for devices/VMs |
| `/api/extras/config-context-profiles/` | `NetBoxConfigContextProfile` | 📋 Planned | name, description, config_contexts | NetBoxConfigContext | Config context profiles |
| `/api/extras/config-templates/` | `NetBoxConfigTemplate` | 📋 Planned | name, description, template_code, environment_params, data_source | None | Configuration templates |
| `/api/extras/scripts/` | `NetBoxScript` | ❌ Not Applicable | N/A | N/A | Custom scripts - not suitable for GitOps |

---

## 9. CORE (System)

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/core/data-sources/` | `NetBoxDataSource` | ❌ Not Applicable | N/A | N/A | Data source configuration - system-level |
| `/api/core/data-files/` | `NetBoxDataFile` | ❌ Not Applicable | N/A | N/A | Data file management - system-level |
| `/api/core/jobs/` | `NetBoxJob` | ❌ Not Applicable | N/A | N/A | Background jobs - read-only |
| `/api/core/object-changes/` | `NetBoxObjectChange` | ❌ Not Applicable | N/A | N/A | Change log - read-only |
| `/api/core/object-types/` | `NetBoxObjectType` | ❌ Not Applicable | N/A | N/A | Object type registry - read-only |
| `/api/core/background-queues/` | `NetBoxBackgroundQueue` | ❌ Not Applicable | N/A | N/A | Background queue status - read-only |
| `/api/core/background-workers/` | `NetBoxBackgroundWorker` | ❌ Not Applicable | N/A | N/A | Background worker status - read-only |
| `/api/core/background-tasks/` | `NetBoxBackgroundTask` | ❌ Not Applicable | N/A | N/A | Background task status - read-only |

---

## 10. USERS

| API Endpoint | CRD Name | Status | Key Fields | Dependencies | Notes |
|-------------|----------|--------|------------|--------------|-------|
| `/api/users/users/` | `NetBoxUser` | ❌ Not Applicable | N/A | N/A | User management - not suitable for GitOps |
| `/api/users/groups/` | `NetBoxGroup` | ❌ Not Applicable | N/A | N/A | User group management - not suitable for GitOps |
| `/api/users/tokens/` | `NetBoxToken` | ❌ Not Applicable | N/A | N/A | API token management - not suitable for GitOps |
| `/api/users/permissions/` | `NetBoxObjectPermission` | ❌ Not Applicable | N/A | N/A | Permission management - not suitable for GitOps |
| `/api/users/config/` | `NetBoxUserConfig` | ❌ Not Applicable | N/A | N/A | User preferences - not suitable for GitOps |

---

## Implementation Priority

### Phase 1: Core IPAM (✅ Complete)
- ✅ NetBoxPrefix
- ✅ NetBoxAggregate
- ✅ NetBoxRole
- ✅ NetBoxTenant
- ✅ NetBoxSite
- ✅ NetBoxTag
- ✅ IPPool (custom CRD)
- ✅ IPClaim (custom CRD)

### Phase 2: Essential IPAM (📋 Next)
- 📋 NetBoxVRF
- 📋 NetBoxVLAN
- 📋 NetBoxVLANGroup
- 📋 NetBoxIPRange
- 📋 NetBoxASN
- 📋 NetBoxASNRange
- 📋 NetBoxRIR
- 📋 NetBoxRouteTarget

### Phase 3: Device Management (📋 Planned)
- 📋 NetBoxDevice
- 📋 NetBoxInterface
- 📋 NetBoxDeviceRole
- 📋 NetBoxPlatform
- 📋 NetBoxManufacturer
- 📋 NetBoxDeviceType
- 📋 NetBoxMACAddress

### Phase 4: Site & Rack Management (📋 Planned)
- 📋 NetBoxRegion
- 📋 NetBoxSiteGroup
- 📋 NetBoxLocation
- 📋 NetBoxRack
- 📋 NetBoxRackType
- 📋 NetBoxRackRole

### Phase 5: Services & Advanced (📋 Planned)
- 📋 NetBoxService
- 📋 NetBoxServiceTemplate
- 📋 NetBoxFHRPGroup
- 📋 NetBoxConfigContext
- 📋 NetBoxConfigContextProfile
- 📋 NetBoxConfigTemplate

### Phase 6: Lower Priority (⏸️ Deferred)
- All other endpoints marked as "Deferred"

---

## Notes

1. **Custom CRDs**: `IPPool` and `IPClaim` are custom CRDs that don't map directly to NetBox API endpoints but provide higher-level abstractions for IP address pool management.

2. **Read-Only Resources**: Many resources (jobs, object-changes, etc.) are read-only and not suitable for GitOps management.

3. **User-Specific Resources**: Resources like bookmarks, saved filters, and user preferences are user-specific and not suitable for GitOps.

4. **System Resources**: Core system resources (data sources, background queues, etc.) are typically managed outside of GitOps workflows.

5. **Dependencies**: Many CRDs have dependencies on other CRDs. The controller should handle reference resolution (e.g., NetBoxPrefix references NetBoxSite, NetBoxTenant, etc.).

---

## Next Steps

1. **Complete Phase 2** - Essential IPAM resources (VRF, VLAN, etc.)
2. **Add Reference Resolution** - Improve handling of CRD-to-CRD references
3. **Add Validation** - Ensure CRD specs are valid before reconciliation
4. **Add Status Reporting** - Better status updates for all resources
5. **Add Documentation** - Generate CRD documentation from schemas

