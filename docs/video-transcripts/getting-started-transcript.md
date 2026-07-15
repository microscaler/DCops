# Video Transcript: Getting Started with DCops

**Duration:** ~10 minutes  
**Target Audience:** SREs, DevOps Engineers, Infrastructure Engineers  
**Prerequisites:** Basic Kubernetes knowledge, NetBox familiarity helpful but not required

---

## Introduction (0:00 - 0:30)

[Screen: DCops logo and tagline]

**Narrator:** "Welcome to DCops - GitOps Infrastructure Control for On-Premise and Datacenter Infrastructure. In this video, we'll get you up and running with DCops in just a few minutes. By the end, you'll have created your first infrastructure resources and understand how DCops eliminates the need for IP address spreadsheets."

[Screen: Problem statement slide]

**Narrator:** "If you're managing infrastructure in a datacenter, you're probably tracking IP addresses in spreadsheets, managing network inventory in disconnected tools, and dealing with configuration drift. DCops solves all of this by bringing GitOps to your physical infrastructure."

---

## What You'll Learn (0:30 - 1:00)

[Screen: Learning objectives]

**Narrator:** "In this walkthrough, we'll:
1. Install DCops controllers in your Kubernetes cluster
2. Set up a NetBox tenant
3. Create your first site
4. Set up an IP pool
5. Allocate your first IP address

Let's get started!"

---

## Prerequisites Check (1:00 - 1:30)

[Screen: Terminal showing prerequisites]

**Narrator:** "First, let's verify you have everything you need. You'll need:
- A Kubernetes cluster - version 1.24 or later
- kubectl configured to access your cluster
- A NetBox instance - version 4.0 or later
- A NetBox API token with appropriate permissions

Let me check my setup..."

[Screen: Terminal commands]

```bash
kubectl version --client
kubectl get nodes
```

**Narrator:** "Good, I have kubectl and a cluster. Now let's check if NetBox is accessible..."

[Screen: NetBox URL check]

**Narrator:** "Perfect. I have NetBox running. Now let's install DCops."

---

## Step 1: Install CRDs (1:30 - 2:30)

[Screen: Terminal - installing CRDs]

**Narrator:** "The first step is to install the Custom Resource Definitions. These define all the resource types DCops can manage - things like sites, devices, IP pools, and IP claims."

[Screen: Command execution]

```bash
kubectl apply -f config/crd/all-crds.yaml
```

**Narrator:** "This installs 31 CRDs. Let's verify they're installed..."

[Screen: Verifying CRDs]

```bash
kubectl get crds | grep dcops.microscaler.io | wc -l
```

**Narrator:** "Great! All 31 CRDs are installed. You can see them listed here - NetBoxSite, NetBoxDevice, IPPool, IPClaim, and many more."

---

## Step 2: Create Namespace (2:30 - 3:00)

[Screen: Creating namespace]

**Narrator:** "Next, we need to create a namespace for the DCops controllers."

```bash
kubectl create namespace dcops-system
```

**Narrator:** "This namespace will contain the controller deployment and related resources."

---

## Step 3: Configure NetBox Connection (3:00 - 4:00)

[Screen: Creating NetBox secret]

**Narrator:** "Now we need to configure the connection to NetBox. We'll create a Kubernetes Secret with the NetBox API token."

[Screen: Secret creation]

```bash
kubectl create secret generic netbox-token \
  --from-literal=token=YOUR_NETBOX_API_TOKEN \
  --namespace=dcops-system
```

**Narrator:** "Replace YOUR_NETBOX_API_TOKEN with your actual NetBox API token. You can create this token in the NetBox UI under User Menu → API Tokens."

[Screen: NetBox UI showing token creation]

**Narrator:** "The token needs read and write permissions for IPAM, DCIM, Tenancy, and Extras. Once you have the token, create the secret as shown."

---

## Step 4: Deploy Controller (4:00 - 5:00)

[Screen: Deploying controller]

**Narrator:** "Now let's deploy the NetBox controller. We'll use kustomize to apply all the controller resources."

```bash
kubectl apply -k config/netbox-controller
```

**Narrator:** "This creates the ServiceAccount, Role, RoleBinding, Deployment, and Secret. Let's check that the controller is running..."

[Screen: Checking controller status]

```bash
kubectl get pods -n dcops-system
```

**Narrator:** "Perfect! The controller is running. You can see it's in the Running state. Let's check the logs to make sure it connected to NetBox..."

```bash
kubectl logs -n dcops-system deployment/netbox-controller | head -20
```

**Narrator:** "Great! The controller is connected and ready to reconcile resources."

---

## Step 5: Set Up Tenant (5:00 - 6:30)

[Screen: Creating tenant]

**Narrator:** "Most NetBox resources require a tenant. Let's create our first tenant. First, we need to create a secret for the tenant's API token..."

```bash
kubectl create secret generic netbox-token-datacenter-tenant \
  --from-literal=token=TENANT_API_TOKEN \
  --namespace=default
```

