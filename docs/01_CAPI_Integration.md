# CAPI + Talos Linux Integration

## Overview

DCops integrates with **Cluster API (CAPI)** to provide declarative management of Talos Linux clusters. CAPI manages cluster lifecycle; DCops provides the infrastructure layer (PXE boot, IP allocation) that CAPI needs.

## Talos Linux + CAPI Providers

Talos Linux has official CAPI providers maintained by Sidero Labs:

### 1. Cluster API Bootstrap Provider for Talos (CABPT)

**Purpose:** Generates Talos machine configurations during cluster bootstrap.

**What it does:**
- Generates Talos machine configs (YAML) from CAPI Machine resources
- Handles control plane and worker node configurations
- Manages Talos secrets (PKI, etcd, kubelet certificates)
- Supports Talos Linux v1.11.x+ and CAPI v1.x (v1beta1)

**Repository:** [siderolabs/cluster-api-bootstrap-provider-talos](https://github.com/siderolabs/cluster-api-bootstrap-provider-talos)

**Latest Release:** v0.6.10

### 2. Cluster API Control Plane Provider for Talos (CACPPT)

**Purpose:** Manages control plane components of Talos-based Kubernetes clusters.

**What it does:**
- Manages control plane node lifecycle
- Handles etcd cluster bootstrap and management
- Coordinates control plane upgrades
- Ensures control plane quorum and health

**Repository:** [siderolabs/cluster-api-control-plane-provider-talos](https://github.com/siderolabs/cluster-api-control-plane-provider-talos)

## Architecture

### Management Cluster Pattern

```
┌─────────────────────────────────────────┐
│  Management Cluster                     │
│  (Runs DCops + CAPI controllers)        │
│                                         │
│  ┌─────────────────────────────────┐   │
│  │  DCops Controllers              │   │
│  │  - PXE Intent Controller        │   │
│  │  - IP Claim Controller          │   │
│  └─────────────────────────────────┘   │
│                                         │
│  ┌─────────────────────────────────┐   │
│  │  CAPI Controllers                │   │
│  │  - CAPI Core                     │   │
│  │  - CABPT (Talos Bootstrap)      │   │
│  │  - CACPPT (Talos Control Plane) │   │
│  │  - Infrastructure Provider      │   │
│  └─────────────────────────────────┘   │
└─────────────────┬───────────────────────┘
                  │
                  │ Manages
                  ▼
┌─────────────────────────────────────────┐
│  Workload Clusters (Talos Linux)        │
│                                         │
│  ┌─────────────────────────────────┐   │
│  │  Control Plane Nodes            │   │
│  │  - Talos Linux                  │   │
│  │  - Kubernetes API Server        │   │
│  │  - etcd                         │   │
│  └─────────────────────────────────┘   │
│                                         │
│  ┌─────────────────────────────────┐   │
│  │  Worker Nodes                   │   │
│  │  - Talos Linux                  │   │
│  │  - Kubelet                      │   │
│  │  - Workload Pods                │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

**Key Principle:** Management cluster never manages itself. Controllers run on management cluster, manage workload clusters.

## Workflow: Machine Provisioning

### Step-by-Step Process

```
1. CAPI Creates Machine CRD
   └─ Infrastructure provider creates bare-metal machine
   
2. DCops IP Claim Controller
   └─ Allocates IP from NetBox pool
   └─ Writes IP to Machine status
   
3. DCops PXE Intent Controller
   └─ Configures PXE boot (Talos installer)
   └─ Maps MAC address → BootProfile
   
4. Machine Boots via PXE
   └─ Downloads Talos installer
   └─ Installs Talos Linux to disk
   └─ Reboots into Talos
   
5. CAPI Bootstrap Provider (CABPT)
   └─ Generates Talos machine config
   └─ Includes cluster join token, certificates
   
6. CAPI Applies Config via Talos API
   └─ Uses talosctl or Talos gRPC API
   └─ Applies machine configuration
   └─ Bootstraps Kubernetes components
   
7. Node Joins Cluster
   └─ Kubelet registers with API server
   └─ Node becomes Ready
   
8. CAPI Control Plane Provider (CACPPT)
   └─ Manages control plane lifecycle
   └─ Handles etcd cluster
   └─ Coordinates upgrades
```

## CAPI Resources

### Cluster

```yaml
apiVersion: cluster.x-k8s.io/v1beta1
kind: Cluster
metadata:
  name: pricewhisperer-cluster
spec:
  controlPlaneRef:
    apiVersion: controlplane.cluster.x-k8s.io/v1beta1
    kind: TalosControlPlane
    name: pricewhisperer-control-plane
  infrastructureRef:
    apiVersion: infrastructure.cluster.x-k8s.io/v1beta1
    kind: MetalCluster
    name: pricewhisperer-cluster
```

### TalosControlPlane (CACPPT)

```yaml
apiVersion: controlplane.cluster.x-k8s.io/v1beta1
kind: TalosControlPlane
metadata:
  name: pricewhisperer-control-plane
spec:
  replicas: 3
  version: v1.29.0
  infrastructureTemplate:
    apiVersion: infrastructure.cluster.x-k8s.io/v1beta1
    kind: MetalMachineTemplate
    name: pricewhisperer-control-plane-template
```

### Machine / MachineSet

```yaml
apiVersion: cluster.x-k8s.io/v1beta1
kind: Machine
metadata:
  name: pricewhisperer-worker-0
spec:
  clusterName: pricewhisperer-cluster
  version: v1.29.0
  bootstrap:
    configRef:
      apiVersion: bootstrap.cluster.x-k8s.io/v1beta1
      kind: TalosConfigTemplate
      name: pricewhisperer-worker-template
  infrastructureRef:
    apiVersion: infrastructure.cluster.x-k8s.io/v1beta1
    kind: MetalMachine
    name: pricewhisperer-worker-0
```

### TalosConfig (CABPT)

```yaml
apiVersion: bootstrap.cluster.x-k8s.io/v1beta1
kind: TalosConfig
metadata:
  name: pricewhisperer-worker-0
spec:
  # CABPT generates Talos machine config from this
  # Includes cluster join info, certificates, etc.
```

## DCops Integration Points

### 1. IP Allocation for CAPI Machines

**DCops IP Claim Controller** allocates IPs that CAPI needs:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: IPClaim
metadata:
  name: pricewhisperer-worker-0-ip
spec:
  poolRef:
    name: worker-pool
  deviceRef:
    name: pricewhisperer-worker-0
status:
  ip: 192.168.1.100
  # CAPI reads this IP to configure Talos API access
```

### 2. PXE Boot Configuration

**DCops PXE Intent Controller** ensures machines boot correctly:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: BootIntent
metadata:
  name: pricewhisperer-worker-0-boot
spec:
  macAddress: "aa:bb:cc:dd:ee:ff"
  profileRef:
    name: talos-installer
status:
  bootConfigured: true
  # PXE service uses this to boot machine
```

### 3. Future: CAPI Infrastructure Provider

A future DCops component could act as a CAPI infrastructure provider:

- Watches CAPI Machine resources
- Creates/updates DCops IPClaim resources
- Creates/updates DCops BootIntent resources
- Reports machine status back to CAPI

## Talos API Management

### Talos API (gRPC)

Talos nodes expose a gRPC API for configuration management:

- **No SSH** — All management via API
- **No shell** — Immutable, minimal OS
- **Declarative** — Apply config, Talos reconciles state
- **Mutually authenticated** — mTLS for security

### talosctl CLI

CAPI providers use `talosctl` (or Talos gRPC client) to:

- Apply machine configurations
- Bootstrap etcd cluster
- Manage certificates
- Query node state
- Perform upgrades

**Example:**
```bash
talosctl apply-config \
  --insecure \
  --nodes 192.168.1.100 \
  --file machine-config.yaml
```

## Key Design Decisions

### 1. Management Cluster Isolation

- CAPI controllers run on management cluster
- Never run on workload clusters they manage
- Prevents self-management issues
- Enables pivot/move operations

### 2. Out-of-Band Talos Management

- Talos nodes managed via Talos API (not Kubernetes)
- CAPI generates configs, applies via Talos API
- Talos never depends on Kubernetes for its own health
- Prevents circular dependencies

### 3. Infrastructure Layer Separation

- **DCops:** Infrastructure (PXE, IPAM, network)
- **CAPI:** Cluster lifecycle (machines, control plane)
- **Talos:** OS and Kubernetes runtime
- Clear separation of concerns

## References

- [Talos Linux Documentation](https://docs.siderolabs.com/talos/v1.12/overview/what-is-talos)
- [Cluster API Documentation](https://cluster-api.sigs.k8s.io/)
- [CABPT Repository](https://github.com/siderolabs/cluster-api-bootstrap-provider-talos)
- [CACPPT Repository](https://github.com/siderolabs/cluster-api-control-plane-provider-talos)
- [Talos API Reference](https://www.talos.dev/latest/reference/api/)

