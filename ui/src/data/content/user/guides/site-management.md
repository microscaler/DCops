# Site Management

Manage physical locations (datacenters, colocation facilities, cabinets) with DCops.

## Overview

DCops uses a hierarchical structure for organizing infrastructure:

```
Region
  └── Site Group (optional)
      └── Site
          └── Location (optional, nested)
              └── Device
```

## Creating a Site Hierarchy

### Step 1: Create Region

Regions represent geographic areas:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxRegion
metadata:
  name: us-east
  namespace: default
spec:
  name: "US East"
  slug: "us-east"
  description: "US East region for datacenter operations"
```

**Optional:** Create hierarchical regions:

```yaml
spec:
  name: "US East"
  parent:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxRegion"
    name: "north-america"
```

Apply it:

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/netbox-region-example.yaml
```

### Step 2: Create Site Group (Optional)

Site groups provide an alternative to regions for organizing sites:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxSiteGroup
metadata:
  name: production-sites
  namespace: default
spec:
  name: "Production Sites"
  slug: "production-sites"
  description: "Production datacenter sites"
```

Apply it:

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/netbox-site-group-example.yaml
```

### Step 3: Create Site

Sites represent physical locations (datacenters, colocation facilities):

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
  siteGroup:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxSiteGroup"
    name: "production-sites"
  facility: "DC1"
  timeZone: "UTC"
  physicalAddress: "123 Main St, City, State 12345"
  latitude: 40.7128
  longitude: -74.0060
```

**Required Fields:**
- `name` - Site name
- `tenant` - Tenant reference (required)

**Optional Fields:**
- `region` - Region reference
- `siteGroup` - Site group reference
- `status` - Site status (active, planned, retired, staging)
- `facility` - Facility code
- `timeZone` - Time zone
- `physicalAddress` - Physical address
- `latitude` / `longitude` - Geographic coordinates

Apply it:

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/netbox-site-example.yaml
```

### Step 4: Create Location (Optional)

Locations represent nested locations within a site (building, floor, room, rack):

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxLocation
metadata:
  name: datacenter-1-rack-a
  namespace: default
spec:
  name: "Rack A"
  slug: "rack-a"
  site:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxSite"
    name: "datacenter-1"
  tenant:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxTenant"
    name: "datacenter-tenant"
  description: "Rack A in datacenter-1"
  status: active
```

**Nested Locations:**

You can create hierarchical locations:

```yaml
# Building
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxLocation
metadata:
  name: datacenter-1-building-1
spec:
  name: "Building 1"
  site:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxSite"
    name: "datacenter-1"
---
# Floor (parent: Building)
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxLocation
metadata:
  name: datacenter-1-floor-1
spec:
  name: "Floor 1"
  site:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxSite"
    name: "datacenter-1"
  parent:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxLocation"
    name: "datacenter-1-building-1"
---
# Room (parent: Floor)
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxLocation
metadata:
  name: datacenter-1-room-101
spec:
  name: "Room 101"
  site:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxSite"
    name: "datacenter-1"
  parent:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxLocation"
    name: "datacenter-1-floor-1"
---
# Rack (parent: Room)
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxLocation
metadata:
  name: datacenter-1-rack-a
spec:
  name: "Rack A"
  site:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxSite"
    name: "datacenter-1"
  parent:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxLocation"
    name: "datacenter-1-room-101"
```

Apply it:

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/netbox-location-example.yaml
```

## Organizing Infrastructure

### By Region

Organize sites by geographic region:

```yaml
# US East Region
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxRegion
metadata:
  name: us-east
spec:
  name: "US East"
---
# US West Region
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxRegion
metadata:
  name: us-west
spec:
  name: "US West"
---
# Site in US East
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxSite
metadata:
  name: datacenter-1
spec:
  name: "Data Center 1"
  region:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxRegion"
    name: "us-east"
```

### By Site Group

Organize sites by function or environment:

```yaml
# Production Sites
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxSiteGroup
metadata:
  name: production-sites
spec:
  name: "Production Sites"
---
# Staging Sites
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxSiteGroup
metadata:
  name: staging-sites
spec:
  name: "Staging Sites"
---
# Site in Production
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxSite
metadata:
  name: datacenter-1
spec:
  name: "Data Center 1"
  siteGroup:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxSiteGroup"
    name: "production-sites"
```