**Narrator:** "Now let's create the NetBoxTenant CRD. This tells DCops about the tenant and which secret contains its API token."

[Screen: Tenant YAML]

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

[Screen: Applying tenant]

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/netbox-tenant-example.yaml
```

**Narrator:** "Let's check the status..."

```bash
kubectl get netboxtenant datacenter-tenant -o yaml
```

**Narrator:** "Excellent! The tenant was created. You can see the status shows `state: Created` and there's a `netboxId` which means it was successfully created in NetBox."

---

## Step 6: Create Your First Site (6:30 - 7:30)

[Screen: Creating site]

**Narrator:** "Now let's create your first site. A site represents a physical location like a datacenter or colocation facility."

[Screen: Site YAML]

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxSite
metadata:
  name: datacenter-1
  namespace: default
spec:
  name: "Data Center 1"
  slug: "datacenter-1"
  tenant:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxTenant"
    name: "datacenter-tenant"
```

[Screen: Applying site]

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/netbox-site-example.yaml
```

**Narrator:** "Let's watch the status change..."

```bash
kubectl get netboxsite datacenter-1 -w
```

**Narrator:** "You can see the state change from Pending to Created, and the netboxId appears. This means the site was successfully created in NetBox!"

---

## Step 7: Create IP Pool (7:30 - 8:30)

[Screen: Creating prefix and pool]

**Narrator:** "Now let's set up IP address management. First, we create a prefix in NetBox, then an IP pool that references it."

[Screen: Prefix YAML]

**Narrator:** "The prefix defines the CIDR block - in this case, 192.168.1.0/24."

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/netbox-prefix-example.yaml
```

[Screen: IPPool YAML]

**Narrator:** "Now the IP pool references that prefix..."

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/ippool-example.yaml
```

**Narrator:** "Let's check the pool status..."

```bash
kubectl get ippool control-plane-pool -o yaml
```

**Narrator:** "Perfect! The pool shows 254 total IPs available. This is for a /24 network, which gives us 254 usable IP addresses."

---

## Step 8: Allocate Your First IP (8:30 - 9:30)

[Screen: Creating IPClaim]

**Narrator:** "Now for the exciting part - allocating your first IP address! We'll create an IPClaim that requests an IP from the pool."

[Screen: IPClaim YAML]

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPClaim
metadata:
  name: talos-control-plane-01
spec:
  poolRef:
    name: control-plane-pool
  deviceRef:
    name: "talos-control-plane-01"
    interface: "eth0"
  preferredIp: "192.168.1.10/24"
```

[Screen: Applying IPClaim]

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/ipclaim-example.yaml
```

**Narrator:** "Let's watch it get allocated..."

```bash
kubectl get ipclaim talos-control-plane-01 -w
```

**Narrator:** "Excellent! The state changed to Allocated and we can see the IP address: 192.168.1.10/24. This IP is now allocated in NetBox and tracked in Git!"

---

## Verification in NetBox (9:30 - 10:00)

[Screen: NetBox UI]

**Narrator:** "Let's verify everything in NetBox. I'll open the NetBox UI..."

[Screen: NetBox showing site]

**Narrator:** "Here's our site - Data Center 1. It was created automatically by DCops."

[Screen: NetBox showing IP address]

**Narrator:** "And here's our allocated IP address - 192.168.1.10/24. It shows as allocated and is associated with our device."

[Screen: NetBox showing prefix]

**Narrator:** "And the prefix shows 1 IP allocated out of 254 available. Perfect!"

---

## What We Accomplished (10:00 - 10:30)

[Screen: Summary slide]

**Narrator:** "In just a few minutes, we:
1. Installed DCops controllers
2. Created a tenant
3. Created a site
4. Set up an IP pool
5. Allocated an IP address

All of this is now tracked in Git, version controlled, and automatically reconciled with NetBox. No more spreadsheets!"

---

## Next Steps (10:30 - 11:00)

[Screen: Next steps slide]

**Narrator:** "To learn more:
- Check out the Quick Start guide for more examples
- Learn about IP Pool Management
- Explore Site Management for organizing infrastructure
- Read the complete CRD Reference

All documentation is available in the DCops UI. Thanks for watching!"

[Screen: End screen with links]

---

## Production Notes for Video Creation

### Visual Elements Needed:
1. Terminal screen recordings for all commands
2. NetBox UI screenshots/demo
3. Kubernetes dashboard or kubectl output
4. Git repository view showing YAML files
5. Animated diagrams showing the workflow

### Timing Notes:
- Pause after each command to show results
- Highlight important output (status changes, IDs, etc.)
- Use zoom/pan on terminal output for readability
- Show NetBox UI transitions smoothly

### Voiceover Tips:
- Speak clearly and at moderate pace
- Pause before important commands
- Emphasize key concepts (GitOps, automatic reconciliation)
- Use enthusiasm for the "exciting parts" (first IP allocation)

