# Product Requirements Document (PRD)
## DCops: Microscaler Infrastructure Fabric Controllers

**Version:** 1.0  
**Date:** 2025-01-XX  
**Status:** Draft

---

## Executive Summary

**DCops** (Data Center Operations) is a Kubernetes controller framework that provides deterministic, GitOps-native management of bare-metal compute infrastructure. The system enables safe, automated provisioning and lifecycle management of Raspberry Pi compute clusters running Talos Linux for PriceWhisperer workloads.

**Core Value Proposition:**
- Eliminate manual IP tracking and hardcoded addresses
- Prevent infinite PXE boot loops and accidental node reinstalls
- Enable safe cluster rebuilds without fear
- Provide full auditability through Git-based intent

**Target Users:** Infrastructure engineers managing bare-metal Kubernetes clusters

**Success Metric:** Ability to provision, scale, and rebuild Raspberry Pi clusters entirely through Git commits with zero manual intervention.

---

## Problem Statement

### Current State

Managing bare-metal Raspberry Pi compute clusters requires:

1. **Manual PXE Boot Control**
   - No deterministic control over what machines boot
   - Risk of infinite netboot loops
   - Accidental reinstallation of live nodes
   - No audit trail for boot decisions

2. **Manual IP Management**
   - IP addresses tracked in spreadsheets or memory
   - Hardcoded IPs in configuration files
   - No automatic allocation or conflict detection
   - Human error in IP assignment

3. **Fragile Cluster Rebuilds**
   - Fear of destroying working clusters
   - Manual intervention required for recovery
   - No reproducible rebuild process
   - Loss of configuration history

4. **ClickOps Infrastructure**
   - Network devices configured via web UI
   - No version control for network state
   - No rollback capability
   - Inconsistent configurations

### Impact

- **Operational Risk:** Manual errors cause outages
- **Time to Value:** Slow cluster provisioning and scaling
- **Maintenance Burden:** High cognitive load, difficult troubleshooting
- **Lack of Reproducibility:** Cannot reliably rebuild clusters

---

## Goals & Objectives

### Primary Goals

1. **Deterministic PXE Boot Control**
   - Explicit control over what machines boot and when
   - Prevent infinite boot loops
   - Prevent accidental reinstallation

2. **Automatic IP Allocation**
   - Zero manual IP tracking
   - No hardcoded addresses in Git
   - Automatic conflict detection and resolution

3. **Safe Cluster Rebuilds**
   - Destroy and recreate clusters without fear
   - Full reproducibility from Git
   - Automated recovery procedures

4. **GitOps-Native Workflow**
   - All infrastructure intent in Git
   - Full audit trail
   - Version-controlled infrastructure

### Success Criteria

Phase 1 is successful when:

1. ✅ Can boot a Raspberry Pi via PXE with Git-defined intent
2. ✅ Can allocate IPs automatically from NetBox pools
3. ✅ Can rebuild a cluster without manual intervention
4. ✅ All infrastructure state is auditable via Git history
5. ✅ No manual IP tracking or hardcoded addresses

---

## User Stories

### As an Infrastructure Engineer

**US-1: Provision New Cluster**
- **Given** I have Raspberry Pi hardware ready
- **When** I commit a BootIntent and IPClaim CRD to Git
- **Then** The cluster provisions automatically via PXE boot
- **And** IPs are allocated from NetBox pools
- **And** Talos Linux installs and joins Kubernetes cluster

**US-2: Scale Cluster**
- **Given** I have a running cluster
- **When** I create additional IPClaim and BootIntent CRDs
- **Then** New nodes boot and join the cluster automatically
- **And** IPs are allocated without conflicts

**US-3: Rebuild Cluster**
- **Given** I have a cluster that needs rebuilding
- **When** I delete and recreate the cluster CRDs
- **Then** The cluster rebuilds from Git-defined state
- **And** No manual intervention is required

**US-4: Prevent Accidental Reinstall**
- **Given** I have a live production cluster
- **When** Someone accidentally triggers a reinstall
- **Then** The PXE Intent Controller prevents the reinstall
- **And** The cluster continues operating normally

**US-5: Audit Infrastructure Changes**
- **Given** Infrastructure state changes over time
- **When** I review Git history
- **Then** I can see all infrastructure changes with full context
- **And** I can rollback to any previous state

---

