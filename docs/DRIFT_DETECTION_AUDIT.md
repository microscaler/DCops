# Drift Detection Implementation Audit

This document tracks the implementation of drift detection across all NetBox CRDs and reconcilers.

## Overview

Drift detection ensures that any changes made to resources in the NetBox UI are automatically corrected to match the Git CRD spec (Git is the source of truth).

### Implementation Checklist

For each CRD, we need to:
- [ ] Add `driftDetection: Option<bool>` field to CRD spec (defaults to `true`)
- [ ] Create `*_needs_update()` function comparing CRD spec with NetBox resource
- [ ] Update reconciler to check drift when `UseExisting` is returned
- [ ] Emit `DRIFT_DETECTED` event when correcting field drift
- [ ] Verify resource exists in NetBox via curl

---

## DCIM Resources

### NetBoxPlatform
- [x] `driftDetection` field added to CRD spec
- [x] `platform_needs_update()` function created
- [x] Reconciler updated with drift detection
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
# Get NetBox token from secret
NETBOX_TOKEN=$(kubectl get secret datacenter-tenant-netbox-token -n default -o jsonpath='{.data.token}' | base64 -d)
NETBOX_URL="http://netbox.netbox"

# Query platform by name
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/dcim/platforms/?name=talos-linux" | jq '.results[] | {id, name, slug, manufacturer, napalm_driver, description, comments}'
```

---

### NetBoxManufacturer
- [x] `driftDetection` field added to CRD spec
- [x] `manufacturer_needs_update()` function created
- [x] Reconciler updated with drift detection
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/dcim/manufacturers/?name=Raspberry%20Pi" | jq '.results[] | {id, name, slug, description, comments}'
```

---

### NetBoxDeviceType
- [ ] `driftDetection` field added to CRD spec
- [ ] `device_type_needs_update()` function created
- [ ] Reconciler updated with drift detection
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/dcim/device-types/?model=Pi%204" | jq '.results[] | {id, manufacturer, model, slug, part_number, u_height, description, comments}'
```

---

### NetBoxDeviceRole
- [ ] `driftDetection` field added to CRD spec
- [ ] `device_role_needs_update()` function created
- [ ] Reconciler updated with drift detection
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/dcim/device-roles/?name=control-plane" | jq '.results[] | {id, name, slug, color, description, comments}'
```

---

### NetBoxRegion
- [ ] `driftDetection` field added to CRD spec
- [ ] `region_needs_update()` function created
- [ ] Reconciler updated with drift detection
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/dcim/regions/?name=US-West" | jq '.results[] | {id, name, slug, parent, description, comments}'
```

---

### NetBoxSiteGroup
- [ ] `driftDetection` field added to CRD spec
- [ ] `site_group_needs_update()` function created
- [ ] Reconciler updated with drift detection
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/dcim/site-groups/?name=Production%20Sites" | jq '.results[] | {id, name, slug, parent, description, comments}'
```

---

### NetBoxLocation
- [ ] `driftDetection` field added to CRD spec
- [ ] `location_needs_update()` function created
- [ ] Reconciler updated with drift detection
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/dcim/locations/?name=Rack%20A" | jq '.results[] | {id, name, slug, site, parent, description, comments}'
```

---

### NetBoxSite
- [ ] `driftDetection` field added to CRD spec
- [ ] `site_needs_update()` function created (already exists, verify)
- [ ] Reconciler updated with drift detection (already has drift detection, verify)
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/dcim/sites/?name=datacenter-1" | jq '.results[] | {id, name, slug, status, tenant, region, site_group, description, physical_address, comments}'
```

---

### NetBoxVLAN
- [ ] `driftDetection` field added to CRD spec
- [ ] `vlan_needs_update()` function created
- [ ] Reconciler updated with drift detection
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/ipam/vlans/?vid=100" | jq '.results[] | {id, vid, name, site, tenant, role, description, comments}'
```

---

### NetBoxInterface
- [ ] `driftDetection` field added to CRD spec
- [ ] `interface_needs_update()` function created
- [ ] Reconciler updated with drift detection
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/dcim/interfaces/?name=eth0&device=talos-control-plane-01" | jq '.results[] | {id, device, name, type, enabled, mac_address, mtu, description, comments}'
```

---

### NetBoxDevice
- [ ] `driftDetection` field added to CRD spec
- [ ] `device_needs_update()` function created
- [ ] Reconciler updated with drift detection
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/dcim/devices/?name=talos-control-plane-01" | jq '.results[] | {id, name, device_type, device_role, site, tenant, platform, serial, asset_tag, status, description, comments}'
```

---

### NetBoxMACAddress
- [ ] `driftDetection` field added to CRD spec
- [ ] `mac_address_needs_update()` function created
- [ ] Reconciler updated with drift detection
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/dcim/mac-addresses/?mac_address=aa:bb:cc:dd:ee:ff" | jq '.results[] | {id, mac_address, assigned_object_type, assigned_object_id, description, comments}'
```

---

## IPAM Resources

