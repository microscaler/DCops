# Video Transcript: Site Management with DCops

**Duration:** ~10 minutes  
**Target Audience:** Infrastructure Engineers, Datacenter Managers, SREs  
**Prerequisites:** DCops installed, basic understanding of infrastructure hierarchy

---

## Introduction (0:00 - 0:30)

[Screen: DCops logo]

**Narrator:** "Welcome to Site Management with DCops. In this video, we'll learn how to organize your physical infrastructure using DCops' hierarchical site structure - from regions down to individual racks."

[Screen: Infrastructure hierarchy]

**Narrator:** "DCops uses a flexible hierarchy to organize your infrastructure: Regions contain Sites, Sites contain Locations, and Locations can be nested. This gives you the flexibility to model any datacenter structure."

---

## What You'll Learn (0:30 - 1:00)

[Screen: Learning objectives]

**Narrator:** "In this walkthrough, we'll:
1. Understand the site hierarchy
2. Create a region
3. Create a site
4. Create nested locations
5. Organize infrastructure by geography and function
6. Learn best practices

Let's get started!"

---

## Site Hierarchy Overview (1:00 - 2:00)

[Screen: Hierarchy diagram]

**Narrator:** "DCops supports a flexible hierarchy for organizing infrastructure:

At the top level, we have **Regions** - these represent geographic areas like 'US East' or 'Europe'.

Within regions, we have **Sites** - these are physical locations like datacenters or colocation facilities.

Within sites, we have **Locations** - these can be nested: buildings, floors, rooms, and racks.

You can also use **Site Groups** as an alternative to regions - for example, organizing by environment like 'Production Sites' or 'Staging Sites'."

[Screen: Animated diagram showing hierarchy]

**Narrator:** "Here's how it works: A region contains sites. A site contains locations. Locations can be nested - so you can have Building → Floor → Room → Rack. This gives you complete flexibility to model your actual infrastructure."

---

## Step 1: Create a Region (2:00 - 3:30)

[Screen: Creating region]

**Narrator:** "Let's start by creating a region. Regions help organize sites geographically."

[Screen: Region YAML]

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

**Narrator:** "Notice we're using a slug - this is a URL-friendly version of the name. The controller will auto-generate a slug if you don't provide one."

[Screen: Applying region]

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/netbox-region-example.yaml
```

**Narrator:** "Let's verify it was created..."

```bash
kubectl get netboxregion us-east -o yaml
```

**Narrator:** "Perfect! The region was created. You can see it has a netboxId, which means it exists in NetBox."

[Screen: Hierarchical regions]

**Narrator:** "You can also create hierarchical regions. For example, you might have 'North America' as a parent region, with 'US East' and 'US West' as child regions. This is useful for large organizations with multiple geographic areas."

---

## Step 2: Create a Site Group (3:30 - 4:30)

[Screen: Creating site group]

**Narrator:** "Site groups provide an alternative way to organize sites - by function or environment rather than geography."

[Screen: Site Group YAML]

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

[Screen: Applying site group]

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/netbox-site-group-example.yaml
```

**Narrator:** "Site groups are useful when you want to organize by environment - production, staging, development - rather than by geography."

---

## Step 3: Create a Site (4:30 - 6:00)

[Screen: Creating site]

**Narrator:** "Now let's create a site. A site represents a physical location - a datacenter, colocation facility, or even a single cabinet."

[Screen: Site YAML]

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

**Narrator:** "Notice we're referencing both a region and a site group. This gives us flexibility - we can filter by geography or by environment. We've also included geographic coordinates and a physical address, which is useful for mapping and documentation."

[Screen: Applying site]

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/netbox-site-example.yaml
```

**Narrator:** "Let's check the status..."

```bash
kubectl get netboxsite datacenter-1 -o yaml
```

**Narrator:** "Excellent! The site was created. You can see it's linked to both the region and site group, and it has geographic coordinates."

---

## Step 4: Create Nested Locations (6:00 - 8:00)

[Screen: Creating locations]

**Narrator:** "Now let's create locations within the site. Locations can be nested, so we can model buildings, floors, rooms, and racks."

[Screen: Building location]

**Narrator:** "First, let's create a building..."

```yaml
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
  tenant:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxTenant"
    name: "datacenter-tenant"
