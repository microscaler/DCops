# DHCP Pod Testing Options for Kind

This document explores options for testing DHCP IP address assignment to Kubernetes pods/devices in a Kind cluster, with integration to NetBox for IPAM tracking.

## NetBox ↔ ISC Kea Integration Pattern

**Important**: Kea does NOT directly query or use NetBox. The integration works as follows:

```
┌─────────────┐         ┌──────────────────┐         ┌─────────────┐
│ Git (CRDs)  │ ──────> │ NetBox Controller│ ──────> │   NetBox    │
│             │         │                  │         │  (Database) │
└─────────────┘         └──────────────────┘         └─────────────┘
                                                              │
                                                              │ (reads)
                                                              ▼
                                                       ┌──────────────┐
                                                       │ DHCP Controller│ (Phase 2+)
                                                       │ (NetBox → Kea) │
                                                       └──────────────┘
                                                              │
                                                              │ (configures via REST API)
                                                              ▼
                                                       ┌─────────────┐
                                                       │  ISC Kea    │
                                                       │ DHCP Server │
                                                       └─────────────┘
                                                              │
                                                              │ (serves DHCP)
                                                              ▼
                                                       ┌─────────────┐
                                                       │  RouterOS   │
                                                       │ (DHCP Relay)│
                                                       └─────────────┘
```

### How It Works

1. **NetBox is the Source of Truth**
   - NetBox stores IPAM data (prefixes, IP ranges, IP addresses, interfaces)
   - NetBox Controller reconciles Git CRDs → NetBox (already implemented)
   - NetBox acts as the authoritative IPAM database