## Multi-Tenant Support

DCops supports multiple tenants with clear isolation:

### Tenant-Specific Sites

Each tenant can have their own sites:

```yaml
# Tenant A Site
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxSite
metadata:
  name: tenant-a-datacenter
spec:
  name: "Tenant A Datacenter"
  tenant:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxTenant"
    name: "tenant-a"
---
# Tenant B Site
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxSite
metadata:
  name: tenant-b-datacenter
spec:
  name: "Tenant B Datacenter"
  tenant:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxTenant"
    name: "tenant-b"
```

### Shared Resources

Platform-level resources (manufacturer, device type, etc.) can be shared:

```yaml
# Shared platform resource (no tenant)
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxManufacturer
metadata:
  name: raspberry-pi
spec:
  name: "Raspberry Pi"
```

## Best Practices

### 1. Consistent Naming

Use consistent naming conventions:

```yaml
# Good
name: us-east
name: datacenter-1
name: rack-a

# Avoid
name: US_East
name: DC1
name: RackA
```

### 2. Use Slugs

Always provide slugs for better URL handling:

```yaml
spec:
  name: "Data Center 1"
  slug: "datacenter-1"  # URL-friendly version
```

### 3. Add Descriptions

Document the purpose of each site:

```yaml
spec:
  name: "Data Center 1"
  description: "Primary datacenter facility for production workloads"
```

### 4. Include Geographic Data

Add coordinates for mapping:

```yaml
spec:
  latitude: 40.7128
  longitude: -74.0060
  physicalAddress: "123 Main St, City, State 12345"
```

### 5. Use Status Fields

Track site lifecycle:

```yaml
spec:
  status: active      # active, planned, retired, staging
```

## Verification

### Check Site Status

```bash
kubectl get netboxsite datacenter-1 -o yaml
```

Look for:
- `status.state: Created`
- `status.netboxId: <number>`
- `status.netboxUrl: http://netbox/api/dcim/sites/<id>/`

### Verify in NetBox UI

1. Log into NetBox UI
2. Navigate to **DCIM → Sites**
3. You should see your site listed
4. Click on it to see details and hierarchy

### Check Hierarchy

View the complete hierarchy:

```bash
# Region
kubectl get netboxregion us-east

# Site
kubectl get netboxsite datacenter-1

# Location
kubectl get netboxlocation datacenter-1-rack-a
```

## Troubleshooting

### Site Creation Fails

**Issue:** Site shows `state: Failed`

**Check:**
1. Tenant exists and is ready:
   ```bash
   kubectl get netboxtenant datacenter-tenant
   ```
2. Region exists (if specified):
   ```bash
   kubectl get netboxregion us-east
   ```
3. Controller logs:
   ```bash
   kubectl logs -n dcops-system deployment/netbox-controller | grep -i site
   ```

### Location Parent Not Found

**Issue:** Location creation fails with "parent not found"

**Solutions:**
- Create parent location first
- Verify parent location name matches reference
- Check parent location is in same site

## Example: Complete Hierarchy

Here's a complete example for a datacenter:

```yaml
# 1. Region
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxRegion
metadata:
  name: us-east
spec:
  name: "US East"
---
# 2. Site
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxSite
metadata:
  name: datacenter-1
spec:
  name: "Data Center 1"
  tenant:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxTenant"
    name: "datacenter-tenant"
  region:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxRegion"
    name: "us-east"
---
# 3. Location (Rack)
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxLocation
metadata:
  name: datacenter-1-rack-a
spec:
  name: "Rack A"
  site:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxSite"
    name: "datacenter-1"
  tenant:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxTenant"
    name: "datacenter-tenant"
```

## Next Steps

- [IP Pool Management](./ip-pool-management.md) - Allocate IP addresses for sites
- [Infrastructure Inventory](../concepts/infrastructure-inventory.md) - Learn about device management
- [CRD Reference](../api-reference/crd-reference.md) - Complete CRD documentation