```

[Screen: Floor location]

**Narrator:** "Then a floor, with the building as its parent..."

```yaml
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
  tenant:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxTenant"
    name: "datacenter-tenant"
```

[Screen: Room location]

**Narrator:** "Then a room, with the floor as its parent..."

```yaml
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
  tenant:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxTenant"
    name: "datacenter-tenant"
```

[Screen: Rack location]

**Narrator:** "And finally, a rack, with the room as its parent..."

```yaml
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
  tenant:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxTenant"
    name: "datacenter-tenant"
```

[Screen: Applying all locations]

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/netbox-location-example.yaml
```

**Narrator:** "Let's verify the hierarchy..."

```bash
kubectl get netboxlocation -o custom-columns=NAME:.metadata.name,SITE:.spec.site.name,PARENT:.spec.parent.name
```

**Narrator:** "Perfect! You can see the nested structure - the rack has the room as its parent, the room has the floor as its parent, and so on."

---

## Organizing by Geography (8:00 - 9:00)

[Screen: Multiple regions example]

**Narrator:** "Let's look at organizing by geography. You might have multiple regions..."

[Screen: Region structure]

**Narrator:** "US East region with datacenter-1 and datacenter-2. US West region with datacenter-3. Europe region with datacenter-4.

Each site references its region, so you can easily filter and organize by geography. This is perfect for multi-region deployments."

---

## Organizing by Function (9:00 - 9:30)

[Screen: Site groups example]

**Narrator:** "Or organize by function using site groups:

Production sites group with datacenter-1 and datacenter-2. Staging sites group with staging-dc-1.

This gives you flexibility to organize however makes sense for your organization."

---

## Best Practices (9:30 - 10:30)

[Screen: Best practices slide]

**Narrator:** "Here are some best practices for site management..."

### 1. Consistent Naming

**Narrator:** "Use consistent naming conventions:
- Good: `us-east`, `datacenter-1`, `rack-a`
- Avoid: `US_East`, `DC1`, `RackA`

Consistency makes it easier to find and manage resources."

### 2. Use Slugs

**Narrator:** "Always provide slugs for URL-friendly names. The controller will auto-generate if you don't, but it's better to be explicit."

### 3. Add Descriptions

**Narrator:** "Document the purpose of each site and location. This helps team members understand the infrastructure."

### 4. Include Geographic Data

**Narrator:** "Add coordinates and addresses. This is useful for:
- Mapping tools
- Documentation
- Emergency planning
- Compliance requirements"

### 5. Use Status Fields

**Narrator:** "Track site lifecycle with status:
- `active` - Currently in use
- `planned` - Planned but not yet built
- `retired` - No longer in use
- `staging` - Staging environment"

---

## Summary (10:30 - 11:00)

[Screen: Summary slide]

**Narrator:** "In this video, we learned:
- How to create regions and sites
- How to use nested locations
- How to organize by geography or function
- Best practices for site management

DCops gives you the flexibility to model your infrastructure exactly as it exists in the real world, all managed through Git."

[Screen: End screen]

**Narrator:** "Thanks for watching! Check out the documentation for more examples and advanced topics."

---

## Production Notes

### Visual Elements:
1. Hierarchy diagrams (animated)
2. Terminal screen recordings
3. NetBox UI showing site structure
4. Geographic map view (if possible)
5. Git repository showing YAML structure

### Key Moments:
- Hierarchy explanation (use animation)
- Nested location creation (show parent-child relationships)
- Best practices (pause for emphasis)

### Voiceover Tips:
- Speak clearly when explaining hierarchy
- Use examples from real datacenters
- Emphasize flexibility of the system
- Pause before best practices