## Functional Requirements

### FR-1: PXE Intent Controller

**Priority:** P0 (Must Have - Phase 1)

**Description:** Control what machines boot and when via PXE.

**Requirements:**
- **FR-1.1:** BootProfile CRD defines boot configurations
  - Kernel image URL
  - Initrd image URL(s)
  - Kernel command-line parameters
  - Boot message (optional)
  - Image Factory schematic ID support (for Raspberry Pi)

- **FR-1.2:** BootIntent CRD maps MAC addresses to boot profiles
  - MAC address → BootProfile reference
  - Lifecycle state (discovered, installing, installed, locked)
  - Prevents infinite boot loops
  - Prevents accidental reinstallation of live nodes

- **FR-1.3:** PXE service integration
  - Pixiecore API mode support
  - OR custom Rust PXE server integration
  - Dynamic boot configuration per MAC address
  - Boot state reconciliation

- **FR-1.4:** NetBox integration
  - Query MAC addresses from NetBox inventory
  - Validate device existence before boot configuration

**Acceptance Criteria:**
- Can create BootProfile and BootIntent CRDs in Git
- Controller reconciles intent to PXE service
- Machine boots according to BootIntent configuration
- Live nodes are protected from accidental reinstall
- Boot state is visible in CRD status

---

### FR-2: IP Claim Controller

**Priority:** P0 (Must Have - Phase 1)

**Description:** Provide deterministic IP allocation without hardcoding.

**Requirements:**
- **FR-2.1:** IPPool CRD defines IP address pools
  - References NetBox prefix
  - Defines pool scope (control-plane, worker, etc.)
  - Allocation strategy (sequential, random, etc.)

- **FR-2.2:** IPClaim CRD requests IP allocation
  - Device/interface reference
  - Pool reference
  - Optional: preferred IP (hint, not guarantee)

- **FR-2.3:** NetBox API integration
  - Allocate IPs from NetBox prefixes
  - Write allocations back to NetBox as IPAddress objects
  - Query existing allocations
  - Handle allocation conflicts

- **FR-2.4:** Status reporting
  - Allocated IP in CRD status
  - Allocation timestamp
  - NetBox object reference
  - Allocation state (pending, allocated, failed)

**Acceptance Criteria:**
- Can create IPPool and IPClaim CRDs in Git
- Controller allocates IPs from NetBox pools
- Allocations are written back to NetBox
- Allocated IPs are visible in CRD status
- No hardcoded IPs in Git repositories

---

### FR-3: NetBox Integration

**Priority:** P0 (Must Have - Phase 1)

**Description:** NetBox as authoritative IPAM and inventory database.

**Requirements:**
- **FR-3.1:** NetBox API client
  - REST API client implementation
  - Authentication (API token)
  - Error handling and retries
  - Rate limiting compliance

- **FR-3.2:** IPAM operations
  - Query prefixes
  - Allocate IP addresses
  - Create IPAddress objects
  - Query existing allocations

- **FR-3.3:** Inventory operations
  - Query devices by MAC address
  - Query interfaces
  - Validate device existence

- **FR-3.4:** Object tagging
  - Tag managed objects: `managed-by=gitops`, `owner=microscaler`
  - Only mutate tagged objects
  - Ignore untagged objects (human-managed)

**Acceptance Criteria:**
- Controllers can read/write NetBox objects
- All managed objects are properly tagged
- Controllers never mutate untagged objects
- NetBox operations are idempotent

---

### FR-4: Kubernetes CRD Definitions

**Priority:** P0 (Must Have - Phase 1)

**Description:** Custom resource definitions for infrastructure intent.

**Requirements:**
- **FR-4.1:** BootProfile CRD
  - Schema validation
  - Status subresource
  - Finalizers for cleanup

- **FR-4.2:** BootIntent CRD
  - Schema validation
  - Status subresource
  - Finalizers for cleanup
  - MAC address validation

- **FR-4.3:** IPPool CRD
  - Schema validation
  - Status subresource
  - Finalizers for cleanup
  - NetBox prefix reference validation

- **FR-4.4:** IPClaim CRD
  - Schema validation
  - Status subresource
  - Finalizers for cleanup
  - Device reference validation

**Acceptance Criteria:**
- All CRDs are installable via kubectl
- CRDs have proper OpenAPI schema
- CRDs support status subresources
- CRDs have appropriate validation rules

