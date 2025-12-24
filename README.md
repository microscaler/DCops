# DCops: Microscaler Infrastructure Fabric Controllers

> **Deterministic bare-metal infrastructure control for Microscaler systems.**
> Git-defined intent, reconciled safely into real hardware.

## What We're Building

A set of **Kubernetes controllers** that manage bare-metal compute infrastructure (Raspberry Pi compute blades for PriceWhisperer) through a GitOps workflow.

**Core Capabilities:**
- **Deterministic PXE boot** — Control what machines boot and when
- **Automatic IP allocation** — No manual IP tracking or hardcoded addresses
- **Safe cluster rebuilds** — Destroy and rebuild clusters without fear
- **GitOps-native** — All intent lives in Git, controllers reconcile to hardware

## Architecture

```
Git (YAML CRDs)
   ↓
Kubernetes Controllers (Rust / kube-rs)
   ↓
NetBox (IPAM / Inventory Database)
   ↓
PXE / DHCP / Network Devices
   ↓
Bare-metal Nodes (Talos Linux)
   ↓
Kubernetes Clusters (compute)
```

**Key Principle:** Git is source of truth. NetBox is the database. Controllers reconcile intent to hardware.

**CAPI Integration:** DCops provides infrastructure layer (PXE boot, IP allocation) that CAPI uses to provision Talos clusters. CAPI manages cluster lifecycle; DCops manages infrastructure prerequisites.

## Phase 1 Controllers

### 1. PXE Intent Controller

Controls what machines boot and when.

**CRDs:**
- `BootProfile` — Defines boot configurations (kernel, initrd, cmdline)
- `BootIntent` — Maps MAC addresses to boot profiles

**Integration:** PXE boot service (Pixiecore API or custom Rust PXE server)

**Prevents:** Infinite netboot loops, accidental reinstallation of live nodes

### 2. IP Claim Controller

Provides deterministic IP allocation without hardcoding.

**CRDs:**
- `IPPool` — Defines IP address pools (references NetBox prefixes)
- `IPClaim` — Requests an IP for a device/interface

**Integration:** NetBox API (allocates IPs, writes back allocations)

**Removes:** Human IP bookkeeping, spreadsheets, manual tracking

## Design Principles

1. **Git is the source of truth** — All desired state in YAML CRDs
2. **NetBox is a backend database** — Not a control surface, not configured manually
3. **Controllers are idempotent** — Small, focused, reconcile intent not workflows
4. **Hardware is projection targets** — Routers/DHCP never own state
5. **Management cluster isolation** — Controllers never run on nodes they manage
6. **Phase discipline** — Build only what unlocks the next stage

## Technology Stack

- **Language:** Rust
- **Kubernetes:** `kube-rs` for controller framework
- **IPAM:** NetBox (authoritative inventory + IPAM)
- **PXE:** Pixiecore (Go) or custom Rust PXE server (`dhcproto` + `async-tftp` + `axum`)
- **DHCP:** ISC Kea (optional, Phase 2+)
- **Network:** MikroTik RouterOS/SwitchOS (REST API, Phase 2+)
  - RouterOS API for routers/switches
  - SwitchOS API for managed switches
  - Target for RouterOS Controller
- **OS:** Talos Linux
  - Managed out-of-band via Talos API (gRPC)
  - API-managed configuration (no SSH, no shell)
  - Raspberry Pi support (Pi 4, CM4)
  - Image Factory for custom images
  
- **Cluster Management:** Cluster API (CAPI) + Talos Providers
  - **CABPT** (Bootstrap Provider) — Generates Talos machine configs
  - **CACPPT** (Control Plane Provider) — Manages control plane lifecycle
  - Declarative cluster management via CAPI CRDs
  - Management cluster pattern

## Repository Structure

```
DCops/
├─ controllers/          # Rust controllers (kube-rs)
│  ├─ pxe-intent/       # PXE Intent Controller
│  ├─ ip-claim/         # IP Claim Controller
│  └─ (future)
├─ crds/                # Kubernetes CRD definitions
│  ├─ bootprofile.yaml
│  ├─ bootintent.yaml
│  ├─ ippool.yaml
│  └─ ipclaim.yaml
├─ netbox/              # NetBox integration docs
│  ├─ conventions.md
│  └─ data-model.md
├─ docs/                # Architecture and design docs
│  ├─ 00_Summary.md
│  ├─ 01_CAPI_Integration.md
│  ├─ 02_Raspberry_Pi_Talos.md
│  ├─ 03_RouterOS_Controller.md
│  ├─ PRD.md
│  └─ ...
├─ ADRs/                # Architecture Decision Records
│  └─ ADR-001-Scope_and_Non-Goals.md
└─ README.md
```

## Out of Scope (Phase 1)

- **CAPI Integration** (deferred to Phase 2+)
  - CAPI infrastructure provider for DCops
  - Full CAPI + Talos provider integration
  - MachineSet creation and scaling via CAPI
  
- **RouterOS Controller** (deferred to Phase 2+)
  - MikroTik RouterOS/SwitchOS API integration
  - DHCP relay configuration
  - VLAN management and bridge configuration
  - Network device state reconciliation
  
- DHCP Controller (deferred to Phase 2+)
- Full NetBox GitOps Sync (deferred)
- Interface-level network intent (deferred)
- Multi-rack / multi-fabric abstraction (deferred)

See [ADR-001](ADRs/ADR-001-Scope_and_Non-Goals.md) for detailed scope decisions.

## Status

**Early development** — Phase 1 focus:

1. ✅ Architecture and scope defined
2. 🔄 PXE Intent Controller (in progress)
3. ✅ IP Claim Controller (implemented)
4. ⏳ Integration testing with hardware

## Contributing

This is internal Microscaler infrastructure. See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.
