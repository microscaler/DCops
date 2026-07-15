# Video Transcript: IP Pool Management with DCops

**Duration:** ~12 minutes  
**Target Audience:** SREs, Network Engineers, Infrastructure Engineers  
**Prerequisites:** DCops installed, basic understanding of IP addressing

---

## Introduction (0:00 - 0:30)

[Screen: DCops logo]

**Narrator:** "Welcome to IP Pool Management with DCops. In this video, we'll learn how to manage IP address pools, allocate IPs to devices, and track utilization - all through GitOps."

[Screen: Problem statement]

**Narrator:** "Managing IP addresses in spreadsheets is error-prone and doesn't scale. DCops brings GitOps to IP address management, giving you version control, automatic allocation, and conflict prevention."

---

## What You'll Learn (0:30 - 1:00)

[Screen: Learning objectives]

**Narrator:** "In this walkthrough, we'll:
1. Understand the IP pool architecture
2. Create a prefix in NetBox
3. Create an IP pool
4. Allocate multiple IP addresses
5. Monitor pool utilization
6. Learn best practices

Let's dive in!"

---

## IP Pool Architecture (1:00 - 2:30)

[Screen: Architecture diagram]

**Narrator:** "DCops uses a three-layer architecture for IP management:

First, we have **NetBoxPrefix** - this represents a CIDR block in NetBox, like 192.168.1.0/24.

Second, we have **IPPool** - this is a high-level abstraction that references a prefix and provides allocation strategy.

Third, we have **IPClaim** - this requests an IP address from a pool.

Let me show you how they work together..."

[Screen: Diagram animation]

**Narrator:** "The prefix defines the available IP space. The pool provides allocation logic - sequential or random. And claims request specific IPs from the pool. All of this is tracked in Git and automatically synced to NetBox."

---

## Step 1: Create a Prefix (2:30 - 4:00)

[Screen: Terminal - creating prefix]

**Narrator:** "Let's start by creating a prefix. This represents a CIDR block in NetBox."

[Screen: Prefix YAML file]

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxPrefix
metadata:
  name: control-plane-prefix
  namespace: default
spec:
  prefix: "192.168.1.0/24"
  description: "Control plane IP address pool"
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

**Narrator:** "Notice we're using Kubernetes object references - apiGroup, kind, and name. This is GitOps-friendly because we reference resources by name, not by ID. The controller resolves these references automatically."

[Screen: Applying prefix]

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/netbox-prefix-example.yaml
```

**Narrator:** "Let's check the status..."

```bash
kubectl get netboxprefix control-plane-prefix -o yaml
```

**Narrator:** "Perfect! The prefix was created. You can see the state is Created and there's a netboxId. This means it exists in NetBox now."

---

## Step 2: Create an IP Pool (4:00 - 5:30)

[Screen: Creating IPPool]

**Narrator:** "Now let's create an IP pool that references this prefix. The pool provides the allocation logic."

[Screen: IPPool YAML]

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

**Narrator:** "The pool references the prefix we just created. We're using sequential allocation, which means IPs will be allocated in order. You could also use random allocation for better security."

[Screen: Applying pool]

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/ippool-example.yaml
```

**Narrator:** "Let's check the pool status..."

```bash
kubectl get ippool control-plane-pool -o yaml
```

**Narrator:** "Excellent! The pool shows 254 total IPs, all available, and 0 allocated. The netboxPrefixId shows it successfully resolved the prefix reference."

---

## Step 3: Allocate Your First IP (5:30 - 7:00)

[Screen: Creating IPClaim]

**Narrator:** "Now let's allocate our first IP address. We'll create an IPClaim."

[Screen: IPClaim YAML]

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

**Narrator:** "The IPClaim references the pool and specifies which device needs the IP. We've included a preferred IP as a hint, but the controller will allocate the next available IP if that one is taken."

[Screen: Applying claim]

```bash
kubectl apply -f config/examples/tenant-datacenter-tenant/ipclaim-example.yaml
```

**Narrator:** "Let's watch it get allocated..."

```bash
kubectl get ipclaim talos-control-plane-01 -w
```

**Narrator:** "Perfect! The state changed to Allocated and we got 192.168.1.10/24. Notice it used our preferred IP since it was available."

[Screen: Checking pool status]

```bash
kubectl get ippool control-plane-pool
```

**Narrator:** "The pool now shows 1 allocated and 253 available. Great!"

---

## Step 4: Allocate Multiple IPs (7:00 - 8:30)

[Screen: Creating multiple claims]

**Narrator:** "Let's allocate a few more IPs to see sequential allocation in action. I'll create three more IPClaims..."

[Screen: Multiple IPClaims]

```bash
kubectl apply -f - <<EOF
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPClaim
metadata:
  name: talos-cp-02
spec:
  poolRef:
    name: control-plane-pool
  deviceRef:
    name: "talos-cp-02"
---
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPClaim
metadata:
  name: talos-cp-03
spec:
  poolRef:
    name: control-plane-pool
  deviceRef:
    name: "talos-cp-03"
---
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPClaim
metadata:
  name: talos-cp-04
spec:
  poolRef:
    name: control-plane-pool
  deviceRef:
    name: "talos-cp-04"
EOF
```

