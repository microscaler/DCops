# IP Resource Architecture

## Overview

The DCops controller manages IP addresses through **two different abstraction layers**:

1. **Kubernetes-Native IP Management** (`IPPool` + `IPClaim`)
   - Higher-level abstraction for Kubernetes workloads
   - Similar to Kubernetes `PersistentVolume`/`PersistentVolumeClaim` pattern
   - Allocates IPs from NetBox **prefixes**

2. **Direct NetBox Resource Management** (`NetBoxIPAddress` + `NetBoxIPRange`)
   - Direct 1:1 mapping to NetBox resources
   - For GitOps-style management of NetBox inventory
   - Can allocate IPs from NetBox **ranges** (for DHCP)

## Resource Comparison

### Kubernetes-Native IP Management

#### `IPPool`
- **Purpose**: Defines a pool of IPs from a NetBox prefix
- **References**: `NetBoxPrefix` CRD
- **Features**:
  - Allocation strategies (sequential, random)
  - Role-based pools (control-plane, worker, management)
  - Tracks available/allocated IPs
- **Use Case**: Kubernetes workload IP allocation
- **Example**: "Allocate IPs from prefix 192.168.1.0/24 for control-plane nodes"

#### `IPClaim`
- **Purpose**: Requests an IP allocation from an `IPPool`
- **References**: `IPPool` CRD
- **Features**:
  - Device/interface binding
  - Preferred IP hints
  - Automatic allocation from pool
- **Use Case**: "I need an IP for this device/interface"
- **Flow**: `IPClaim` → `IPPool` → `NetBoxPrefix` → NetBox API

### Direct NetBox Resource Management

#### `NetBoxIPAddress`
- **Purpose**: Direct management of a specific IP address in NetBox
- **References**: `NetBoxIPRange` (optional, for DHCP), `NetBoxTenant`, `NetBoxVLAN`
- **Features**:
  - Specific IP address management
  - DHCP IP tracking
  - Direct NetBox sync
  - Duplicate detection and remediation
- **Use Case**: "Track this specific IP address in NetBox"
- **Example**: "192.168.1.10/24 assigned to web-server-01"

#### `NetBoxIPRange`
- **Purpose**: Direct management of IP ranges in NetBox (for DHCP pools)
- **References**: `NetBoxTenant`, `NetBoxVRF` (optional)
- **Features**:
  - DHCP pool definition
  - Range-based IP allocation
  - Mark as utilized/populated
- **Use Case**: "Define a DHCP pool range in NetBox"
- **Example**: "192.168.1.100-200/24 DHCP pool"

## Functional Overlap

### Overlap Areas

1. **IP Allocation**:
   - `IPClaim` allocates from `NetBoxPrefix` (via `IPPool`)
   - `NetBoxIPAddress` can allocate from `NetBoxIPRange`
   - Both create IP addresses in NetBox, but from different sources

2. **NetBox Integration**:
   - Both eventually create `IPAddress` resources in NetBox
   - Both track allocation state

### Key Differences

| Feature | `IPPool`/`IPClaim` | `NetBoxIPAddress`/`NetBoxIPRange` |
|---------|-------------------|----------------------------------|
| **Abstraction Level** | Kubernetes-native (high-level) | Direct NetBox mapping (low-level) |
| **Source** | NetBox **Prefixes** | NetBox **Ranges** (for DHCP) or direct IPs |
| **Allocation** | Automatic from pool | Manual or from range |
| **Use Case** | Kubernetes workloads | NetBox inventory management |
| **Binding** | Device/interface via `IPClaim` | Direct IP address specification |
| **Strategy** | Sequential/Random allocation | Specific IP or range-based |

## When to Use Which

### Use `IPPool` + `IPClaim` When:
- ✅ Managing IPs for Kubernetes workloads (nodes, pods, services)
- ✅ Need automatic IP allocation from prefixes
- ✅ Want role-based IP pools (control-plane, worker, etc.)
- ✅ Need allocation strategies (sequential, random)
- ✅ Working with NetBox **prefixes**

### Use `NetBoxIPAddress` + `NetBoxIPRange` When:
- ✅ Managing NetBox inventory directly (GitOps style)
- ✅ Tracking specific IP addresses
- ✅ Working with DHCP pools (ranges)
- ✅ Need to track DHCP-assigned IPs
- ✅ Want direct control over NetBox resources
- ✅ Working with NetBox **ranges** or specific IPs

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    Kubernetes Cluster                         │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐         ┌──────────────┐                  │
│  │   IPClaim    │────────▶│   IPPool     │                  │
│  │  (Request)   │         │  (Pool Def)  │                  │
│  └──────────────┘         └──────┬───────┘                  │
│                                   │                           │
│                                   │ references                │
│                                   ▼                           │
│                          ┌─────────────────┐                 │
│                          │  NetBoxPrefix  │                 │
│                          │     (CRD)      │                 │
│                          └─────────────────┘                 │
│                                                               │
│  ┌──────────────────┐         ┌──────────────────┐        │
│  │ NetBoxIPAddress  │────────▶│  NetBoxIPRange   │        │
│  │   (Direct IP)    │         │  (DHCP Pool)      │        │
│  └──────────────────┘         └──────────────────┘        │
│                                                               │
└───────────────────────────┬───────────────────────────────────┘
                            │
                            │ All reconcile to
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                      NetBox API                              │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐         ┌──────────────┐                 │
│  │   Prefix     │         │    Range     │                 │
│  │  (192.168.1. │         │ (192.168.1.  │                 │
│  │   0/24)      │         │  100-200/24)  │                 │
│  └──────┬───────┘         └──────┬───────┘                 │
│         │                        │                           │
│         │ allocates from         │ allocates from            │
│         ▼                        ▼                           │
│  ┌──────────────────────────────────────────┐               │
│  │         IP Address (192.168.1.10/24)     │               │
│  └──────────────────────────────────────────┘               │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

## Summary

**They are NOT duplicates** - they serve different purposes:

- **`IPPool`/`IPClaim`**: Kubernetes-native IP management for workloads, allocates from **prefixes**
- **`NetBoxIPAddress`/`NetBoxIPRange`**: Direct NetBox inventory management, works with **ranges** (DHCP) or specific IPs

The overlap is intentional - it provides flexibility for different use cases:
- Kubernetes workloads → Use `IPPool`/`IPClaim`
- NetBox inventory/GitOps → Use `NetBoxIPAddress`/`NetBoxIPRange`
- DHCP management → Use `NetBoxIPRange` + `NetBoxIPAddress`

