# NetBox Controller

The NetBox Controller is the core component of DCops. It manages all NetBox resources through Kubernetes CRDs, providing GitOps-driven infrastructure management.

## Overview

The NetBox Controller:
- Watches all NetBox CRDs in Kubernetes
- Reconciles desired state (Git) with actual state (NetBox)
- Detects and corrects drift
- Emits Kubernetes events for observability
- Supports multi-tenant operations

## Reconciliation Behavior

### Continuous Reconciliation

The controller continuously reconciles resources:

1. **Watch for Changes** - Watches Kubernetes API for CRD changes
2. **Debounce** - Waits 5 seconds after last event before reconciling (batches updates)
3. **Reconcile** - Compares Git state (CRD spec) with NetBox state
4. **Create/Update** - Creates missing resources or updates changed resources
5. **Requeue** - Requeues after 10 seconds for continuous monitoring

### Reconciliation Flow

```
CRD Created/Updated
    ↓
Controller Detects Change
    ↓
Resolve Dependencies (tenant, site, etc.)
    ↓
Check NetBox State
    ↓
Create or Update in NetBox
    ↓
Update CRD Status
    ↓
Emit Kubernetes Event
```

### Drift Detection

The controller automatically detects and corrects drift:

- **Resource Deleted in NetBox** - Controller recreates it
- **Resource Modified in NetBox** - Controller updates it to match Git state
- **Dependency Missing** - Controller waits and retries

### Dependency Resolution

Resources are reconciled in dependency order:

1. **Tenant** - Must exist first (required by most resources)
2. **Platform Resources** - Manufacturer, DeviceType, DeviceRole, Platform
3. **Infrastructure** - Region, Site, Location
4. **IPAM** - Prefix, IPAddress, VLAN
5. **Devices** - Device, Interface, MACAddress

If a dependency is missing, the controller:
- Sets status to `Pending`
- Retries every 10 seconds
- Emits events when dependencies become available

## Status Fields

All NetBox CRDs include status fields:

### Common Status Fields

- `netboxId` - NetBox resource ID (set after creation)
- `netboxUrl` - NetBox API URL to the resource
- `state` - Resource state (see below)
- `error` - Error message if reconciliation failed
- `lastReconciled` - Last reconciliation timestamp

### Resource States

- **Pending** - Waiting for dependencies or initial creation
- **Created** - Successfully created in NetBox
- **Updated** - Successfully updated in NetBox
- **Failed** - Reconciliation failed (check error field)

### IPPool Status Fields

- `totalIps` - Total available IPs in pool
- `allocatedIps` - Number of allocated IPs
- `availableIps` - Number of available IPs
- `netboxPrefixId` - Resolved NetBox prefix ID

### IPClaim Status Fields

- `state` - Allocation state (Pending, Allocated, Failed)
- `ip` - Allocated IP address (CIDR notation)
- `netboxIpRef` - NetBox IPAddress object reference

## Error Handling

### Error Types

The controller handles several error types:

1. **NetBox Errors**
   - Connection failures
   - Authentication errors
   - Validation errors
   - Not found errors

2. **Dependency Errors**
   - Missing tenant
   - Missing site
   - Missing device type
   - Missing references

3. **Validation Errors**
   - Invalid CIDR notation
   - Invalid MAC address format
   - Missing required fields

### Error Recovery

The controller uses exponential backoff for errors:

- **First Error** - Retry after 1 second
- **Second Error** - Retry after 2 seconds
- **Third Error** - Retry after 3 seconds
- **Continues** - Fibonacci backoff (1, 2, 3, 5, 8, 13, ...)

Errors are logged and emitted as Kubernetes events.

### Error Status

When reconciliation fails:

1. Status is updated with `state: Failed`
2. Error message is stored in `status.error`
3. Kubernetes event is emitted
4. Controller retries with backoff

## Multi-Tenant Support

The controller supports multiple tenants:

### Tenant Isolation

- Each tenant has its own API token (stored in Kubernetes Secret)
- Resources reference tenants via CRD references
- Controller uses tenant-specific tokens for NetBox API calls

### Token Resolution

The controller resolves tokens using:

1. **Tenant CRD** - References a Kubernetes Secret
2. **Secret Lookup** - Retrieves token from Secret
3. **Token Usage** - Uses token for all NetBox API calls for that tenant

### Shared Resources

Some resources can be shared across tenants:
- Manufacturer
- DeviceType
- DeviceRole
- Platform
- RIR

## Event Emission

The controller emits Kubernetes events for:

- **Resource Created** - When resource is created in NetBox
- **Resource Updated** - When resource is updated in NetBox
- **Resource Deleted** - When resource is deleted in NetBox (drift)
- **Recreation** - When resource is recreated after drift
- **Error** - When reconciliation fails
- **Retry** - When retrying after error

View events:

```bash
kubectl get events --field-selector involvedObject.name=datacenter-tenant
```

## Monitoring

### Controller Health

Check controller status:

```bash
kubectl get pods -n dcops-system
kubectl logs -n dcops-system deployment/netbox-controller
```

### Resource Status

Check resource reconciliation status:

```bash
# All resources
kubectl get netboxsite,netboxdevice,netboxprefix -A

# Specific resource
kubectl get netboxsite datacenter-1 -o yaml

# Watch for changes
kubectl get netboxsite datacenter-1 -w
```

### Metrics

The controller logs reconciliation metrics:
- Reconciliation duration
- Success/failure rates
- Error counts
- Drift detection events

## Configuration

### Environment Variables

- `NETBOX_URL` - NetBox API URL
- `NETBOX_TOKEN` - NetBox API token (from Secret)
- `WATCH_NAMESPACE` - Namespace to watch (empty = all namespaces)

### Resource Limits

Default resource limits:

```yaml
resources:
  requests:
    cpu: 100m
    memory: 128Mi
  limits:
    cpu: 500m
    memory: 256Mi
```

### Concurrency

- **Per Resource Type** - 3 concurrent reconciliations
- **Total** - Up to 51 concurrent reconciliations (17 watchers × 3)
- **Debounce** - 5 seconds (batches status updates)

## Troubleshooting

### Controller Not Reconciling

**Check:**
1. Controller is running:
   ```bash
   kubectl get pods -n dcops-system
   ```
2. NetBox connection:
   ```bash
   kubectl logs -n dcops-system deployment/netbox-controller | grep -i netbox
   ```
3. RBAC permissions:
   ```bash
   kubectl get role,rolebinding -n dcops-system
   ```

### Resources Stuck in Pending

**Check:**
1. Dependencies exist:
   ```bash
   kubectl get netboxtenant,netboxsite
   ```
2. Dependencies are ready:
   ```bash
   kubectl get netboxtenant datacenter-tenant -o jsonpath='{.status.state}'
   ```
3. Controller logs for dependency errors

### Resources Stuck in Failed

**Check:**
1. Error message:
   ```bash
   kubectl get netboxsite datacenter-1 -o jsonpath='{.status.error}'
   ```
2. Controller logs:
   ```bash
   kubectl logs -n dcops-system deployment/netbox-controller | grep -i error
   ```
3. NetBox API status:
   - Verify NetBox is accessible
   - Check API token permissions
   - Verify resource doesn't conflict in NetBox

## Best Practices

### 1. Monitor Events

Watch for reconciliation events:

```bash
kubectl get events -w --field-selector involvedObject.kind=NetBoxSite
```

### 2. Check Status Regularly

Monitor resource status:

```bash
kubectl get netboxsite,netboxdevice,netboxprefix -o custom-columns=NAME:.metadata.name,STATE:.status.state,ERROR:.status.error
```

### 3. Review Logs

Check controller logs for issues:

```bash
kubectl logs -n dcops-system deployment/netbox-controller --tail=100
```

### 4. Verify in NetBox UI

Always verify resources in NetBox UI:
- Check resources were created
- Verify fields match CRD spec
- Check for any manual modifications

## Next Steps

- [CRD Reference](./crd-reference.md) - Complete CRD documentation
- [Installation Guide](../getting-started/installation.md) - Deploy the controller
- [Quick Start](../getting-started/quick-start.md) - Create your first resources