2. **DHCP Controller (Phase 2+, not yet implemented)**
   - **Watches Kubernetes CRDs directly** (NetBoxPrefix, NetBoxIPRange, NetBoxIPAddress)
   - **Kubernetes-native event-driven sync**: Watches CRD status changes via Kubernetes watch API
   - **No polling loops**: Reacts instantly to CRD status updates from NetBox reconcilers
   - **Two sync modes**:
     - **Full sync at startup**: Overwrites current Kea configuration with all CRDs
     - **Event-driven sync**: Reacts to CRD status changes (Created/Updated/Deleted) in real-time
   - **Notification mechanism**: NetBox reconcilers update CRD status → DHCP Controller watches CRDs → instant Kea update
   - Translates CRD spec/status data to Kea configuration format:
     - **NetBoxPrefix** (CRD) → Kea subnet configuration
     - **NetBoxIPRange** (CRD) → Kea pool (within subnet)
     - **NetBoxIPAddress** (CRD with MAC/interface) → Kea host reservation
   - Pushes configuration to Kea via **Kea Control Agent REST API** (port 8000)
   - Uses Kea's `config-set` command to update runtime configuration
   - Kea receives configuration and serves DHCP accordingly
   
   **Advantages over NetBox webhooks**:
   - ✅ **No external webhook configuration** - uses Kubernetes watch API
   - ✅ **Instant updates** - reacts to CRD status changes immediately
   - ✅ **No polling** - event-driven, no polling loops
   - ✅ **Kubernetes-native** - leverages existing CRD infrastructure
   - ✅ **State already available** - NetBox reconcilers already have full CR state
   - ✅ **Better observability** - can use Kubernetes events and CRD status
   
   **Reference Implementation**: The [`netbox-kea-dhcp`](https://github.com/francoismdj/netbox-kea-dhcp) project demonstrates the NetBox → Kea mapping pattern, but uses NetBox webhooks. Our implementation will be more efficient by watching Kubernetes CRDs directly.

3. **ISC Kea**
   - Does NOT query NetBox directly
   - Receives configuration from DHCP Controller via REST API
   - Serves DHCP based on the configuration it received
   - Provides REST API for configuration management (Kea Control Agent)

4. **RouterOS (Network Device)**
   - Acts as DHCP relay
   - Forwards DHCP requests from devices to Kea
   - Does NOT run DHCP server itself

### Key Points

- **NetBox** = Source of truth (IPAM database)
- **DHCP Controller** = Kubernetes controller that syncs NetBox → Kea (Phase 2+)
- **Kea** = DHCP server that receives configuration (doesn't query NetBox)
- **RouterOS** = Network device that relays DHCP requests to Kea

### NetBox → Kea Data Mapping

The DHCP Controller translates NetBox objects to Kea configuration:

| NetBox Object | Kea Configuration | Mapping Details |
|--------------|-------------------|-----------------|
| **NetBox Prefix** | Kea Subnet | Prefix CIDR becomes subnet prefix, VLAN associations map to subnet options |
| **NetBox IP Range** | Kea Pool | Range start/end becomes pool range, status must be `dhcp` |
| **NetBox IP Address** | Kea Reservation | MAC address from assigned interface (or custom field) maps to reservation hardware address |

**Example Mapping**:
- NetBox Prefix: `192.168.1.0/24` → Kea Subnet: `192.168.1.0/24`
- NetBox IP Range: `192.168.1.100-200/24` (status: `dhcp`) → Kea Pool: `192.168.1.100-192.168.1.200`
- NetBox IP Address: `192.168.1.100/24` (assigned to interface with MAC `aa:bb:cc:dd:ee:ff`) → Kea Reservation: IP `192.168.1.100` for MAC `aa:bb:cc:dd:ee:ff`

### DHCP Controller Notification Mechanism

The DHCP Controller uses **Kubernetes-native event-driven sync** - no NetBox webhooks required:

**How It Works**:
1. **NetBox Reconciler** updates CRD status after reconciliation (e.g., `status.state: Created`)
2. **DHCP Controller** watches CRDs via Kubernetes watch API (using `kube-runtime::Controller`)
3. **On CRD status change**, DHCP Controller:
   - Reads CRD spec and status (all state already available in CRD)
   - Translates CRD data to Kea configuration
   - Updates Kea via Control Agent API
   - **No polling** - instant reaction to status changes

**CRD Watch Pattern**:
```rust
// DHCP Controller watches these CRDs:
- NetBoxPrefix (when status.state changes to Created/Updated)
- NetBoxIPRange (when status.state changes to Created/Updated, and spec.status == "dhcp")
- NetBoxIPAddress (when status.state changes to Created/Updated, and spec.status == Dhcp)
```

**Filtering Logic**:
- Only sync IP Ranges where `spec.status == "dhcp"` (DHCP-enabled ranges)
- Only sync IP Addresses where `spec.status == Dhcp` (DHCP-managed IPs)
- Optionally filter Prefixes by custom field or annotation (e.g., `dhcp-enabled: "true"`)

**Benefits**:
- ✅ **No external dependencies** - no NetBox webhook configuration needed
- ✅ **Instant updates** - reacts to CRD status changes immediately (no webhook delay)
- ✅ **No polling loops** - event-driven via Kubernetes watch API
- ✅ **State already available** - NetBox reconcilers populate CRD spec/status with all needed data
- ✅ **Better observability** - can use Kubernetes events and CRD status for debugging

**Note**: The [`netbox-kea-dhcp`](https://github.com/francoismdj/netbox-kea-dhcp) reference implementation uses NetBox webhooks, but our Kubernetes-native approach is more efficient and eliminates the need for external webhook configuration.

### Kea Control Agent API

The DHCP Controller uses Kea's Control Agent REST API to configure Kea:

**Key API Endpoints**:
- `POST /` - Execute Kea commands (e.g., `config-set`, `config-get`, `config-test`)
- Commands are JSON-RPC format: `{"command": "config-set", "service": ["dhcp4"], "arguments": {...}}`

**Configuration Flow**:
1. Controller gets current Kea configuration via `config-get`
2. Controller modifies configuration (adds/updates/removes subnets/pools/reservations)
3. Controller validates configuration via `config-test`
4. Controller applies configuration via `config-set`
5. Kea reloads configuration and serves DHCP

**Note**: Kea open-source commands require full configuration replacement (limitation of free Kea). ISC paid subscription provides more granular update commands.

### For Testing

In our tests, we can simulate the DHCP Controller behavior:
1. Create/update NetBox CRDs (NetBoxPrefix, NetBoxIPRange, NetBoxIPAddress)
2. Wait for NetBox reconciler to update CRD status (simulates reconciliation)
3. Simulate DHCP Controller watching CRD status changes
4. Read CRD spec/status (all state available in CRD, no NetBox API call needed)
5. Translate CRD data to Kea configuration format (subnets, pools, reservations)
6. Configure Kea via Control Agent REST API (`config-set` command)
7. Verify Kea serves DHCP correctly based on CRD data

This allows us to test the NetBox → Kea integration pattern without waiting for the DHCP Controller to be implemented.

**Key Difference from Reference**: The [`netbox-kea-dhcp`](https://github.com/francoismdj/netbox-kea-dhcp) Python project uses NetBox webhooks and queries NetBox API. Our implementation will be more efficient by:
- Watching Kubernetes CRDs directly (no webhooks)
- Reading state from CRD spec/status (no NetBox API calls needed)
- Reacting instantly to CRD status changes (no polling)

## Overview

**Key Insight**: Using [`dhcpm`](https://lib.rs/crates/dhcpm), we can test DHCP functionality entirely in test containers without needing Kubernetes pods, CNI plugins, or complex networking setup.

**For Pod Testing**: If you need to test DHCP in actual Kubernetes pods (e.g., for CNI integration), see the "Alternative: dhcpm in Pod Init Container" section below.

## ⭐ Recommended: dhcpm in Test Containers (Simplest)

**The simplest approach** uses [`dhcpm`](https://lib.rs/crates/dhcpm), a Rust CLI tool for mocking DHCP messages. Since `dhcpm` can send DHCP messages without needing actual network interfaces configured, we can test DHCP functionality entirely in test containers - no pods needed!

### Architecture

```
Test Container (testcontainers-rs)
├── dhcpm CLI tool
│   ├── Sends DHCP DISCOVER/REQUEST messages
│   ├── Receives DHCP OFFER/ACK responses
│   └── Parses assigned IP address
└── Integration with NetBox Controller
    └── Verifies NetBoxIPAddress CRD status updates
```

### Why Test Containers?

- ✅ **No Kubernetes complexity** - test DHCP without pods, CNI, or Multus
- ✅ **Faster iteration** - spin up test containers quickly
- ✅ **Isolated testing** - each test gets a clean environment
- ✅ **CI/CD friendly** - works in any environment with Docker
- ✅ **Real DHCP server** - uses ISC Kea (production standard) in container
- ✅ **NetBox integration** - can test against real NetBox instance or mock

### Test Container Setup

Using `testcontainers-rs` for unit/integration tests:

```rust
// tests/dhcp_integration_test.rs
use testcontainers::{clients, images, Container, Docker};

#[tokio::test]
async fn test_dhcp_allocation_with_netbox() {
    // 1. Start NetBox container
    let netbox = start_netbox_container().await;
    
    // 2. Start DHCP server container (e.g., ISC Kea)
    let dhcp_server = start_dhcp_server_container().await;
    
    // 3. Start test container with dhcpm
    let test_container = start_dhcpm_test_container().await;
    
    // 4. Run dhcpm to request IP
    let output = test_container.exec("dhcpm", vec![
        "255.255.255.255",
        "-i", "eth0",
        "dora",
        "--output", "json"
    ]).await;
    
    // 5. Parse IP from dhcpm output
    let ip = parse_ip_from_dhcpm_output(&output);
    
    // 6. Verify NetBoxIPAddress CRD was updated
    let crd = get_netbox_ip_address_crd("test-dhcp-ip").await;
    assert_eq!(crd.status.address, Some(ip));
}
```

### Kind-Based Testing

For integration tests that need a real Kubernetes cluster:

```rust
// tests/dhcp_kind_integration_test.rs
use kube::Client;

#[tokio::test]
#[ignore] // Requires Kind cluster - run with: cargo test -- --ignored
async fn test_dhcp_with_kind_cluster() {
    // Check if Kind cluster is available
    if std::env::var("E2E_KIND").is_err() {
        println!("Skipping: set E2E_KIND=1 to enable Kind e2e test");
        return;
    }
    
    // Assumes Kind cluster is running
    let client = Client::try_default().await.unwrap();
    
    // 1. Create NetBoxIPAddress CRD in Kind cluster
    create_netbox_ip_address_crd(&client, "test-dhcp-ip").await;
    
    // 2. Create test pod with dhcpm init container
    create_dhcp_test_pod(&client).await;
    
    // 3. Wait for init container to complete
    wait_for_pod_ready(&client, "dhcp-test-pod").await;
    
    // 4. Verify NetBoxIPAddress CRD status updated
    let crd = get_netbox_ip_address_crd(&client, "test-dhcp-ip").await;
    assert!(crd.status.address.is_some());
}
```

### Test Scenarios

**1. Random DHCP Allocation Test**
```rust
#[tokio::test]
async fn test_random_dhcp_allocation() {
    // Create NetBoxIPAddress CRD without address
    create_netbox_ip_address_crd("test-random", None).await;
    
    // Run dhcpm to request IP
    let ip = run_dhcpm_discover().await;
    
    // Verify CRD status updated
    let crd = get_netbox_ip_address_crd("test-random").await;
    assert_eq!(crd.status.address, Some(ip));
    assert_eq!(crd.status.state, ResourceState::Created);
}
```

**2. Static DHCP Reservation Test**
```rust
#[tokio::test]
async fn test_static_dhcp_reservation() {
    let mac = "aa:bb:cc:dd:ee:ff";
    
    // Create NetBoxIPAddress CRD with MAC address
    create_netbox_ip_address_crd("test-static", Some(mac)).await;
    
    // Run dhcpm with specific MAC
    let ip = run_dhcpm_with_mac(mac).await;
    
    // Verify correct IP assigned (from DHCP server reservation)
    assert_eq!(ip, "192.168.1.100/24");
}
```

**3. NetBox Controller Integration Test**
```rust
#[tokio::test]
async fn test_netbox_controller_dhcp_sync() {
    // 1. DHCP server assigns IP to test container
    let ip = run_dhcpm_discover().await;
    
    // 2. Create NetBoxIPAddress CRD
    create_netbox_ip_address_crd("test-sync", None).await;
    
    // 3. NetBox controller should reconcile and update NetBox
    wait_for_reconciliation().await;
    
    // 4. Verify IP exists in NetBox
    let netbox_ip = get_netbox_ip_address(ip).await;
    assert!(netbox_ip.is_some());
}
```

### Benefits Over Pod-Based Testing

- ✅ **Simpler** - no Kubernetes API, no CNI, no Multus
- ✅ **Faster** - containers start in seconds vs minutes for pods
- ✅ **More isolated** - each test is completely independent
- ✅ **Easier debugging** - can exec into containers, view logs easily
- ✅ **CI/CD ready** - works in GitHub Actions, GitLab CI, etc.
- ✅ **Real DHCP** - tests against ISC Kea (production DHCP server for RouterOS integration)

### When to Use Pod-Based Testing

Pod-based testing is still useful for:
- Testing actual CNI integration
- Testing Multus multi-interface scenarios
- End-to-end Kubernetes workflow testing
- Testing pod networking policies

But for **DHCP functionality testing**, test containers with `dhcpm` are the better choice.

## Alternative: dhcpm in Pod Init Container

If you need to test DHCP in actual Kubernetes pods (e.g., for CNI integration testing), you can use `dhcpm` in an init container:

### Implementation

1. **Create dhcpm-based init container image**
   ```dockerfile
   FROM rust:alpine AS builder
   RUN cargo install dhcpm --locked
   
   FROM alpine:latest
   RUN apk add --no-cache iproute2 kubectl
   COPY --from=builder /usr/local/cargo/bin/dhcpm /usr/local/bin/dhcpm
   COPY dhcp-init.sh /usr/local/bin/
   RUN chmod +x /usr/local/bin/dhcp-init.sh
   ```

2. **DHCP Init Script** (`dhcp-init.sh`)
   ```bash
   #!/bin/sh
   set -e
   
   INTERFACE=${INTERFACE:-net1}
   NETBOX_IP_CRD=${NETBOX_IP_CRD:-pod-dhcp-ip}
   NAMESPACE=${NAMESPACE:-default}
   
   # Wait for interface to be available
   while ! ip link show $INTERFACE >/dev/null 2>&1; do
     echo "Waiting for interface $INTERFACE..."
     sleep 1
   done
   
   # Get interface MAC address
   MAC=$(ip link show $INTERFACE | grep -oP 'link/ether \K[^ ]+')
   echo "Interface $INTERFACE MAC: $MAC"
   
   # Send DHCP DISCOVER and REQUEST (DORA sequence)
   # dhcpm will bind to the interface and send DHCP messages
   RESPONSE=$(dhcpm 255.255.255.255 -i $INTERFACE dora --chaddr "$MAC" --output json)
   
   # Parse IP from response (parse JSON)
   IP=$(echo "$RESPONSE" | jq -r '.yiaddr // empty')
   
   if [ -z "$IP" ]; then
     echo "ERROR: No IP address received from DHCP"
     exit 1
   fi
   
   echo "DHCP assigned IP: $IP"
   
   # Configure interface with assigned IP
   # Note: This requires NET_ADMIN capability
   ip addr add "$IP" dev $INTERFACE
   ip link set $INTERFACE up
   
   # Update NetBoxIPAddress CRD status
   kubectl patch netboxipaddress $NETBOX_IP_CRD \
     -n $NAMESPACE \
     --type=merge \
     -p "{\"status\":{\"address\":\"$IP\",\"state\":\"Created\"}}"
   
   echo "Updated NetBoxIPAddress CRD with IP: $IP"
   ```

3. **Pod with DHCP Init Container**
   ```yaml
   apiVersion: v1
   kind: Pod
   metadata:
     name: dhcp-test-pod
     annotations:
       k8s.v1.cni.cncf.io/networks: netbox-dhcp-network
   spec:
     initContainers:
     - name: dhcp-init
       image: netbox-dhcp-init:latest
       securityContext:
         capabilities:
           add: ["NET_ADMIN", "NET_RAW"]
       env:
       - name: INTERFACE
         value: "net1"
       - name: NETBOX_IP_CRD
         value: "pod-dhcp-ip"
       - name: NAMESPACE
         valueFrom:
           fieldRef:
             fieldPath: metadata.namespace
       command: ["/usr/local/bin/dhcp-init.sh"]
     containers:
     - name: app
       image: busybox
       command: ["sleep", "3600"]
   ```

4. **NetBoxIPAddress CRD** (already exists)
   ```yaml
   apiVersion: dcops.microscaler.io/v1alpha1
   kind: NetBoxIPAddress
   metadata:
     name: pod-dhcp-ip
   spec:
     status: dhcp
     ipRange:
       apiGroup: dcops.microscaler.io
       kind: NetBoxIPRange
       name: dhcp-pool-range
     tenant:
       apiGroup: dcops.microscaler.io
       kind: NetBoxTenant
       name: datacenter-tenant
   ```

### Pros
- ✅ **Simplest approach** - no custom CNI development
- ✅ Uses existing Rust tooling (`dhcpm`)
- ✅ Works with existing `NetBoxIPAddress` CRD
- ✅ Easy to test and debug
- ✅ Can be packaged as standard container image
- ✅ Supports both random and static DHCP (via MAC address)

### Cons
- ⚠️ Requires Multus CNI for multi-interface (or host network)
- ⚠️ Requires `NET_ADMIN` capability for interface configuration
- ⚠️ Init container must have kubectl access (or use Kubernetes API client)

### Alternative: Rust-based Init Container

Instead of shell script, create a Rust binary that:
1. Uses `dhcpm` library directly (or `dhcproto` crate)
2. Sends DHCP messages
3. Parses responses
4. Updates NetBoxIPAddress CRD via Kubernetes client

This would be more robust and easier to maintain:

```rust
// dhcp-init-rs/src/main.rs
use dhcproto::v4::{Message, MessageType, OptionCode};
use kube::Api;
use kube::Client;
use std::net::UdpSocket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Send DHCP DISCOVER
    // 2. Receive DHCP OFFER
    // 3. Send DHCP REQUEST
    // 4. Receive DHCP ACK
    // 5. Parse IP from ACK
    // 6. Configure interface
    // 7. Update NetBoxIPAddress CRD
    Ok(())
}
```

### Testing Scenarios

**Random DHCP Allocation:**
```yaml
# NetBoxIPAddress CRD - no address specified
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxIPAddress
metadata:
  name: pod-random-dhcp-ip
spec:
  status: dhcp
  ipRange:
    apiGroup: dcops.microscaler.io
    kind: NetBoxIPRange
    name: dhcp-pool-range
```

**Static DHCP Reservation:**
```yaml
# NetBoxIPAddress CRD - MAC address specified
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxIPAddress
metadata:
  name: pod-static-dhcp-ip
spec:
  address: "192.168.1.100/24"
  status: dhcp
  macAddress: "aa:bb:cc:dd:ee:ff"  # Pod interface MAC
```

## Option 2: Multus CNI (For Multi-Interface Support)

**Multus CNI** is the standard Kubernetes solution for multi-interface pods. It allows pods to have multiple network interfaces, each managed by different CNI plugins.

### Architecture

```
Pod
├── eth0 (Primary) - Managed by kindnetd (default)
│   └── IP: 10.204.x.x/16 (from Kind pod subnet)
└── net1 (Secondary) - Managed by Multus + DHCP CNI
    └── IP: DHCP-assigned from NetBox-managed range
```

### Implementation Steps

1. **Install Multus CNI in Kind cluster**
   ```bash
   kubectl apply -f https://raw.githubusercontent.com/k8snetworkplumbingwg/multus-cni/master/deployments/multus-daemonset.yml
   ```

2. **Create NetworkAttachmentDefinition for DHCP interface**
   ```yaml
   apiVersion: k8s.cni.cncf.io/v1
   kind: NetworkAttachmentDefinition
   metadata:
     name: netbox-dhcp-network
     namespace: default
   spec:
     config: |
       {
         "cniVersion": "0.3.1",
         "type": "macvlan",
         "master": "eth0",  # Use host interface
         "mode": "bridge",
         "ipam": {
           "type": "dhcp"
         }
       }
   ```

3. **Create Pod with Secondary Interface**
   ```yaml
   apiVersion: v1
   kind: Pod
   metadata:
     name: dhcp-test-pod
     annotations:
       k8s.v1.cni.cncf.io/networks: netbox-dhcp-network
   spec:
     containers:
     - name: test
       image: busybox
       command: ["sleep", "3600"]
   ```

### Pros
- ✅ Industry standard for multi-interface pods
- ✅ Supports multiple CNI plugins per pod
- ✅ Works with DHCP CNI plugins
- ✅ Well-documented and maintained

### Cons
- ⚠️ Requires additional CNI installation
- ⚠️ DHCP CNI plugin needed (e.g., `whereabouts` or custom DHCP CNI)
- ⚠️ More complex setup

## Option 3: Host Network Mode + DHCP Client

Use host networking mode and run DHCP client directly in the pod.

### Implementation

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: dhcp-test-pod
spec:
  hostNetwork: true
  containers:
  - name: dhcp-client
    image: busybox
    command: ["/bin/sh", "-c"]
    args:
    - |
      # Install DHCP client (udhcpc)
      apk add --no-cache busybox-extras
      # Request DHCP IP on specific interface
      udhcpc -i eth1 -s /etc/udhcpc/udhcpc.sh
      sleep 3600
```

### Pros
- ✅ Simple - no additional CNI needed
- ✅ Direct DHCP client control
- ✅ Easy to test and debug

### Cons
- ⚠️ Security concerns (host network access)
- ⚠️ Not suitable for production
- ⚠️ Limited to single interface per pod
- ⚠️ Requires privileged containers

## Option 4: Custom DHCP CNI Plugin

Create a custom CNI plugin that:
1. Requests IP from NetBox API (via NetBoxIPAddress CRD)
2. Configures the interface with the assigned IP
3. Updates NetBoxIPAddress status with the assigned IP

### Architecture

```
Pod Creation
    ↓
Multus invokes custom-dhcp-netbox CNI
    ↓
CNI reads NetBoxIPAddress CRD spec
    ↓
CNI requests IP allocation from NetBox API
    ↓
CNI configures pod interface with assigned IP
    ↓
CNI updates NetBoxIPAddress status.address
```

### Implementation Components

1. **Custom CNI Plugin** (`netbox-dhcp-cni`)
   - Written in Go (CNI standard)
   - Reads `NetBoxIPAddress` CRD from Kubernetes API
   - Calls NetBox API to allocate IP
   - Configures pod interface
   - Updates CRD status

2. **NetworkAttachmentDefinition**
   ```yaml
   apiVersion: k8s.cni.cncf.io/v1
   kind: NetworkAttachmentDefinition
   metadata:
     name: netbox-dhcp
   spec:
     config: |
       {
         "cniVersion": "0.3.1",
         "type": "netbox-dhcp",
         "netboxIPAddressCRD": "pod-dhcp-ip",
         "netboxURL": "http://netbox.netbox:80"
       }
   ```

3. **NetBoxIPAddress CRD** (already exists)
   ```yaml
   apiVersion: dcops.microscaler.io/v1alpha1
   kind: NetBoxIPAddress
   metadata:
     name: pod-dhcp-ip
   spec:
     status: dhcp
     ipRange:
       apiGroup: dcops.microscaler.io
       kind: NetBoxIPRange
       name: dhcp-pool-range
     tenant:
       apiGroup: dcops.microscaler.io
       kind: NetBoxTenant
       name: datacenter-tenant
   ```

### Pros
- ✅ Full integration with NetBox controller
- ✅ GitOps-friendly (CRD-driven)
- ✅ Automatic IPAM tracking
- ✅ Supports both static and dynamic allocation

### Cons
- ⚠️ Requires custom CNI development
- ⚠️ More complex implementation
- ⚠️ Maintenance overhead

## Option 5: Init Container + Standard DHCP Client (Alternative to dhcpm)

Use an init container to:
1. Request DHCP IP via standard DHCP client
2. Create/update NetBoxIPAddress CRD with assigned IP
3. Main container uses the configured interface

### Implementation

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: dhcp-test-pod
  annotations:
    k8s.v1.cni.cncf.io/networks: netbox-dhcp-network
spec:
  initContainers:
  - name: dhcp-init
    image: netbox-dhcp-init:latest
    securityContext:
      capabilities:
        add: ["NET_ADMIN", "NET_RAW"]
    env:
    - name: INTERFACE
      value: "net1"
    - name: NETBOX_IP_ADDRESS_CRD
      value: "pod-dhcp-ip"
    command:
    - /bin/sh
    - -c
    - |
      # Request DHCP IP
      udhcpc -i $INTERFACE
      IP=$(ip addr show $INTERFACE | grep "inet " | awk '{print $2}')
      
      # Update NetBoxIPAddress CRD
      kubectl patch netboxipaddress $NETBOX_IP_ADDRESS_CRD \
        --type=merge \
        -p "{\"status\":{\"address\":\"$IP\"}}"
  containers:
  - name: app
    image: busybox
    command: ["sleep", "3600"]
```

### Pros
- ✅ Uses standard DHCP client
- ✅ Integrates with NetBox via CRD
- ✅ No custom CNI needed
- ✅ Works with Multus

### Cons
- ⚠️ Requires privileged init container
- ⚠️ Race conditions possible
- ⚠️ Less elegant than CNI solution

## Option 6: Device Plugin + NetBox Integration

Use Kubernetes Device Plugin framework to expose NetBox-managed IPs as devices.

### Architecture

```
NetBox Device Plugin
    ↓
Exposes NetBox IPs as Kubernetes Devices
    ↓
Pod requests device via resources
    ↓
Device plugin allocates IP and configures interface
```

### Pros
- ✅ Native Kubernetes integration
- ✅ Resource-based allocation
- ✅ Good for static IP assignments

### Cons
- ⚠️ Complex implementation
- ⚠️ Less suitable for dynamic DHCP
- ⚠️ Device plugins are for hardware resources

## Alternative Approach: Hybrid CNI (Option 2 + Option 4)

**Combine Multus CNI with a lightweight NetBox-aware DHCP CNI plugin:**

1. **Use Multus** for multi-interface support
2. **Create lightweight CNI plugin** that:
   - Reads `NetBoxIPAddress` CRD name from NetworkAttachmentDefinition
   - Queries NetBox controller for IP allocation
   - Configures interface
   - Updates CRD status

### Implementation Plan

1. **Phase 1: Basic Setup**
   - Install Multus CNI in Kind cluster
   - Create NetworkAttachmentDefinition with DHCP CNI
   - Test basic multi-interface pod

2. **Phase 2: NetBox Integration**
   - Create `netbox-dhcp-cni` plugin (Go-based)
   - Plugin reads `NetBoxIPAddress` CRD
   - Plugin calls NetBox controller API or directly updates CRD
   - Plugin configures interface

3. **Phase 3: Testing**
   - Create test pod with secondary interface
   - Verify IP assignment from NetBox range
   - Verify NetBoxIPAddress CRD status update
   - Test static reservations (MAC address)

## Testing Scenarios

### Scenario 1: Random DHCP Allocation
```yaml
# NetBoxIPAddress CRD
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxIPAddress
metadata:
  name: pod-random-dhcp-ip
spec:
  status: dhcp
  ipRange:
    apiGroup: dcops.microscaler.io
    kind: NetBoxIPRange
    name: dhcp-pool-range
  tenant:
    apiGroup: dcops.microscaler.io
    kind: NetBoxTenant
    name: datacenter-tenant
---
# Pod with secondary interface
apiVersion: v1
kind: Pod
metadata:
  name: dhcp-test-pod
  annotations:
    k8s.v1.cni.cncf.io/networks: netbox-dhcp-network
spec:
  containers:
  - name: test
    image: busybox
```

### Scenario 2: Static DHCP Reservation (MAC-based)
```yaml
# NetBoxIPAddress CRD with MAC address
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxIPAddress
metadata:
  name: pod-static-dhcp-ip
spec:
  address: "192.168.1.100/24"
  status: dhcp
  macAddress: "aa:bb:cc:dd:ee:ff"  # Pod interface MAC
  tenant:
    apiGroup: dcops.microscaler.io
    kind: NetBoxTenant
    name: datacenter-tenant
```

## Kind-Specific Considerations

### Limitations
- Kind uses `kindnetd` CNI (basic bridge networking)
- Host interface access may be limited
- Some CNI plugins may not work in Kind's containerized environment

### Workarounds
- Use `macvlan` CNI with host network interface
- Use `ipvlan` for better Kind compatibility
- Consider using `bridge` CNI for testing

### Recommended Kind Configuration

```yaml
# kind-config.yaml additions
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
networking:
  podSubnet: "10.204.0.0/16"
  serviceSubnet: "10.205.0.0/16"
  # Enable host network access for DHCP testing
  disableDefaultCNI: false  # Keep kindnetd for primary interface
```

## Implementation Milestones

### Milestone 1: Foundation - Test Infrastructure Setup
**Goal**: Establish basic test container infrastructure with `bollard` and `dhcpm`

**Tasks**:
1. Add `bollard` dependency to test crate
   - Add to `controllers/netbox/Cargo.toml`: `bollard = { version = "0.19", features = ["ssl", "chrono"] }`
   - Add `bytes = "1"` for bollard body handling
   - Add `futures-util = "0.3"` for async stream handling

2. Create `DockerTestContainer` RAII wrapper
   - Implement struct with `docker: Docker` and `container_id: String`
   - Implement `Drop` trait for automatic cleanup
   - Add helper methods: `from_id()`, `id()`
   - Location: `controllers/netbox/src/test_utils/docker_test_container.rs`

3. Create test utilities module
   - Helper functions for container management
   - Port mapping utilities
   - Health check utilities
   - Location: `controllers/netbox/src/test_utils/docker_helpers.rs`

4. Build and test `Dockerfile.dhcpm-test`
   - Verify image builds successfully
   - Test `dhcpm` binary is available in container
   - Test basic container lifecycle (create, start, stop, remove)

**Acceptance Criteria**:
- ✅ `cargo test` can create and clean up Docker containers
- ✅ RAII wrapper prevents orphaned containers
- ✅ Tests skip gracefully when `E2E_DOCKER` is not set

**Estimated Effort**: 2-3 hours

---

### Milestone 2: DHCP Server Container Setup
**Goal**: Set up a working DHCP server in a container for testing

**Tasks**:
1. Choose DHCP server: **ISC Kea** (production standard)
   - **ISC Kea** is the production DHCP server for integration with RouterOS switches/routers
   - Kea provides REST API for configuration management
   - Kea supports DHCPv4/v6, PXE boot options, and static reservations
   - Alternative: dnsmasq can be used for simpler unit tests, but Kea should be used for integration tests

2. Create `Dockerfile.dhcp-server` (or use existing image)
   - Use `iscorg/kea` official image or `networkboot/kea-dhcp4`
   - Configure Kea via JSON config file or REST API
   - Set up DHCP pool (e.g., 192.168.1.100-200/24)
   - Expose DHCP port (67/udp) and Kea Control Agent (8000/tcp for REST API)

3. Create DHCP server container helper
   - Function to start ISC Kea container
   - Configure DHCP subnet via Kea REST API or config file
   - Configure static reservations (MAC-based) via API
   - Wait for server to be ready (health check via Kea Control Agent)

4. Test DHCP server functionality
   - Verify Kea responds to DHCP DISCOVER
   - Verify IP allocation from pool works
   - Test static reservations (MAC-based) via Kea API
   - Verify Kea REST API is accessible for configuration

**Acceptance Criteria**:
- ✅ ISC Kea container starts and responds to DHCP requests
- ✅ Can allocate IPs from configured pool
- ✅ Supports static reservations via MAC address (configured via Kea API)
- ✅ Kea Control Agent REST API is accessible for configuration

**Estimated Effort**: 4-5 hours (Kea setup is more complex than dnsmasq)

**Note**: For production integration, ISC Kea is the standard DHCP server that integrates with RouterOS switches/routers via DHCP relay. Using Kea in tests ensures compatibility with production workflows.

**How Kea Integrates with NetBox**:
- **Kea does NOT directly use NetBox** - it doesn't query NetBox APIs
- Instead, a **DHCP Controller** (Phase 2+, not yet implemented) reconciles CRD data to Kea:
  1. Controller watches NetBox CRDs (NetBoxPrefix, NetBoxIPRange, NetBoxIPAddress) via Kubernetes watch API
  2. Controller reads IPAM data from CRD spec/status (no NetBox API call needed - state is already in CRD)
  3. Controller translates CRD data to Kea configuration format
  4. Controller pushes configuration to Kea via Kea Control Agent REST API (port 8000)
  5. Kea serves DHCP based on the configuration it received
- **NetBox CRDs are the source of truth** - Kea is configured from CRD state, which is reconciled from Git → NetBox
- **No polling loops** - event-driven sync reacts instantly to CRD status changes
- **RouterOS** acts as DHCP relay, forwarding DHCP requests to Kea

---

### Milestone 3: Basic DHCP Test with dhcpm
**Goal**: Test DHCP allocation using `dhcpm` in a container

**Tasks**:
1. Create basic DHCP test
   - Start ISC Kea DHCP server container
   - Start test container with `dhcpm`
   - Execute `dhcpm` DISCOVER/REQUEST (DORA sequence)
   - Parse IP from JSON output

2. Implement IP parsing from dhcpm output
   - Parse JSON response from `dhcpm --output json`
   - Extract `yiaddr` field (your IP address)
   - Handle errors gracefully

3. Verify IP assignment
   - Confirm IP is within DHCP pool range
   - Verify IP format (CIDR notation)

4. Add test utilities
   - `run_dhcpm_discover()` helper function
   - `parse_ip_from_dhcpm_output()` helper function
   - Error handling and logging

**Acceptance Criteria**:
- ✅ Test successfully requests IP from DHCP server
- ✅ IP is parsed correctly from dhcpm output
- ✅ IP is within expected range

**Estimated Effort**: 4-5 hours

---

### Milestone 4: NetBox Integration - Direct API
**Goal**: Integrate DHCP testing with NetBox API (without Kubernetes)

**Tasks**:
1. Start NetBox container (or use mock)
   - Use `netboxcommunity/netbox:latest` or mock
   - Configure NetBox with test tenant
   - Create test IP range in NetBox

2. Create NetBoxIPAddress via API
   - After DHCP allocation, create IP in NetBox
   - Set status to `dhcp`
   - Link to tenant and IP range

3. Verify NetBox integration
   - Query NetBox API to verify IP exists
   - Verify IP has correct status and associations
   - Test both random and static allocation

4. Add NetBox test utilities
   - `create_netbox_ip_address()` helper
   - `verify_ip_in_netbox()` helper
   - NetBox client setup for tests

**Acceptance Criteria**:
- ✅ DHCP-allocated IP is created in NetBox
- ✅ IP has correct status (`dhcp`)
- ✅ IP is linked to correct tenant and range

**Estimated Effort**: 4-5 hours

---

### Milestone 5: NetBoxIPAddress CRD Integration
**Goal**: Test DHCP allocation with NetBoxIPAddress CRD reconciliation

**Tasks**:
1. Create NetBoxIPAddress CRD in test
   - Use existing CRD definitions
   - Create CRD with `status: dhcp` and `ipRange` reference
   - No `address` specified (random allocation)

2. Start NetBox controller (or use mock)
   - Use existing mock controller or real controller
   - Controller should reconcile NetBoxIPAddress CRD

3. Test reconciliation flow
   - Create NetBoxIPAddress CRD
   - Run DHCP allocation via `dhcpm`
   - Update CRD status with allocated IP
   - Verify controller reconciles to NetBox

4. Test static reservation flow
   - Create NetBoxIPAddress CRD with `macAddress`
   - Run `dhcpm` with specific MAC
   - Verify correct IP assigned (from DHCP reservation)
   - Verify CRD status updated

**Acceptance Criteria**:
- ✅ NetBoxIPAddress CRD created and reconciled
- ✅ DHCP-allocated IP stored in CRD `status.address`
- ✅ Controller creates IP in NetBox with correct associations

**Estimated Effort**: 6-8 hours

---

### Milestone 6: Kind Integration Tests
**Goal**: Test DHCP allocation in real Kubernetes cluster (Kind)

**Tasks**:
1. Set up Kind test environment
   - Verify Kind cluster is running
   - Create test namespace
   - Deploy NetBox controller to cluster

2. Create test pod with dhcpm init container
   - Build `netbox-dhcp-init` container image
   - Create pod YAML with init container
   - Init container runs `dhcpm` and updates CRD

3. Test pod-based DHCP allocation
   - Create NetBoxIPAddress CRD in Kind
   - Create pod with dhcpm init container
   - Wait for init container to complete
   - Verify CRD status updated with IP

4. Test with Multus CNI (optional)
   - Install Multus CNI in Kind cluster
   - Create NetworkAttachmentDefinition
   - Test pod with secondary interface

**Acceptance Criteria**:
- ✅ Test pod successfully requests DHCP IP
- ✅ NetBoxIPAddress CRD status updated
- ✅ IP exists in NetBox via controller reconciliation

**Estimated Effort**: 6-8 hours

---

### Milestone 7: Comprehensive Test Suite
**Goal**: Complete test coverage for all DHCP scenarios

**Tasks**:
1. Test random DHCP allocation
   - No address specified in CRD
   - IP allocated from range
   - CRD status updated

2. Test static DHCP reservation (MAC-based)
   - MAC address in CRD spec
   - Specific IP assigned
   - Interface association in NetBox

3. Test static DHCP reservation (interface-based)
   - Interface reference in CRD spec
   - Interface resolved and IP assigned
   - Interface association in NetBox

4. Test error scenarios
   - ISC Kea DHCP server unavailable
   - Invalid MAC address format
   - Missing required fields (ipRange, tenant)
   - DHCP allocation timeout

5. Test validation
   - MAC address format validation
   - DHCP scenario validation (static vs random)
   - Required field validation

**Acceptance Criteria**:
- ✅ All DHCP scenarios have test coverage
- ✅ Error cases are handled gracefully
- ✅ Validation rules are tested

**Estimated Effort**: 4-6 hours

---

### Milestone 8: Documentation and Examples
**Goal**: Complete documentation and example configurations

**Tasks**:
1. Update `DHCP_POD_TESTING_OPTIONS.md`
   - Document all test scenarios
   - Add troubleshooting guide
   - Add CI/CD integration examples

2. Create example test files
   - `tests/dhcp_integration_test.rs` - Basic DHCP test
   - `tests/dhcp_netbox_integration_test.rs` - NetBox integration
   - `tests/dhcp_kind_integration_test.rs` - Kind integration

3. Create example YAML files
   - `config/examples/dhcp-test-pod.yaml` - Pod with dhcpm init container
   - `config/examples/dhcp-network-attachment.yaml` - Multus NetworkAttachmentDefinition

4. Add to CONTRIBUTING.md
   - How to run DHCP tests
   - Environment setup requirements
   - Troubleshooting common issues

**Acceptance Criteria**:
- ✅ All test scenarios documented
- ✅ Examples work out of the box
- ✅ Contributing guide updated

**Estimated Effort**: 3-4 hours

---

## Implementation Timeline

**Total Estimated Effort**: 37-50 hours (updated for ISC Kea complexity, DHCP Controller simulation, and webhook pattern implementation)

**Recommended Order**:
1. Milestone 1 (Foundation) - **Week 1**
2. Milestone 2 (DHCP Server - ISC Kea) - **Week 1**
3. Milestone 3 (Basic DHCP Test) - **Week 2**
4. Milestone 4 (NetBox API Integration) - **Week 2**
5. Milestone 5 (CRD Integration) - **Week 3**
6. Milestone 6 (Kind Integration) - **Week 3-4**
7. Milestone 7 (Test Suite) - **Week 4**
8. Milestone 8 (Documentation) - **Week 4**

**Dependencies**:
- Milestone 1 → Milestone 2, 3
- Milestone 2 → Milestone 3
- Milestone 3 → Milestone 4, 5
- Milestone 4 → Milestone 5
- Milestone 5 → Milestone 6
- Milestone 6 → Milestone 7
- All → Milestone 8

## Quick Start

1. **Start with Milestone 1**: Set up test infrastructure
   - Add `bollard` dependency
   - Create `DockerTestContainer` wrapper
   - Verify basic container lifecycle works

2. **Proceed sequentially**: Each milestone builds on the previous
   - Don't skip ahead - dependencies matter
   - Test each milestone before moving to next

3. **Iterate and refine**: Adjust milestones based on learnings
   - Some tasks may be simpler/harder than estimated
   - Add/remove tasks as needed

## References

### NetBox ↔ Kea Integration
- **[netbox-kea-dhcp](https://github.com/francoismdj/netbox-kea-dhcp)** - Reference implementation of NetBox → Kea sync
  - Python daemon that syncs NetBox prefixes/ranges/addresses to Kea subnets/pools/reservations
  - Webhook-based event-driven sync pattern
  - Kea Control Agent API usage examples
  - NetBox webhook configuration examples
  - Full sync at startup + incremental event-driven updates
- **[netbox-kea-dhcp on PyPI](https://pypi.org/project/netbox-kea-dhcp/)** - Python package

### Kubernetes Networking
- [Multus CNI Documentation](https://github.com/k8snetworkplumbingwg/multus-cni)
- [CNI Specification](https://github.com/containernetworking/cni/blob/master/SPEC.md)
- [Kubernetes Network Plugins](https://kubernetes.io/docs/concepts/extend-kubernetes/compute-storage-net/network-plugins/)

### ISC Kea
- [Kea Control Agent API](https://kea.readthedocs.io/en/kea-2.2.0/arm/ctrl-socket.html#control-agent)
- [Kea Configuration Guide](https://kea.readthedocs.io/en/kea-2.2.0/arm/config.html)
- [Kea Commands Reference](https://kea.readthedocs.io/en/kea-2.2.0/arm/commands.html)

### Project Documentation
- [NetBox DHCP Integration](docs/DHCP_IP_ADDRESS_INVESTIGATION.md)