---

## Non-Functional Requirements

### NFR-1: Performance

- **NFR-1.1:** Controller reconciliation latency < 5 seconds for CRD changes
- **NFR-1.2:** IP allocation completes < 2 seconds
- **NFR-1.3:** PXE boot configuration updates < 3 seconds
- **NFR-1.4:** Support 100+ concurrent IP allocations

### NFR-2: Reliability

- **NFR-2.1:** Controllers are idempotent (safe to reconcile repeatedly)
- **NFR-2.2:** Controllers handle partial failures gracefully
- **NFR-2.3:** No data loss on controller restart
- **NFR-2.4:** Controllers recover from NetBox API failures

### NFR-3: Security

- **NFR-3.1:** NetBox API credentials stored in Kubernetes Secrets
- **NFR-3.2:** PXE service credentials stored in Kubernetes Secrets
- **NFR-3.3:** Controllers use RBAC for Kubernetes API access
- **NFR-3.4:** All API communications use TLS

### NFR-4: Observability

- **NFR-4.1:** Controllers emit Prometheus metrics
- **NFR-4.2:** Controllers emit structured logs
- **NFR-4.3:** CRD status fields provide operational visibility
- **NFR-4.4:** Controller events are recorded in Kubernetes

### NFR-5: Maintainability

- **NFR-5.1:** Code written in Rust with `kube-rs`
- **NFR-5.2:** Comprehensive test coverage (target: 80%)
- **NFR-5.3:** Clear error messages and troubleshooting guides
- **NFR-5.4:** Documentation for all CRDs and controllers

---

## Architecture Overview

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Management Kubernetes Cluster                          │
│                                                         │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Git (Source of Truth)                           │  │
│  │  - BootProfile CRDs                               │  │
│  │  - BootIntent CRDs                                │  │
│  │  - IPPool CRDs                                    │  │
│  │  - IPClaim CRDs                                   │  │
│  └──────────────────────────────────────────────────┘  │
│                         │                                │
│                         ▼                                │
│  ┌──────────────────────────────────────────────────┐  │
│  │  DCops Controllers (Rust / kube-rs)              │  │
│  │  - PXE Intent Controller                          │  │
│  │  - IP Claim Controller                            │  │
│  └──────────────────────────────────────────────────┘  │
│                         │                                │
│                         ▼                                │
│  ┌──────────────────────────────────────────────────┐  │
│  │  NetBox (Authoritative Database)                  │  │
│  │  - IPAM (Prefixes, IPs)                           │  │
│  │  - Inventory (Devices, Interfaces, MACs)         │  │
│  └──────────────────────────────────────────────────┘  │
│                         │                                │
│                         ▼                                │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Infrastructure Layer                             │  │
│  │  - PXE Server (Pixiecore or custom Rust)          │  │
│  │  - DHCP Server (ISC Kea, Phase 2+)                │  │
│  │  - Network Devices (MikroTik RouterOS/SwitchOS)  │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│  Workload Clusters (Talos Linux on Raspberry Pi)        │
│                                                         │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Control Plane Nodes                             │  │
│  │  - Talos Linux                                   │  │
│  │  - Kubernetes API Server                         │  │
│  │  - etcd                                          │  │
│  └──────────────────────────────────────────────────┘  │
│                                                         │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Worker Nodes                                    │  │
│  │  - Talos Linux                                   │  │
│  │  - Kubelet                                       │  │
│  │  - Workload Pods                                 │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Data Flow

1. **Intent Definition:** Engineer commits CRDs to Git
2. **Reconciliation:** Controllers watch CRDs, reconcile state
3. **NetBox Integration:** Controllers read/write NetBox objects
4. **Infrastructure Projection:** Controllers configure PXE/DHCP/network devices
5. **Machine Provisioning:** Hardware boots via PXE, installs Talos
6. **Cluster Formation:** Talos nodes join Kubernetes cluster

### Key Design Principles

1. **Git is source of truth** — All desired state in YAML CRDs
2. **NetBox is database, not control surface** — Controllers write, humans don't click
3. **Controllers are idempotent** — Small, focused, reconcile intent
4. **Hardware is projection target** — Never source of truth
5. **Management cluster isolation** — Controllers never manage themselves
6. **Phase discipline** — Build minimum to unlock next stage

---

