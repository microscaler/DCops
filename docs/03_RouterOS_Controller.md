# RouterOS Controller Specification

## Overview

The **RouterOS Controller** manages MikroTik RouterOS and SwitchOS devices through their REST API, reconciling network intent from NetBox and Git CRDs to actual router/switch configuration.

**Status:** Phase 2+ (deferred from Phase 1)

**Purpose:** Enable GitOps-native management of RouterOS/SwitchOS network devices, eliminating ClickOps and manual router configuration.

## Responsibilities

### Phase 2+ Scope

The RouterOS Controller will handle:

1. **DHCP Relay Configuration**
   - Configure DHCP relay on RouterOS interfaces
   - Point relay to ISC Kea DHCP server
   - Support multiple VLANs with different DHCP servers
   - Essential for PXE boot to work across VLANs

2. **VLAN Management**
   - Create VLAN interfaces on RouterOS
   - Configure bridge VLAN tables
   - Manage tagged/untagged port membership
   - Enforce VLAN intent from NetBox

3. **Bridge Configuration**
   - Create and manage bridges
   - Enable VLAN filtering
   - Configure bridge VLAN table entries
   - Manage port membership (access/trunk)

4. **Network Device State Reconciliation**
   - Detect configuration drift
   - Correct unauthorized changes
   - Maintain desired state from NetBox
   - Report reconciliation status

5. **Device Management**
   - RouterOS device discovery and registration
   - Connection management (REST API)
   - Credential management (Kubernetes Secrets)
   - Device capability detection

## Architecture

### Data Flow

```
┌─────────────────┐
│  Git (CRDs)     │  RouterDevice, VLANPolicy CRDs
│  - RouterDevice │  (optional, Phase 2+)
│  - VLANPolicy   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  RouterOS       │  Reconciliation engine
│  Controller     │  (Rust / kube-rs)
└────────┬────────┘
         │
         ├─────────────────┐
         │                 │
         ▼                 ▼
┌─────────────────┐  ┌─────────────────┐
│  NetBox         │  │  RouterOS       │
│  - VLANs        │  │  REST API       │
│  - Prefixes     │  │  - DHCP relay  │
│  - Interfaces   │  │  - VLANs        │
│  - Devices      │  │  - Bridges      │
└─────────────────┘  └─────────────────┘
```

### Integration Points

**Inputs:**
- NetBox API (VLANs, prefixes, device inventory)
- Git CRDs (RouterDevice, VLANPolicy - optional)
- RouterOS device inventory

**Outputs:**
- RouterOS REST API calls
- RouterOS configuration changes
- Reconciliation status in CRD status

## CRDs (Phase 2+)

### RouterDevice

Defines a RouterOS/SwitchOS device to manage.

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: RouterDevice
metadata:
  name: router-01
spec:
  address: "192.168.1.1"
  apiPort: 8728  # RouterOS REST API port
  credentialsRef:
    name: router-01-credentials
  netBoxDeviceRef: "router-01"  # NetBox device name
  managed: true
status:
  connected: true
  lastReconciled: "2025-01-XX"
  reconciliationStatus: "Success"
```

### VLANPolicy (Optional)

Defines VLAN intent for RouterOS devices.

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: VLANPolicy
metadata:
  name: management-vlan-policy
spec:
  routerRef:
    name: router-01
  vlans:
    - vlanId: 10
      name: "management"
      netBoxVlanRef: "vlan-10"
      interfaces:
        - name: "ether2"
          mode: "access"
    - vlanId: 20
      name: "control-plane"
      netBoxVlanRef: "vlan-20"
      interfaces:
        - name: "ether3"
          mode: "access"
```

## RouterOS API Integration

### REST API Client

RouterOS exposes a REST API (port 8728) for configuration management.

**Key Operations:**
- `/interface/bridge` - Bridge management
- `/interface/vlan` - VLAN interface management
- `/interface/bridge/vlan` - Bridge VLAN table
- `/ip/dhcp-relay` - DHCP relay configuration
- `/interface` - Interface management

**Authentication:**
- Username/password (stored in Kubernetes Secrets)
- API token (preferred for automation)

**Example Operations:**

```rust
// Create VLAN interface
POST /rest/interface/vlan
{
  "name": "vlan10",
  "interface": "bridge",
  "vlan-id": 10
}

// Configure bridge VLAN table
POST /rest/interface/bridge/vlan
{
  "bridge": "bridge",
  "vlan-ids": "10",
  "tagged": "bridge,ether1",
  "untagged": "ether2"
}

// Configure DHCP relay
POST /rest/ip/dhcp-relay
{
  "interface": "vlan10",
  "server": "192.168.1.100"  // ISC Kea
}
```

## Use Cases

### Use Case 1: DHCP Relay for PXE Boot

**Scenario:** Raspberry Pi nodes need to PXE boot, but PXE server is on different VLAN.

**Solution:**
1. RouterOS Controller configures DHCP relay on RouterOS
2. Relay forwards PXE boot requests to ISC Kea DHCP server
3. Kea responds with PXE boot options
4. Nodes successfully boot via PXE

