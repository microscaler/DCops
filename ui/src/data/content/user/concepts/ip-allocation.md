# IP Address Allocation

DCops provides deterministic IP address allocation through NetBox IPAM.

## IP Pools

An IP Pool represents a range of IP addresses managed by DCops:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPPool
metadata:
  name: production-pool
spec:
  prefix: 10.0.0.0/24
  netboxRef:
    prefixId: 456
```

## IP Claims

An IP Claim requests a specific IP address from a pool:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPClaim
metadata:
  name: web-server-ip
spec:
  poolRef:
    name: production-pool
  address: 10.0.0.10
```

## Automatic Allocation

DCops automatically:
- Allocates IPs from NetBox
- Prevents conflicts
- Tracks allocation state
- Detects drift