**Narrator:** "Let's check what IPs were allocated..."

```bash
kubectl get ipclaim -o custom-columns=NAME:.metadata.name,IP:.status.ip,STATE:.status.state
```

**Narrator:** "Perfect! You can see we got 192.168.1.10, 192.168.1.11, 192.168.1.12, and 192.168.1.13 - sequential allocation working as expected!"

[Screen: Pool utilization]

```bash
kubectl get ippool control-plane-pool
```

**Narrator:** "The pool now shows 4 allocated and 250 available. This is exactly what we'd expect."

---

## Step 5: Monitor Utilization (8:30 - 9:30)

[Screen: Monitoring commands]

**Narrator:** "Let's look at some ways to monitor your IP pools..."

[Screen: List all pools]

```bash
kubectl get ippool -o wide
```

**Narrator:** "This shows all pools with their utilization. You can see total IPs, allocated, and available for each pool."

[Screen: Detailed pool view]

```bash
kubectl get ippool control-plane-pool -o yaml | grep -A 5 status
```

**Narrator:** "For more detail, you can get the full YAML. The status shows exactly how many IPs are allocated and available."

[Screen: List all claims]

```bash
kubectl get ipclaim -o custom-columns=NAME:.metadata.name,POOL:.spec.poolRef.name,IP:.status.ip,STATE:.status.state
```

**Narrator:** "And here are all the IP claims, showing which pool they're from and what IP was allocated."

---

## Best Practices (9:30 - 11:00)

[Screen: Best practices slide]

**Narrator:** "Let's talk about best practices for IP pool management..."

### 1. Use Descriptive Names

[Screen: Good vs bad naming]

**Narrator:** "Use clear, descriptive names:
- Good: `control-plane-pool`, `worker-pool`, `management-pool`
- Avoid: `pool1`, `ip-pool`, `test-pool`

This makes it easy to understand what each pool is for."

### 2. Document Purpose

[Screen: Prefix with description]

**Narrator:** "Always add descriptions to your prefixes:

```yaml
spec:
  prefix: "192.168.1.0/24"
  description: "Control plane IP address pool for Talos clusters"
```

This helps team members understand the purpose."

### 3. Monitor Utilization

[Screen: Monitoring script]

**Narrator:** "Regularly check pool status. You can create a simple script:

```bash
kubectl get ippool -o custom-columns=NAME:.metadata.name,TOTAL:.status.totalIps,ALLOCATED:.status.allocatedIps,AVAILABLE:.status.availableIps
```

Watch for pools getting low on available IPs."

### 4. Plan for Growth

[Screen: Growth planning]

**Narrator:** "When creating prefixes, consider:
- Current needs
- Growth projections  
- Reserve capacity

Don't use 100% of available IPs - leave room for growth."

### 5. Use Roles

[Screen: Role examples]

**Narrator:** "Assign roles to pools for organization:

```yaml
role: "control-plane"  # For Kubernetes control plane
role: "worker"         # For worker nodes
role: "management"    # For management infrastructure
```

This helps organize and filter pools."

### 6. Sequential vs Random

[Screen: Strategy comparison]

**Narrator:** "Choose allocation strategy based on use case:
- **Sequential** - Better for predictable IPs, easier debugging
- **Random** - Better for security, harder to predict IPs

For most cases, sequential is fine and easier to manage."

---

## Real-World Example (11:00 - 12:00)

[Screen: Complete example]

**Narrator:** "Let me show you a complete real-world example - setting up IP pools for a Talos Kubernetes cluster..."

[Screen: Multiple pools]

**Narrator:** "For a Kubernetes cluster, you might have:
- Control plane pool - 3 IPs for control plane nodes
- Worker pool - 10+ IPs for worker nodes
- Management pool - IPs for management infrastructure

Each pool references its own prefix, and you allocate IPs as you add nodes."

[Screen: Git repository view]

**Narrator:** "All of this is in Git - you can see the complete infrastructure definition. When you need to add a node, you just create a new IPClaim, commit it, and the IP is automatically allocated. No spreadsheets, no manual tracking!"

---

## Summary (12:00 - 12:30)

[Screen: Summary slide]

**Narrator:** "In this video, we learned:
- How to create prefixes and IP pools
- How to allocate IP addresses
- How to monitor utilization
- Best practices for IP pool management

DCops makes IP address management as easy as managing any other infrastructure - through Git, with automatic reconciliation and conflict prevention."

[Screen: End screen]

**Narrator:** "Thanks for watching! Check out the documentation for more examples and advanced topics."

---

## Production Notes

### Visual Elements:
1. Terminal screen recordings
2. Animated architecture diagrams
3. NetBox UI showing prefixes and IPs
4. Git repository view
5. Utilization charts/graphs

### Key Moments to Highlight:
- First IP allocation (exciting moment)
- Sequential allocation demonstration
- Utilization monitoring
- Best practices (pause for emphasis)

### Voiceover Tips:
- Use enthusiasm for "exciting" moments
- Pause before best practices
- Speak clearly when showing commands
- Emphasize the "no spreadsheets" benefit