**Configuration:**
- RouterOS interface (VLAN) → DHCP relay → ISC Kea
- NetBox defines VLAN and prefix relationships
- Controller reconciles NetBox intent to RouterOS

### Use Case 2: VLAN Provisioning

**Scenario:** New cluster needs dedicated VLAN for isolation.

**Solution:**
1. Engineer creates VLAN in NetBox (or via CRD)
2. RouterOS Controller detects new VLAN
3. Controller creates VLAN interface on RouterOS
4. Controller configures bridge VLAN table
5. Controller configures port membership (access/trunk)
6. VLAN is operational

**Configuration:**
- NetBox VLAN → RouterOS VLAN interface
- NetBox interface intent → RouterOS bridge VLAN table
- Controller ensures state matches NetBox

### Use Case 3: Configuration Drift Correction

**Scenario:** Someone manually changes RouterOS VLAN configuration.

**Solution:**
1. RouterOS Controller detects drift during reconciliation
2. Controller compares RouterOS state vs NetBox intent
3. Controller corrects RouterOS configuration
4. Controller logs correction event
5. Status updated in CRD

**Safety:**
- Only managed objects are corrected
- Objects tagged `managed-by=gitops` are protected
- Unmanaged objects are ignored

## Safety Considerations

### 1. Management VLAN Protection

**Rule:** Management VLAN (VLAN 10) must never be modified by controller.

**Implementation:**
- Protected VLAN list in RouterDevice CRD
- Admission controller prevents modification
- Controller skips protected VLANs during reconciliation

### 2. Self-Management Prevention

**Rule:** RouterOS Controller must never manage the router that provides network connectivity to the management cluster.

**Implementation:**
- RouterDevice CRD validation
- Admission controller blocks self-management
- Controller checks router IP against management cluster network

### 3. Rollback Safety

**Rule:** RouterOS configuration changes must be reversible.

**Implementation:**
- Dry-run mode before apply
- Configuration backup before changes
- Rollback capability via Git history
- Status reporting for all changes

### 4. Credential Management

**Rule:** RouterOS credentials must be stored securely.

**Implementation:**
- Kubernetes Secrets for credentials
- RBAC for secret access
- Credential rotation support
- No credentials in Git

## Implementation Details

### RouterOS REST API Client

**Rust Implementation:**
- HTTP client for RouterOS REST API
- Authentication handling
- Error handling and retries
- Rate limiting compliance

**Key Libraries:**
- `reqwest` or `ureq` for HTTP client
- `serde` for JSON serialization
- `anyhow` / `thiserror` for error handling

### Reconciliation Loop

```rust
async fn reconcile_router(router: &RouterDevice) -> Result<()> {
    // 1. Connect to RouterOS API
    let client = RouterOSClient::new(&router.spec.address)?;
    
    // 2. Query NetBox for VLAN intent
    let vlans = netbox.get_vlans_for_device(&router.spec.netBoxDeviceRef)?;
    
    // 3. Query RouterOS current state
    let current_vlans = client.get_vlans().await?;
    
    // 4. Diff and plan changes
    let changes = diff_vlans(&vlans, &current_vlans)?;
    
    // 5. Apply changes (dry-run mode first)
    if !dry_run {
        client.apply_vlan_changes(&changes).await?;
    }
    
    // 6. Update status
    update_router_status(router, &changes).await?;
    
    Ok(())
}
```

## Phase 1 Workaround

For Phase 1, RouterOS must be manually configured:

- **DHCP Relay:** Manually configure on RouterOS
- **VLANs:** Manually create and configure
- **Bridges:** Manually set up

**Acceptable for Phase 1:** Manual RouterOS configuration is acceptable as long as:
- Configuration is documented
- Changes are infrequent
- PXE boot works correctly

**Phase 2 Goal:** Eliminate all manual RouterOS configuration.

## Dependencies

### External

- **RouterOS REST API** - Must be enabled on RouterOS devices
- **NetBox API** - For VLAN and device inventory
- **Kubernetes Secrets** - For RouterOS credentials

### Internal

- **NetBox API Client** - Shared with other controllers
- **kube-rs** - Controller framework
- **RouterOS REST API Client** - Custom implementation

## Success Criteria (Phase 2)

RouterOS Controller is successful when:

1. ✅ Can configure DHCP relay via Git CRDs
2. ✅ Can create VLANs on RouterOS from NetBox intent
3. ✅ Can configure bridge VLAN tables automatically
4. ✅ Can detect and correct configuration drift
5. ✅ Zero manual RouterOS configuration required
6. ✅ All RouterOS changes are auditable via Git

## References

- [MikroTik RouterOS REST API Documentation](https://help.mikrotik.com/docs/display/ROS/REST+API)
- [NetBox Documentation](https://docs.netbox.dev/)
- [DCops Summary](../docs/00_Summary.md)
- [ADR-001](../ADRs/ADR-001-Scope_and_Non-Goals.md)

