# IP Pool Management

Learn how to manage IP address pools in DCops, from creating pools to allocating IPs.

## Overview

IP pools in DCops provide a high-level abstraction for IP address management:

1. **NetBoxPrefix** - Defines the CIDR block in NetBox
2. **IPPool** - References the prefix and provides allocation strategy
3. **IPClaim** - Requests an IP address from the pool

## Creating an IP Pool

### Step 1: Create NetBoxPrefix

First, create a prefix in NetBox:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxPrefix
metadata:
  name: control-plane-prefix
  namespace: default
spec:
  prefix: "192.168.1.0/24"
  description: "Control plane IP address pool for Talos clusters"
  status: active
  tenant:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxTenant"
    name: "datacenter-tenant"
  site:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxSite"
    name: "datacenter-1"
```

Apply it:

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/netbox-prefix-example.yaml
```

Verify it was created:

```bash
kubectl get netboxprefix control-plane-prefix -o yaml
```

Look for `status.state: Created` and `status.netboxId`.

### Step 2: Create IPPool

Create an IPPool that references the prefix:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPPool
metadata:
  name: control-plane-pool
  namespace: default
spec:
  netboxPrefixRef:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxPrefix"
    name: "control-plane-prefix"
  role: "control-plane"
  allocationStrategy: sequential
```

**Fields:**
- `netboxPrefixRef` - Reference to NetBoxPrefix CRD (required)
- `role` - Pool scope/role (e.g., "control-plane", "worker", "management")
- `allocationStrategy` - How to allocate IPs:
  - `sequential` - Allocate in order (default)
  - `random` - Allocate randomly

Apply it:

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/ippool-example.yaml
```

### Step 3: Verify Pool Status

Check the pool status:

```bash
kubectl get ippool control-plane-pool -o yaml
```

You should see:
- `status.totalIps: 254` (for /24)
- `status.availableIps: 254`
- `status.allocatedIps: 0`
- `status.netboxPrefixId: <number>`

## Allocating IP Addresses

### Create IPClaim

Request an IP address from the pool:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPClaim
metadata:
  name: talos-control-plane-01
  namespace: default
spec:
  poolRef:
    name: control-plane-pool
  deviceRef:
    name: "talos-control-plane-01"
    interface: "eth0"
  preferredIp: "192.168.1.10/24"  # Optional hint
```

**Fields:**
- `poolRef` - Reference to IPPool (required)
- `deviceRef.name` - Device name or identifier (required)
- `deviceRef.interface` - Interface name (optional)
- `preferredIp` - Preferred IP hint in CIDR notation (optional)

Apply it:

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/ipclaim-example.yaml
```

### Verify Allocation

Check the IPClaim status:

```bash
kubectl get ipclaim talos-control-plane-01 -o yaml
```

You should see:
- `status.state: Allocated`
- `status.ip: 192.168.1.10/24`
- `status.netboxIpRef: http://netbox/api/ipam/ip-addresses/123/`

### Check Pool Utilization

After allocation, check the pool:

```bash
kubectl get ippool control-plane-pool -o yaml
```

You should see:
- `status.allocatedIps: 1`
- `status.availableIps: 253`

## Multiple Pools

You can create multiple pools for different purposes:

### Control Plane Pool

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPPool
metadata:
  name: control-plane-pool
spec:
  netboxPrefixRef:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxPrefix"
    name: "control-plane-prefix"
  role: "control-plane"
  allocationStrategy: sequential
```

### Worker Pool

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPPool
metadata:
  name: worker-pool
spec:
  netboxPrefixRef:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxPrefix"
    name: "worker-prefix"
  role: "worker"
  allocationStrategy: sequential
```

### Management Pool

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPPool
metadata:
  name: management-pool
spec:
  netboxPrefixRef:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxPrefix"
    name: "management-prefix"
  role: "management"
  allocationStrategy: sequential
```

## Best Practices

### 1. Use Descriptive Names

Name pools and prefixes clearly:

```yaml
# Good
name: control-plane-pool
name: worker-pool
name: management-pool

# Avoid
name: pool1
name: ip-pool
```

### 2. Document Purpose

Add descriptions to prefixes:

```yaml
spec:
  prefix: "192.168.1.0/24"
  description: "Control plane IP address pool for Talos clusters"
```

### 3. Monitor Utilization

Regularly check pool status:

```bash
kubectl get ippool -o wide
```

Watch for:
- Low available IPs
- High allocation rate
- Need for pool expansion

### 4. Plan for Growth

When creating prefixes, consider:
- Current needs
- Growth projections
- Reserve capacity (don't use 100% of available IPs)

### 5. Use Roles

Assign roles to pools for organization:

```yaml
role: "control-plane"  # For Kubernetes control plane nodes
role: "worker"         # For Kubernetes worker nodes
role: "management"    # For management infrastructure
role: "storage"       # For storage nodes
```

### 6. Sequential vs Random

Choose allocation strategy based on use case:

- **Sequential** - Better for predictable IPs, easier debugging
- **Random** - Better for security (harder to predict IPs)

## Troubleshooting

### IP Allocation Fails

**Issue:** IPClaim shows `state: Failed`

**Check:**
1. Pool exists and is ready:
   ```bash
   kubectl get ippool control-plane-pool
   ```
2. Prefix exists in NetBox:
   ```bash
   kubectl get netboxprefix control-plane-prefix
   ```
3. Available IPs in pool:
   ```bash
   kubectl get ippool control-plane-pool -o jsonpath='{.status.availableIps}'
   ```
4. Controller logs:
   ```bash
   kubectl logs -n dcops-system deployment/netbox-controller | grep -i ipclaim
   ```

### Pool Shows Zero Available IPs

**Issue:** Pool shows `availableIps: 0` but prefix has IPs

**Solutions:**
- Verify prefix was created in NetBox
- Check prefix status in NetBox UI
- Verify prefix CIDR is correct
- Check controller logs for prefix resolution errors

### Preferred IP Not Allocated

**Issue:** Preferred IP hint is ignored

**Note:** `preferredIp` is a hint, not a guarantee. The controller will try to allocate the preferred IP, but may allocate a different IP if:
- Preferred IP is already allocated
- Preferred IP is outside the prefix range
- Preferred IP conflicts with existing allocation

## Example: Complete Setup

Here's a complete example for a Talos Kubernetes cluster:

```yaml
# 1. Prefix
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxPrefix
metadata:
  name: talos-control-plane-prefix
spec:
  prefix: "192.168.1.0/24"
  tenant:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxTenant"
    name: "datacenter-tenant"
---
# 2. Pool
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPPool
metadata:
  name: talos-control-plane-pool
spec:
  netboxPrefixRef:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxPrefix"
    name: "talos-control-plane-prefix"
  role: "control-plane"
---
# 3. IP Claims
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPClaim
metadata:
  name: talos-cp-01
spec:
  poolRef:
    name: talos-control-plane-pool
  deviceRef:
    name: "talos-cp-01"
---
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPClaim
metadata:
  name: talos-cp-02
spec:
  poolRef:
    name: talos-control-plane-pool
  deviceRef:
    name: "talos-cp-02"
---
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPClaim
metadata:
  name: talos-cp-03
spec:
  poolRef:
    name: talos-control-plane-pool
  deviceRef:
    name: "talos-cp-03"
```

## Next Steps

- [IP Address Allocation](../concepts/ip-allocation.md) - Learn more about IP allocation concepts
- [Site Management](./site-management.md) - Organize infrastructure by location
- [CRD Reference](../api-reference/crd-reference.md) - Complete CRD documentation
