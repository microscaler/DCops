# DCops Project Summary

## What We're Building

**DCops** (Data Center Operations) is a Kubernetes controller framework for managing bare-metal compute infrastructure through GitOps.

### Problem Statement

Managing bare-metal Raspberry Pi compute clusters requires:
- Deterministic PXE boot control (prevent infinite boot loops, accidental reinstalls)
- Automatic IP allocation (no manual tracking, no hardcoded addresses)
- Safe cluster rebuilds (destroy and recreate without fear)
- GitOps-native workflow (all intent in Git, auditable, reproducible)

Current solutions are either:
- Manual/ClickOps (error-prone, not reproducible)
- Over-engineered (full cloud platforms, too complex)
- Missing critical pieces (no PXE control, no IP automation)

### Solution

Build a minimal set of Kubernetes controllers that:
1. Reconcile Git-defined intent (YAML CRDs) to hardware
2. Use NetBox as authoritative IPAM/inventory database
3. Project intent to PXE services, DHCP servers, and MikroTik network devices (RouterOS/SwitchOS)
4. Enable safe, deterministic bare-metal cluster management

## Architecture

### Data Flow

```
┌─────────────────┐
│  Git (CRDs)     │  Source of truth
│  - BootIntent   │  All desired state
│  - IPClaim      │  Version controlled
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Controllers    │  Reconciliation engine
│  (Rust/kube-rs) │  Watch CRDs, reconcile state
│  - PXE Intent   │  DCops controllers
│  - IP Claim     │  Infrastructure layer
│  - CAPI         │  Cluster lifecycle (future)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  NetBox         │  Authoritative database
│  - IPAM         │  Inventory + IP allocations
│  - Devices      │  MAC addresses, interfaces
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Hardware       │  Execution layer
│  - PXE Server   │  Boot services
│  - DHCP         │  IP assignment (ISC Kea)
│  - Network      │  MikroTik RouterOS/SwitchOS (future)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Bare-metal     │  Target infrastructure
│  - Talos Linux  │  Kubernetes OS (CAPI-managed)
│  - Clusters     │  Compute workloads
│  - CAPI CRDs    │  Cluster, Machine, MachineSet
└─────────────────┘
```

### Key Components

#### 1. PXE Intent Controller

**Purpose:** Control what machines boot and when.

**CRDs:**
- `BootProfile` — Boot configuration (kernel, initrd, cmdline)
- `BootIntent` — MAC address → BootProfile mapping

**Integration:**
- PXE boot service (Pixiecore API mode or custom Rust PXE server)
- NetBox (for MAC address inventory)

**Prevents:**
- Infinite netboot loops
- Accidental reinstallation of live nodes
- Uncontrolled boot behavior

#### 2. IP Claim Controller

**Purpose:** Deterministic IP allocation without hardcoding.

**CRDs:**
- `IPPool` — IP address pool definition (references NetBox prefix)
- `IPClaim` — IP allocation request (device/interface → IP)

**Integration:**
- NetBox API (allocates from prefixes, writes back allocations)

**Removes:**
- Manual IP tracking
- Hardcoded IP addresses in Git
- Spreadsheet-based IP management

## Technology Decisions

### Core Stack

- **Language:** Rust
  - Type safety, performance, memory safety
  - `kube-rs` for Kubernetes controller framework
  
- **IPAM Backend:** NetBox
  - Authoritative inventory and IPAM
  - Rich API for automation
  - Industry-standard for bare-metal ops

- **PXE Service:** 
  - **Option A:** Pixiecore (Go) — Proven, maintained, API mode
  - **Option B:** Custom Rust PXE server — `dhcproto` + `async-tftp` + `axum`

- **OS:** Talos Linux
  - Kubernetes-native OS
  - Managed out-of-band via Talos API (gRPC)
  - Deterministic, immutable
  - API-managed configuration (no SSH, no shell)
  - Raspberry Pi support via `rpi_generic` platform
  - Image Factory for custom images (GPU support, config.txt customization)

- **Cluster Management:** Cluster API (CAPI) + Talos Providers
  - **CABPT** (Cluster API Bootstrap Provider for Talos) — Generates Talos machine configs
  - **CACPPT** (Cluster API Control Plane Provider for Talos) — Manages control plane lifecycle
  - Declarative cluster and machine management via CAPI CRDs
  - Talos nodes managed via `talosctl` / Talos API (out-of-band from Kubernetes)
  - Management cluster pattern (CAPI controllers run on management cluster, manage workload clusters)

- **Network Hardware:** MikroTik RouterOS/SwitchOS
  - RouterOS API (REST API) for routers/switches
  - SwitchOS API for managed switches
  - Target for VLAN, DHCP relay, and network configuration
  - Phase 2+ integration via VLAN Fabric Controller

### Future Components (Phase 2+)

- **Cluster API Integration:**
  - CAPI core + Talos providers (CABPT, CACPPT)
  - Declarative cluster lifecycle management
  - MachineSet creation and scaling
  - Talos machine configuration via CAPI bootstrap provider
  - Control plane management via CAPI control plane provider

- **DHCP Controller:** NetBox → ISC Kea reconciliation

