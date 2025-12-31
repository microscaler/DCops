# IP Pool Management

Learn how to manage IP pools in DCops.

## Creating IP Pools

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPPool
metadata:
  name: my-pool
spec:
  prefix: 192.168.1.0/24
  netboxRef:
    prefixId: 123
```

## Managing IP Claims

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPClaim
metadata:
  name: my-claim
spec:
  poolRef:
    name: my-pool
  address: 192.168.1.10
```

## Best Practices

- Use descriptive names for pools
- Document IP allocation purposes
- Monitor pool utilization
- Plan for growth

