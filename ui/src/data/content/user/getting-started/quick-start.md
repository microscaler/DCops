# Quick Start

Get DCops up and running in minutes.

## Step 1: Install Controllers

```bash
kubectl apply -f config/crd/all-crds.yaml
kubectl apply -f config/netbox-controller/
```

## Step 2: Configure NetBox

Create a Secret with your NetBox credentials:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: netbox-credentials
  namespace: dcops-system
type: Opaque
stringData:
  url: https://netbox.example.com
  token: your-netbox-api-token
```

## Step 3: Create Your First IP Pool

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPPool
metadata:
  name: example-pool
spec:
  prefix: 192.168.1.0/24
  netboxRef:
    prefixId: 123
```

## Next Steps

- Learn about [IP Address Allocation](../concepts/ip-allocation.md)
- Explore [Infrastructure Inventory](../concepts/infrastructure-inventory.md)