- **RouterOS Controller:** NetBox → MikroTik RouterOS/SwitchOS API
  - RouterOS/SwitchOS device management
  - DHCP relay configuration (for PXE boot)
  - VLAN creation and management (Phase 2+)
  - Bridge VLAN table configuration (Phase 2+)
  - Network device state reconciliation
  - RouterOS REST API integration

- **NetBox Sync Controller:** Git CRDs → NetBox object sync

## Design Principles

1. **Git is source of truth** — All desired state in YAML CRDs
2. **NetBox is database, not control surface** — Controllers write, humans don't click
3. **Controllers are idempotent** — Small, focused, reconcile intent
4. **Hardware is projection target** — Never source of truth
5. **Management cluster isolation** — Controllers never manage themselves
6. **Phase discipline** — Build minimum to unlock next stage

## Phase 1 Scope (Locked)

### Must Have

1. ✅ PXE Intent Controller
   - BootProfile CRD
   - BootIntent CRD
   - PXE service integration
   - MAC address → boot config mapping

2. ✅ IP Claim Controller
   - IPPool CRD
   - IPClaim CRD
   - NetBox integration
   - Automatic IP allocation

### Explicitly Deferred

- RouterOS Controller (NetBox → MikroTik RouterOS/SwitchOS API)
  - Basic RouterOS device management
  - DHCP relay configuration
  - VLAN management (Phase 2+)
- DHCP Controller (NetBox → ISC Kea)
- Full NetBox GitOps Sync
- Interface-level network intent
- Multi-rack abstraction

See [ADR-001](../ADRs/ADR-001-Scope_and_Non-Goals.md) for detailed rationale.

## Success Criteria

Phase 1 is successful when:

1. ✅ Can boot a Raspberry Pi via PXE with Git-defined intent
2. ✅ Can allocate IPs automatically from NetBox pools
3. ✅ Can rebuild a cluster without manual intervention
4. ✅ All infrastructure state is auditable via Git history
5. ✅ No manual IP tracking or hardcoded addresses

## CAPI Integration Strategy

DCops controllers provide the **infrastructure layer** that CAPI needs to provision Talos machines:

1. **PXE Intent Controller** → Ensures machines boot correctly (prerequisite for CAPI)
2. **IP Claim Controller** → Allocates IPs for machines (CAPI needs IPs for Talos API access)
3. **Future: CAPI Infrastructure Provider** → Integrates DCops with CAPI machine lifecycle

### CAPI + Talos Workflow

```
1. CAPI creates Machine CRD
   ↓
2. DCops IP Claim Controller allocates IP from NetBox
   ↓
3. DCops PXE Intent Controller configures boot (Talos installer)
   ↓
4. Machine boots via PXE → Installs Talos
   ↓
5. CAPI Bootstrap Provider (CABPT) generates Talos config
   ↓
6. CAPI applies config via Talos API (talosctl / gRPC)
   ↓
7. Talos node joins Kubernetes cluster
   ↓
8. CAPI Control Plane Provider (CACPPT) manages control plane
```

### Key Integration Points

- **DCops provides:** PXE boot control, IP allocation, network infrastructure
- **CAPI provides:** Cluster lifecycle, machine sets, Talos config generation
- **Talos API provides:** Node configuration, cluster bootstrap, OS management

**Architecture:** Management cluster runs CAPI + DCops controllers, manages workload clusters running Talos.

## Raspberry Pi Considerations

DCops targets Raspberry Pi compute blades. Key considerations:

- **Hardware:** Raspberry Pi 4, Compute Module 4 (officially tested)
- **Installation:** PXE boot via DCops PXE Intent Controller
- **Images:** Talos Image Factory for custom images (GPU support, config.txt)
- **EEPROM:** One-time bootloader update required
- **GPU Support:** Optional `vc4` system extension with CMA size configuration
- **Boot Config:** config.txt customization via Image Factory schematics

See [Raspberry Pi + Talos Documentation](02_Raspberry_Pi_Talos.md) for detailed installation and configuration requirements.

See [RouterOS Controller Documentation](03_RouterOS_Controller.md) for RouterOS/SwitchOS integration details (Phase 2+).

## Next Steps

1. **PXE Intent Controller Implementation**
   - Define BootProfile/BootIntent CRDs
   - Support Image Factory schematic IDs for Raspberry Pi
   - Implement controller reconciliation loop
   - Integrate with PXE service (Pixiecore or custom)

2. **IP Claim Controller Implementation**
   - Define IPPool/IPClaim CRDs
   - Implement NetBox API client
   - Build allocation logic

3. **CAPI Integration Planning**
   - Evaluate CAPI infrastructure provider requirements
   - Design integration between DCops controllers and CAPI
   - Plan for CAPI + Talos provider installation

4. **Integration Testing**
   - Test with real Raspberry Pi hardware
   - Validate end-to-end boot flow
   - Verify IP allocation workflow
   - Test CAPI machine provisioning with DCops infrastructure
   - Validate PXE boot on Raspberry Pi 4

5. **Documentation**
   - Controller usage examples
   - NetBox setup guide
   - CAPI + Talos integration guide
   - Raspberry Pi installation guide
   - Troubleshooting guide

