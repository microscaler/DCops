# NetBox Setup

NetBox is the authoritative source for IPAM and inventory in DCops. This guide covers setting up NetBox for use with DCops.

## Requirements

- **NetBox 4.0 or later** - DCops is designed for NetBox 4.0+
- **API access** - NetBox must be accessible from your Kubernetes cluster
- **API token** - Token with appropriate permissions

## NetBox Installation

DCops doesn't install NetBox for you. You need to have NetBox running separately. Options:

### Option 1: Use Tilt (Development)

For local development, Tilt automatically deploys NetBox:

```bash
tilt up
```

This deploys NetBox, PostgreSQL, and Redis in the shared Kind cluster. NetBox will be available at `http://localhost:8011` when Tilt is running.

### Option 2: Existing NetBox Instance

If you have an existing NetBox instance:

1. Ensure it's accessible from your Kubernetes cluster
2. Note the URL (e.g., `https://netbox.example.com`)
3. Create an API token in NetBox UI

### Option 3: Manual Installation

Follow the [official NetBox installation guide](https://docs.netbox.dev/en/stable/installation/) for your environment.

## API Token Setup

### Create API Token in NetBox

1. **Log into NetBox UI**
2. **Navigate to:** User Menu → API Tokens → Add Token
3. **Configure:**
   - Description: "DCops Controller"
   - Expires: Set expiration or leave blank for no expiration
   - Write enabled: Yes
   - Permissions: Full access (or custom permissions below)

### Required Permissions

Your API token needs:

- **IPAM:**
  - View, Add, Change, Delete for Prefixes
  - View, Add, Change, Delete for IP Addresses
  - View, Add, Change, Delete for IP Ranges
  - View, Add, Change, Delete for Aggregates
  - View, Add, Change, Delete for VLANs
  - View, Add, Change, Delete for Roles

- **DCIM:**
  - View, Add, Change, Delete for Sites
  - View, Add, Change, Delete for Regions
  - View, Add, Change, Delete for Locations
  - View, Add, Change, Delete for Devices
  - View, Add, Change, Delete for Interfaces
  - View, Add, Change, Delete for Device Types
  - View, Add, Change, Delete for Device Roles
  - View, Add, Change, Delete for Manufacturers
  - View, Add, Change, Delete for Platforms

- **Tenancy:**
  - View, Add, Change, Delete for Tenants

- **Extras:**
  - View, Add, Change, Delete for Tags

### Create Kubernetes Secret

Store the API token in a Kubernetes Secret:

```bash
kubectl create secret generic netbox-token \
  --from-literal=token=YOUR_API_TOKEN \
  --namespace=dcops-system
```

Or using YAML:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: netbox-token
  namespace: dcops-system
type: Opaque
stringData:
  token: YOUR_API_TOKEN
```

## Controller Configuration

The NetBox controller connects to NetBox using environment variables:

- `NETBOX_URL` - NetBox API URL (e.g., `http://netbox.netbox:80` or `https://netbox.example.com`)
- `NETBOX_TOKEN` - NetBox API token (from Secret)

For local development with Tilt, these are configured automatically. For production, set them in the deployment:

```yaml
env:
- name: NETBOX_URL
  value: "https://netbox.example.com"
- name: NETBOX_TOKEN
  valueFrom:
    secretKeyRef:
      name: netbox-token
      key: token
```

## Tenant Setup

Most NetBox resources require a tenant. Set up your first tenant:

### 1. Create Tenant in NetBox UI

1. Go to **Tenancy → Tenants**
2. Click **Add Tenant**
3. Fill in:
   - Name: `datacenter-tenant`
   - Slug: `datacenter-ops`
   - Description: "Primary tenant for datacenter operations"
4. Save

### 2. Create API Token for Tenant

1. Go to **User Menu → API Tokens → Add Token**
2. Create a token for the tenant user (or use admin token)
3. Copy the token

### 3. Create Kubernetes Secret

```bash
kubectl create secret generic netbox-token-datacenter-tenant \
  --from-literal=token=TENANT_API_TOKEN \
  --namespace=default
```

### 4. Create NetBoxTenant CRD

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxTenant
metadata:
  name: datacenter-tenant
  namespace: default
spec:
  name: "Data Center Operations"
  slug: "datacenter-ops"
  tokenSecret:
    name: netbox-token-datacenter-tenant
```

Apply it:

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/netbox-tenant-example.yaml
```

## Verification

### Test NetBox Connection

Check controller logs to verify NetBox connection:

```bash
kubectl logs -n dcops-system deployment/netbox-controller | grep -i netbox
```

You should see successful API calls, not connection errors.

### Test Tenant Creation

Verify the tenant was created:

```bash
kubectl get netboxtenant datacenter-tenant -o yaml
```

Look for:
- `status.state: Created`
- `status.netboxId: <number>`
- `status.netboxUrl: http://netbox/api/tenancy/tenants/<id>/`

### Test in NetBox UI

1. Log into NetBox UI
2. Navigate to **Tenancy → Tenants**
3. You should see `datacenter-tenant` listed
4. Click on it to see details

## Troubleshooting

### Connection Errors

**Error:** `Failed to connect to NetBox`

**Solutions:**
- Verify NetBox URL is correct
- Check network connectivity from cluster to NetBox
- Verify NetBox is running and accessible
- Check firewall rules

### Authentication Errors

**Error:** `401 Unauthorized` or `403 Forbidden`

**Solutions:**
- Verify API token is correct
- Check token hasn't expired
- Verify token has required permissions
- Regenerate token if needed

### Tenant Creation Fails

**Error:** `Failed to create tenant`

**Solutions:**
- Verify tenant token secret exists
- Check token has tenant management permissions
- Verify tenant name doesn't already exist in NetBox
- Review controller logs for specific error

## Next Steps

- [Installation Guide](../getting-started/installation.md) - Complete DCops installation
- [Quick Start](../getting-started/quick-start.md) - Create your first resources
- [Site Management](./site-management.md) - Organize your infrastructure