### NetBoxPrefix
- [ ] `driftDetection` field added to CRD spec
- [ ] `prefix_needs_update()` function created (already exists, verify)
- [ ] Reconciler updated with drift detection (already has drift detection, verify)
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/ipam/prefixes/?prefix=192.168.1.0%2F24" | jq '.results[] | {id, prefix, status, tenant, site, vlan, role, description, comments}'
```

---

### NetBoxIPRange
- [ ] `driftDetection` field added to CRD spec
- [ ] `ip_range_needs_update()` function created (already exists, verify)
- [ ] Reconciler updated with drift detection (already has drift detection, verify)
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/ipam/ip-ranges/?start_address=192.168.1.100" | jq '.results[] | {id, start_address, end_address, status, tenant, vrf, role, description, comments}'
```

---

### NetBoxIPAddress
- [ ] `driftDetection` field added to CRD spec
- [ ] `ip_address_needs_update()` function created (already exists, verify)
- [ ] Reconciler updated with drift detection (already has drift detection, verify)
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/ipam/ip-addresses/?address=192.168.1.1%2F24" | jq '.results[] | {id, address, status, tenant, assigned_object_type, assigned_object_id, description, comments}'
```

---

### NetBoxAggregate
- [ ] `driftDetection` field added to CRD spec
- [ ] `aggregate_needs_update()` function created
- [ ] Reconciler updated with drift detection
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/ipam/aggregates/?prefix=10.0.0.0%2F8" | jq '.results[] | {id, prefix, rir, tenant, description, comments}'
```

---

### NetBoxRole (IPAM)
- [ ] `driftDetection` field added to CRD spec
- [ ] `role_needs_update()` function created
- [ ] Reconciler updated with drift detection
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/ipam/roles/?name=dhcp-pool" | jq '.results[] | {id, name, slug, weight, description, comments}'
```

---

### NetBoxRIR
- [ ] `driftDetection` field added to CRD spec
- [ ] `rir_needs_update()` function created
- [ ] Reconciler updated with drift detection
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/ipam/rirs/?name=ARIN" | jq '.results[] | {id, name, slug, description, comments}'
```

---

### NetBoxVRF
- [ ] `driftDetection` field added to CRD spec
- [ ] `vrf_needs_update()` function created (already exists, verify)
- [ ] Reconciler updated with drift detection (already has drift detection, verify)
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/ipam/vrfs/?name=production-vrf" | jq '.results[] | {id, name, rd, enforce_unique, tenant, import_targets, export_targets, description, comments}'
```

---

### NetBoxRouteTarget
- [ ] `driftDetection` field added to CRD spec
- [ ] `route_target_needs_update()` function created
- [ ] Reconciler updated with drift detection
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/ipam/route-targets/?name=65000:100" | jq '.results[] | {id, name, tenant, description, comments}'
```

---

## Tenancy Resources

### NetBoxTenant
- [ ] `driftDetection` field added to CRD spec
- [ ] `tenant_needs_update()` function created
- [ ] Reconciler updated with drift detection
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/tenancy/tenants/?name=datacenter-tenant" | jq '.results[] | {id, name, slug, group, description, comments}'
```

---

### NetBoxTenantGroup
- [ ] `driftDetection` field added to CRD spec
- [ ] `tenant_group_needs_update()` function created
- [ ] Reconciler created (currently missing)
- [ ] Reconciler updated with drift detection
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/tenancy/tenant-groups/?name=Default" | jq '.results[] | {id, name, slug, parent, description, comments}'
```

---

## Extras Resources

### NetBoxTag
- [ ] `driftDetection` field added to CRD spec
- [ ] `tag_needs_update()` function created
- [ ] Reconciler updated with drift detection
- [ ] Verified in NetBox via curl

**Curl Verification:**
```bash
curl -H "Authorization: Token $NETBOX_TOKEN" \
     -H "Accept: application/json" \
     "$NETBOX_URL/api/extras/tags/?name=managed-by-dcops" | jq '.results[] | {id, name, slug, color, description, comments}'
```

---

## Summary

### Progress Tracking

**Total CRDs:** 22
**Completed:** 2 (NetBoxPlatform, NetBoxManufacturer)
**In Progress:** 0
**Remaining:** 20

### Completion Status

| Resource | CRD Field | Needs Update Function | Reconciler | Verified |
|----------|-----------|----------------------|------------|----------|
| NetBoxPlatform | ✅ | ✅ | ✅ | ⏳ |
| NetBoxManufacturer | ✅ | ✅ | ✅ | ⏳ |
| NetBoxDeviceType | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxDeviceRole | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxRegion | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxSiteGroup | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxLocation | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxSite | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxVLAN | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxInterface | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxDevice | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxMACAddress | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxPrefix | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxIPRange | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxIPAddress | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxAggregate | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxRole (IPAM) | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxRIR | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxVRF | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxRouteTarget | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxTenant | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxTenantGroup | ⏳ | ⏳ | ⏳ | ⏳ |
| NetBoxTag | ⏳ | ⏳ | ⏳ | ⏳ |

Legend: ✅ Complete | ⏳ Pending

### Notes

- Resources marked with "(already exists, verify)" need to be checked to ensure they properly implement drift detection with the `driftDetection` flag
- NetBoxTenantGroup reconciler is missing and needs to be created
- All curl commands assume NetBox is accessible at `http://netbox.netbox` and the token is stored in a Kubernetes secret

### Testing Drift Detection

To test drift detection:
1. Apply a CR to create a resource in NetBox
2. Manually modify the resource in NetBox UI (change name, description, etc.)
3. Wait for reconciliation (or trigger manually)
4. Verify the resource in NetBox matches the CRD spec again
5. Check controller logs for `DRIFT_DETECTED` events
