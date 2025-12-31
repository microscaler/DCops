# CRD Reference

Complete reference for all Custom Resource Definitions in DCops.

## IPPool

Manages a pool of IP addresses.

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPPool
metadata:
  name: example
spec:
  prefix: 192.168.1.0/24
  netboxRef:
    prefixId: 123
```

## IPClaim

Requests an IP address from a pool.

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPClaim
metadata:
  name: example
spec:
  poolRef:
    name: example-pool
  address: 192.168.1.10
```

## Site

Represents a physical location.

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: Site
metadata:
  name: example
spec:
  name: "Example Site"
  netboxRef:
    siteId: 456
```