## Technical Specifications

### Technology Stack

- **Language:** Rust
- **Kubernetes Framework:** `kube-rs`
- **IPAM Backend:** NetBox (REST API)
- **PXE Service:** 
  - Option A: Pixiecore (Go) — API mode
  - Option B: Custom Rust PXE server — `dhcproto` + `async-tftp` + `axum`
- **DHCP:** ISC Kea (Phase 2+)
- **Network:** MikroTik RouterOS/SwitchOS (REST API, Phase 2+)
- **OS:** Talos Linux (managed via Talos API, not Kubernetes)
- **Cluster Management:** Cluster API (CAPI) + Talos providers (Phase 2+)

### Controller Implementation

**PXE Intent Controller:**
- Watches BootProfile and BootIntent CRDs
- Integrates with PXE service API
- Queries NetBox for MAC address validation
- Manages boot lifecycle state

**IP Claim Controller:**
- Watches IPPool and IPClaim CRDs
- Allocates IPs from NetBox prefixes
- Writes allocations back to NetBox
- Updates CRD status with allocated IPs

### CRD Schemas

See individual CRD definitions in `crds/` directory:
- `bootprofile.yaml`
- `bootintent.yaml`
- `ippool.yaml`
- `ipclaim.yaml`

### Integration Points

**NetBox API:**
- REST API client
- Authentication via API token
- Object tagging for ownership
- Idempotent operations

**PXE Service:**
- Pixiecore API mode: `GET /v1/boot/<mac-address>`
- Custom Rust PXE: TBD based on implementation choice

**Talos Linux:**
- Out-of-band management via Talos API (gRPC)
- Not managed via Kubernetes CRDs
- CAPI providers handle Talos config generation

---

## Out of Scope (Phase 1)

The following are **explicitly deferred** to Phase 2+:

1. **CAPI Integration**
   - CAPI infrastructure provider for DCops
   - Full CAPI + Talos provider integration
   - MachineSet creation and scaling via CAPI

2. **RouterOS Controller**
   - MikroTik RouterOS/SwitchOS REST API integration
   - RouterOS device management
   - DHCP relay configuration (for PXE boot)
   - VLAN creation and management
   - Bridge VLAN table configuration
   - Network device state reconciliation

3. **DHCP Controller**
   - NetBox → ISC Kea reconciliation
   - DHCP pool management
   - DHCP reservation management

4. **Full NetBox GitOps Sync**
   - Git CRDs → NetBox object sync
   - NetBox object reconciliation from Git

5. **Interface-Level Network Intent**
   - Per-port access/trunk intent
   - Interface-level VLAN enforcement

6. **Multi-Rack / Multi-Fabric Abstraction**
   - Multi-site support
   - Fabric-level policies

See [ADR-001](../ADRs/ADR-001-Scope_and_Non-Goals.md) for detailed rationale.

---

## Implementation Phases

### Phase 1: Core Infrastructure (Current Focus)

**Duration:** 6-8 weeks

**Deliverables:**
1. PXE Intent Controller
   - BootProfile and BootIntent CRDs
   - Controller implementation
   - PXE service integration
   - NetBox MAC address validation

2. IP Claim Controller
   - IPPool and IPClaim CRDs
   - Controller implementation
   - NetBox API integration
   - IP allocation logic

3. Documentation
   - CRD reference documentation
   - Controller usage examples
   - NetBox setup guide
   - Troubleshooting guide

4. Testing
   - Unit tests (target: 80% coverage)
   - Integration tests with NetBox
   - End-to-end tests with Raspberry Pi hardware

**Success Criteria:** See "Success Criteria" section above.

### Phase 2: Enhanced Infrastructure (Future)

**Deliverables:**
- CAPI integration
- RouterOS Controller
  - RouterOS REST API client
  - DHCP relay configuration
  - VLAN management
  - Bridge configuration
- DHCP Controller
- Enhanced observability

**Timeline:** TBD based on Phase 1 learnings

---

## Dependencies

### External Dependencies

1. **NetBox**
   - Must be deployed and accessible
   - API token for authentication
   - Prefixes and devices configured

2. **PXE Service**
   - Pixiecore deployed OR custom Rust PXE server
   - Network access to booting machines
   - HTTP/TFTP services available

3. **Kubernetes Cluster**
   - Management cluster for running controllers
   - RBAC configured for controller permissions
   - Secrets for NetBox and PXE credentials

