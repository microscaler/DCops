# Quick Start

Get DCops up and running in minutes with a complete working example.

## Prerequisites

- DCops installed (see [Installation Guide](./installation.md))
- NetBox tenant created (see Installation Guide, Step 6)
- NetBox API token with appropriate permissions

## Complete Example: Control Plane IP Pool

This example creates a complete IP address management setup for a Kubernetes control plane.

### Step 1: Create Platform Resources

First, create the platform-level resources (manufacturer, device type, etc.):

```bash
kubectl apply -f config/examples/platform/
```

This creates:
- NetBoxManufacturer (Raspberry Pi)
- NetBoxDeviceType (Raspberry Pi 4 Model B)
- NetBoxDeviceRole (Kubernetes Control Plane)
- NetBoxPlatform (Talos Linux)

### Step 2: Create Tenant Resources

Create the tenant-specific resources:

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/
```

This creates:
- NetBoxTenant (if not already created)
- NetBoxRegion
- NetBoxSite
- NetBoxPrefix
- IPPool
- And more...

### Step 3: Verify Resources

Check that resources are being reconciled:

```bash
# Check tenant
kubectl get netboxtenant datacenter-tenant -o yaml

# Check site
kubectl get netboxsite datacenter-1 -o yaml

# Check prefix
kubectl get netboxprefix control-plane-prefix -o yaml

# Check IP pool
kubectl get ippool control-plane-pool -o yaml
```

Look for `status.state: Created` and `status.netboxId` to confirm resources were created in NetBox.

### Step 4: Claim an IP Address

Create an IPClaim to allocate an IP address:

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
  preferredIp: "192.168.1.10/24"
```

Apply it:

```bash
kubectl apply -f - <<EOF
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
  preferredIp: "192.168.1.10/24"
EOF
```

### Step 5: Verify IP Allocation

Check the IPClaim status:

```bash
kubectl get ipclaim talos-control-plane-01 -o yaml
```

You should see:
- `status.state: Allocated`
- `status.ip: 192.168.1.10/24`
- `status.netboxIpRef: http://netbox/api/ipam/ip-addresses/123/`

### Step 6: Create a Device

Create a device that uses the allocated IP:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxDevice
metadata:
  name: talos-control-plane-01
  namespace: default
spec:
  name: "talos-control-plane-01"
  deviceType:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxDeviceType"
    name: "raspberry-pi-4-model-b"
  deviceRole:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxDeviceRole"
    name: "kubernetes-control-plane"
  site:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxSite"
    name: "datacenter-1"
  tenant:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxTenant"
    name: "datacenter-tenant"
  primaryIp4:
    ipClaimRef:
      apiGroup: "dcops.microscaler.io"
      kind: "IPClaim"
      name: "talos-control-plane-01"
  status: active
```

Apply it:

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/netbox-device-example.yaml
```

### Step 7: Verify in NetBox

Check that everything was created in NetBox:

1. **Log into NetBox UI**
2. **Check IP Addresses:**
   - IPAM → IP Addresses
   - You should see `192.168.1.10/24` allocated
3. **Check Devices:**
   - DCIM → Devices
   - You should see `talos-control-plane-01` with the IP address assigned
4. **Check Prefix:**
   - IPAM → Prefixes
   - You should see `192.168.1.0/24` with usage statistics

## Expected Results

After completing this quick start, you should have:

✅ **NetBox Resources Created:**
- 1 Tenant
- 1 Site
- 1 Prefix (192.168.1.0/24)
- 1 IP Address (192.168.1.10/24)
- 1 Device (talos-control-plane-01)

✅ **Kubernetes Resources:**
- All CRDs in `Created` or `Allocated` state
- Status fields populated with NetBox IDs and URLs

✅ **GitOps Workflow:**
- All infrastructure defined in Git
- Changes tracked in Git history
- NetBox automatically reconciled

## Common Issues

### IP Allocation Fails

If IPClaim shows `state: Failed`:

1. Check IPPool status:
   ```bash
   kubectl get ippool control-plane-pool -o yaml
   ```
2. Verify prefix exists in NetBox
3. Check available IPs in the pool
4. Review controller logs:
   ```bash
   kubectl logs -n dcops-system deployment/netbox-controller
   ```

### Device Creation Fails

If NetBoxDevice shows `state: Failed`:

1. Verify all dependencies exist:
   - DeviceType
   - DeviceRole
   - Site
   - Tenant
2. Check that IPClaim is in `Allocated` state
3. Review controller logs for specific errors

### Resources Not Reconciling

If resources stay in `Pending` state:

1. Check controller is running:
   ```bash
   kubectl get pods -n dcops-system
   ```
2. Check controller logs for errors
3. Verify NetBox connection:
   ```bash
   kubectl logs -n dcops-system deployment/netbox-controller | grep -i netbox
   ```

## Next Steps

Now that you have a working setup:

- [IP Address Allocation](../concepts/ip-allocation.md) - Learn more about IP pools and claims
- [Infrastructure Inventory](../concepts/infrastructure-inventory.md) - Manage sites and devices
- [IP Pool Management](../guides/ip-pool-management.md) - Advanced IP pool configuration
- [Site Management](../guides/site-management.md) - Organize your infrastructure
- [CRD Reference](../api-reference/crd-reference.md) - Complete CRD documentation

## Example Files

All example files are available in:
- `config/examples/platform/` - Platform resources
- `config/examples/tenant-datacenter-tenant/` - Tenant-specific resources

You can copy and modify these examples for your own infrastructure.
