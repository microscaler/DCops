# ADR-0001: Scope & Non-Goals

## Status

**Accepted** (Updated 2025-01-XX)

## Date

2025-12-23 (Original)  
2025-01-XX (Updated)

## Context

Microscaler is building **DCops** (Data Center Operations) — a set of Kubernetes controllers to manage **bare-metal compute infrastructure** (Raspberry Pi compute blades) used by PriceWhisperer and related systems.

This work sits at the intersection of:

* Bare-metal provisioning (PXE / Talos Linux)
* Network intent (IPAM, VLANs, DHCP)
* Kubernetes lifecycle management (Cluster API)
* GitOps workflows

**Target Hardware:**
* Raspberry Pi 4, Compute Module 4 (officially tested with Talos Linux)
* Talos Linux as the Kubernetes OS
* MikroTik RouterOS/SwitchOS for network infrastructure

**Technology Stack:**
* Rust controllers using `kube-rs`
* NetBox as authoritative IPAM/inventory database
* PXE service (Pixiecore API mode or custom Rust PXE server)
* Talos Linux managed via Talos API (gRPC, out-of-band)
* Cluster API (CAPI) + Talos providers for cluster lifecycle (Phase 2+)

Without explicit constraints, this space naturally expands into a full cloud control plane, which would introduce unacceptable scope, operational burden, and time-to-value risk.

This ADR exists to **lock scope early**, define **non-goals explicitly**, and protect the project from accidental overreach.

---

## Decision

The Microscaler Infrastructure Fabric repository will be **strictly limited** to a small, phased set of Kubernetes controllers whose sole purpose is to:

1. Enable **deterministic PXE boot** of bare-metal nodes
2. Enable **deterministic IP allocation** without human bookkeeping
3. Support **safe cluster rebuilds** on Talos Linux
4. Act as a control-plane substrate for PriceWhisperer compute

Everything else is explicitly **out of scope until proven necessary**.

**Relationship to Cluster API (CAPI):**

DCops controllers provide the **infrastructure layer** that CAPI needs:
- **PXE Intent Controller** → Ensures machines boot correctly (prerequisite for CAPI)
- **IP Claim Controller** → Allocates IPs for machines (CAPI needs IPs for Talos API access)

CAPI manages cluster lifecycle (machines, control plane, Talos config generation) but depends on DCops for infrastructure prerequisites. Direct CAPI integration (infrastructure provider) is Phase 2+.

---

## In Scope (Locked)

### Phase 1 (Mandatory)

These capabilities **must exist** for the project to be considered successful:

#### 1. PXE Intent Management

* Deterministic mapping of MAC addresses to boot behavior
* Explicit control over *what* a node boots and *when*
* Prevention of infinite netboot or accidental reinstallation
* Support for Raspberry Pi PXE boot via Talos installer
* Image Factory schematic ID support for custom Talos images

**Mechanism:**

* Kubernetes CRDs (`BootProfile`, `BootIntent`)
* PXE service integration:
  * **Option A:** Pixiecore (Go) — API mode, proven and maintained
  * **Option B:** Custom Rust PXE server — `dhcproto` + `async-tftp` + `axum`
* NetBox integration for MAC address inventory validation

---

#### 2. IP Allocation via Claims

* Deterministic IP assignment from defined pools
* No hard-coded IP addresses in Git
* No manual spreadsheets or human tracking
* Automatic conflict detection and resolution
* IP allocation state visible in CRD status

**Mechanism:**

* Kubernetes CRDs (`IPPool`, `IPClaim`)
* NetBox used as authoritative IPAM backend (REST API)
* Controllers allocate IPs from NetBox prefixes
* Allocations written back to NetBox as IPAddress objects
* Object tagging (`managed-by=gitops`, `owner=microscaler`) for ownership

---

### Foundational Assumptions

The following are assumed and fixed:

* **Git is the source of truth** for desired state (all intent in YAML CRDs)
* **NetBox is a backend database**, not a control surface (controllers write, humans don't click)
* **Talos Linux** is managed out-of-band via Talos API (gRPC), not via Kubernetes CRDs
* **Management infrastructure never manages itself** (controllers run on management cluster, manage workload clusters)
* **Raspberry Pi hardware** is the target platform (Pi 4, CM4 officially tested)
* **MikroTik RouterOS/SwitchOS** is the network hardware target (REST API integration)
* **Cluster API (CAPI)** will be integrated in Phase 2+ for cluster lifecycle management

---

## Explicit Non-Goals (Locked)

The following are **intentionally out of scope** for this repository at this time.

### 1. General-Purpose Cloud Platform

This project will **not** attempt to become:

* A generic bare-metal cloud
* A replacement for cloud providers
* A self-hosted OpenStack or equivalent

---

### 2. RouterOS Controller

The following are **not built initially**:

* RouterOS Controller for MikroTik RouterOS/SwitchOS device management
* RouterOS REST API integration
* DHCP relay configuration via RouterOS API
* Automatic VLAN creation on RouterOS/SwitchOS devices
* Bridge VLAN table configuration
* Dynamic port reassignment
* Multi-vendor switch abstraction
* Full L2/L3 fabric modeling

**Note:** MikroTik RouterOS/SwitchOS is the identified target for network device automation. RouterOS Controller will handle:
- RouterOS/SwitchOS device management
- DHCP relay configuration (for PXE boot)
- VLAN management and bridge configuration (Phase 2+)
- Network device state reconciliation

This is deferred to Phase 2+ but is a critical component for full network automation.

These may be revisited only when scale or failure patterns demand it.

---

### 3. DHCP as a First-Class Control Plane

* DHCP server management is deferred to Phase 2+
* ISC Kea is the identified target for DHCP reconciliation (NetBox → Kea)
* RouterOS DHCP relay configuration is deferred to RouterOS Controller (Phase 2+)
* No DHCP reconciliation controller is required for Phase 1
* Static or semi-static DHCP configuration is acceptable for Phase 1
* RouterOS may be manually configured as DHCP relay for Phase 1

---

### 4. NetBox as a GitOps Engine

NetBox will **not**:

* Be configured via YAML directly
* Act as a workflow engine
* Be the place where intent is authored

All writes to NetBox are performed by controllers, not humans.

---

### 5. Interface-Level Network Intent

The following are out of scope initially:

* Per-port access/trunk intent
* Interface-level VLAN enforcement
* Cable/path modeling

---

### 6. Multi-Tenancy & RBAC Complexity

* No tenant isolation beyond simple tagging
* No per-team fabric segmentation
* No external user-facing APIs

### 7. Cluster API Integration (Phase 1)

* CAPI infrastructure provider for DCops is deferred to Phase 2+
* Full CAPI + Talos provider integration (CABPT, CACPPT) is deferred
* MachineSet creation and scaling via CAPI is deferred
* **Note:** DCops controllers provide infrastructure layer (PXE, IPAM) that CAPI needs, but direct CAPI integration is Phase 2+

### 8. Talos OS Management via Kubernetes

* Talos Linux is **not** managed via Kubernetes CRDs
* Talos nodes are managed via Talos API (gRPC) out-of-band
* CAPI providers (CABPT, CACPPT) handle Talos config generation and application
* DCops does not directly manage Talos OS configuration

---

## Consequences

### Positive

* Project remains deliverable in weeks, not years
* Infrastructure work directly unlocks compute capacity
* Reduced operational and cognitive burden
* Clear criteria for when new controllers are justified

### Negative

* Some manual configuration remains in early phases
* Network automation is incomplete by design
* System is opinionated and non-generic

These trade-offs are **intentional and accepted**.

---

## Review & Change Policy

This ADR may only be amended if **at least one** of the following becomes true:

1. PriceWhisperer compute scale makes current constraints painful
2. Rebuild frequency exposes unacceptable human toil
3. Network failures are traced to missing automation
4. External adoption requires additional capabilities

All scope expansions require a **new ADR**, not edits to this one.

---

## Technology Decisions (Locked)

The following technology choices are fixed for Phase 1:

* **Language:** Rust with `kube-rs` for controller framework
* **IPAM Backend:** NetBox (REST API)
* **PXE Service:** Pixiecore (Go) API mode OR custom Rust PXE server
* **OS:** Talos Linux (managed via Talos API, not Kubernetes)
* **Hardware:** Raspberry Pi 4, Compute Module 4
* **Network:** MikroTik RouterOS/SwitchOS (REST API, Phase 2+)
* **Cluster Management:** Cluster API + Talos providers (Phase 2+)

## Summary

This project exists to **ship compute**, not to chase architectural purity.

We will:

* Build the minimum control plane required
* Defer everything else
* Expand only under real pressure

**DCops provides the infrastructure layer** (PXE boot, IP allocation) that enables safe, deterministic bare-metal cluster management. Cluster lifecycle management via CAPI is a Phase 2+ concern.

This ADR is the guardrail that makes that possible.