4. **Raspberry Pi Hardware**
   - Raspberry Pi 4 or Compute Module 4
   - EEPROM updated (one-time)
   - Network boot capable

### Internal Dependencies

1. **NetBox API Client Library**
   - Rust crate for NetBox REST API
   - Authentication and error handling

2. **PXE Service Client**
   - Pixiecore API client OR custom PXE server implementation

3. **kube-rs Framework**
   - Kubernetes controller framework
   - CRD definitions and validation

---

## Risks & Mitigations

### Risk 1: NetBox API Availability

**Risk:** NetBox API downtime prevents IP allocation and boot configuration.

**Mitigation:**
- Implement retry logic with exponential backoff
- Cache NetBox state locally
- Graceful degradation (controllers continue operating with cached state)
- Alerting on NetBox API failures

### Risk 2: PXE Boot Failures

**Risk:** Machines fail to boot via PXE, blocking cluster provisioning.

**Mitigation:**
- Comprehensive boot status monitoring
- Fallback to manual SD card installation
- Clear error messages and troubleshooting guides
- Boot state visibility in CRD status

### Risk 3: IP Allocation Conflicts

**Risk:** Multiple controllers allocate same IP, causing network conflicts.

**Mitigation:**
- NetBox provides atomic IP allocation
- Controllers check allocation status before use
- Conflict detection and resolution logic
- IP allocation state in CRD status

### Risk 4: Controller Bugs Cause Data Loss

**Risk:** Controller bugs corrupt NetBox state or cause IP conflicts.

**Mitigation:**
- Comprehensive test coverage
- Controllers only mutate tagged objects
- NetBox object tagging prevents accidental mutation
- Git history provides rollback capability
- Staged rollout with monitoring

### Risk 5: Talos Installation Failures

**Risk:** Talos installation fails on Raspberry Pi, blocking cluster formation.

**Mitigation:**
- Validate Talos image compatibility
- Support Image Factory for custom images
- Clear error reporting in BootIntent status
- Fallback to manual installation process
- Integration with Talos troubleshooting guides

---

## Success Metrics

### Phase 1 Metrics

1. **Provisioning Time**
   - Target: < 10 minutes from CRD commit to node Ready
   - Measure: Time from Git commit to Kubernetes node Ready

2. **IP Allocation Accuracy**
   - Target: 100% conflict-free allocations
   - Measure: Zero IP conflicts in production

3. **Boot Success Rate**
   - Target: > 95% successful PXE boots
   - Measure: BootIntent status success rate

4. **Zero Manual Intervention**
   - Target: 100% of clusters provisioned via Git
   - Measure: Zero manual IP assignments or boot configurations

5. **Git Auditability**
   - Target: 100% of infrastructure changes in Git
   - Measure: All CRD changes committed to Git

---

## References

- [DCops Summary](../docs/00_Summary.md)
- [CAPI Integration Guide](../docs/01_CAPI_Integration.md)
- [Raspberry Pi + Talos Guide](../docs/02_Raspberry_Pi_Talos.md)
- [ADR-001: Scope & Non-Goals](../ADRs/ADR-001-Scope_and_Non-Goals.md)
- [Talos Linux Documentation](https://docs.siderolabs.com/talos/v1.12/overview/what-is-talos)
- [Cluster API Documentation](https://cluster-api.sigs.k8s.io/)
- [NetBox Documentation](https://docs.netbox.dev/)

---

## Appendix

### Glossary

- **CAPI:** Cluster API — Kubernetes project for declarative cluster management
- **CABPT:** Cluster API Bootstrap Provider for Talos
- **CACPPT:** Cluster API Control Plane Provider for Talos
- **CRD:** Custom Resource Definition — Kubernetes extension mechanism
- **IPAM:** IP Address Management
- **PXE:** Preboot Execution Environment — Network boot protocol
- **Talos API:** Talos Linux gRPC API for node management

### Acronyms

- **DCops:** Data Center Operations
- **GitOps:** Git-based operational workflow
- **RBAC:** Role-Based Access Control
- **REST:** Representational State Transfer
- **TFTP:** Trivial File Transfer Protocol

---

**Document Status:** Draft for Review  
**Next Review Date:** TBD  
**Owner:** Microscaler Infrastructure Team

