# NetBox Setup

NetBox is the authoritative source for IPAM and inventory in DCops.

## Requirements

- NetBox 4.0 or later
- API token with appropriate permissions
- Network prefixes configured

## Configuration

DCops connects to NetBox via API:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: netbox-config
data:
  url: https://netbox.example.com
  apiTokenSecret: netbox-credentials
```

## Permissions

The NetBox API token needs:
- Read/Write access to IPAM
- Read/Write access to DCIM
- Tenant management permissions

