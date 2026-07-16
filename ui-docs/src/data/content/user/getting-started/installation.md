# Installation

DCops integrates with your existing Kubernetes cluster and NetBox instance. This guide walks you through the complete installation process.

## Prerequisites

Before installing DCops, ensure you have:

- **Kubernetes cluster** (1.24 or later)
- **NetBox instance** (4.0 or later) with API access
- **kubectl** configured to access your cluster
- **NetBox API token** with appropriate permissions

### NetBox API Token Permissions

Your NetBox API token needs:
- Read/Write access to IPAM (IP addresses, prefixes, aggregates, VLANs)
- Read/Write access to DCIM (sites, devices, interfaces)
- Tenant management permissions
- Tag management permissions

## Installation Steps

### Step 1: Install Custom Resource Definitions (CRDs)

First, install all DCops CRDs:

```bash
kubectl apply -f config/crd/all-crds.yaml
```

Verify the CRDs are installed:

```bash
kubectl get crds | grep dcops.microscaler.io
```

You should see all 31 CRDs listed.

### Step 2: Create Namespace

Create the namespace for DCops controllers:

```bash
kubectl create namespace dcops-system
```

### Step 3: Configure NetBox Connection

Create a Kubernetes Secret with your NetBox credentials. The controller will use this to connect to NetBox.

**Option A: Using kubectl**

```bash
kubectl create secret generic netbox-token \
  --from-literal=token=YOUR_NETBOX_API_TOKEN \
  --namespace=dcops-system
```

**Option B: Using YAML**

Create `netbox-secret.yaml`:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: netbox-token
  namespace: dcops-system
type: Opaque
stringData:
  token: YOUR_NETBOX_API_TOKEN
```

Apply it:

```bash
kubectl apply -f netbox-secret.yaml
```

**Note:** The controller expects the NetBox URL to be set via environment variable or ConfigMap. For local development with Tilt, this is configured automatically.

### Step 4: Deploy NetBox Controller

Deploy the controller using kustomize:

```bash
kubectl apply -k config/netbox-controller
```

This will create:
- ServiceAccount
- Role and RoleBinding (RBAC)
- Deployment
- Secret (if not already created)

### Step 5: Verify Installation

Check that the controller is running:

```bash
kubectl get pods -n dcops-system
```

You should see the `netbox-controller` pod in `Running` state.

Check controller logs:

```bash
kubectl logs -n dcops-system deployment/netbox-controller
```

### Step 6: Set Up Tenant (Required)

Most NetBox resources require a tenant. Set up your first tenant:

1. **Create NetBox Tenant in NetBox UI:**
   - Go to NetBox UI → Tenancy → Tenants
   - Create a new tenant (e.g., "datacenter-tenant")
   - Create an API token for this tenant

2. **Create Kubernetes Secret for Tenant Token:**

```bash
kubectl create secret generic netbox-token-datacenter-tenant \
  --from-literal=token=TENANT_API_TOKEN \
  --namespace=default
```

3. **Create NetBoxTenant CRD:**

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

Verify the tenant was created in NetBox:

```bash
kubectl get netboxtenant
kubectl describe netboxtenant datacenter-tenant
```

The status should show `netboxId` and `state: Created` when successful.

## Development Installation with Tilt

For local development, use Tilt for automatic rebuilding and deployment:

1. **Set up Kind cluster** (if not already done):

```bash
python3 scripts/setup_kind.py
```

2. **Start Tilt:**

```bash
tilt up
```

Tilt will automatically:
- Build and deploy NetBox
- Build and deploy controllers
- Set up tenants and tokens
- Apply example CRs

See [Development Setup](../../contributor/development/setup.md) for more details.

## Troubleshooting

### Controller Not Starting

Check the controller logs:

```bash
kubectl logs -n dcops-system deployment/netbox-controller
```

Common issues:
- **NetBox connection failed:** Verify NetBox URL and token
- **Secret not found:** Ensure the secret exists in `dcops-system` namespace
- **RBAC errors:** Check Role and RoleBinding are created

### CRDs Not Found

If you get "resource not found" errors:

```bash
# Reinstall CRDs
kubectl apply -f config/crd/all-crds.yaml

# Verify
kubectl get crds | grep dcops
```

### Tenant Creation Fails

If tenant creation fails:
- Verify the tenant token secret exists
- Check NetBox API token has tenant management permissions
- Review controller logs for specific error messages

## Next Steps

- [Quick Start Guide](./quick-start.md) - Create your first resources
- [NetBox Setup](../guides/netbox-setup.md) - Detailed NetBox configuration
- [CRD Reference](../api-reference/crd-reference.md) - Complete CRD documentation
