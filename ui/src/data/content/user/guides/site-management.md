# Site Management

Manage physical locations with DCops.

## Creating Sites

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: Site
metadata:
  name: datacenter-1
spec:
  name: "Primary Datacenter"
  region: "us-east"
  netboxRef:
    siteId: 789
```

## Site Hierarchy

Sites can be organized by:
- Region
- Datacenter
- Colocation facility
- Cabinet

## Multi-Tenant Support

DCops supports multiple tenants:
- Tenant isolation
- Shared resource management
- Clear ownership tracking

