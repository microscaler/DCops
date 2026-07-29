# DHCP Controller

DHCP Controller that syncs NetBox CRDs to ISC Kea DHCP server.

## Architecture

The controller is organized into modular components with single responsibilities:

```
controllers/dhcp/src/
├── main.rs                    # Entry point
├── controller.rs              # Main controller orchestrator
├── error.rs                   # Error types
├── types.rs                   # Shared types and constants
│
├── kea/                       # Kea Control Agent API client module
│   ├── mod.rs                 # Module exports
│   ├── client.rs              # Main KeaClient struct
│   ├── api.rs                 # HTTP API communication layer
│   └── commands.rs            # Kea command execution (config-get, config-set, config-test)
│
├── reconciler/                # Reconciliation module
│   ├── mod.rs                 # Main reconciler orchestrator
│   ├── config_builder.rs      # Builds Kea config from NetBox CRDs
│   ├── prefix_resolver.rs     # Resolves prefixes for IP ranges/addresses
│   ├── ip_utils.rs            # IP address and CIDR utility functions
│   ├── config_comparator.rs   # Compares Kea configurations
│   └── resource_reconciler.rs # Reconciles individual CRDs
│
└── watcher/                   # Watcher module
    ├── mod.rs                 # Main watcher orchestrator
    ├── prefix_watcher.rs      # Watches NetBoxPrefix CRDs
    ├── ip_range_watcher.rs    # Watches NetBoxIPRange CRDs
    └── ip_address_watcher.rs  # Watches NetBoxIPAddress CRDs
```

## Module Responsibilities

### `kea/` - Kea Control Agent API Client
- **`client.rs`**: Main `KeaClient` struct that orchestrates API and commands
- **`api.rs`**: Low-level HTTP communication with Kea Control Agent
- **`commands.rs`**: High-level command interface (config-get, config-set, config-test)

### `reconciler/` - Reconciliation Logic
- **`mod.rs`**: Main `DhcpReconciler` that orchestrates reconciliation
- **`config_builder.rs`**: Builds Kea configuration from all NetBox CRDs
- **`prefix_resolver.rs`**: Resolves which prefix contains a given IP range/address
- **`ip_utils.rs`**: IP address and CIDR manipulation utilities
- **`config_comparator.rs`**: Compares Kea configurations to detect changes
- **`resource_reconciler.rs`**: Reconciles individual CRDs (prefix, IP range, IP address)

### `watcher/` - Kubernetes Resource Watchers
- **`mod.rs`**: Main `DhcpWatcher` that orchestrates all watchers
- **`prefix_watcher.rs`**: Watches `NetBoxPrefix` CRDs
- **`ip_range_watcher.rs`**: Watches `NetBoxIPRange` CRDs
- **`ip_address_watcher.rs`**: Watches `NetBoxIPAddress` CRDs

### Root Modules
- **`main.rs`**: Entry point, loads configuration, starts controller
- **`controller.rs`**: Main controller that orchestrates reconciler and watcher
- **`error.rs`**: Error types for the controller
- **`types.rs`**: Shared constants and types

## Design Principles

1. **Single Responsibility**: Each module has one clear purpose
2. **Separation of Concerns**: HTTP communication, business logic, and watching are separated
3. **Modularity**: Easy to test, extend, and maintain
4. **Reusability**: Utility functions are in dedicated modules

## Usage

```bash
# Set Kea Control Agent URL (optional, defaults to http://localhost:8000)
export KEA_CONTROL_AGENT_URL="http://kea-server:8000"

# Run the controller
cargo run --bin dhcp-controller
```

## Configuration

- `KEA_CONTROL_AGENT_URL`: Kea Control Agent base URL (default: `http://localhost:8000`)
- `WATCH_NAMESPACE`: Kubernetes namespace to watch (default: `default`)

## Features

- ✅ Watches NetBoxPrefix, NetBoxIPRange, NetBoxIPAddress CRDs
- ✅ Full sync at startup
- ✅ Event-driven sync on CRD changes
- ✅ Translates CRDs to Kea configuration
- ✅ Applies configuration via Kea Control Agent API
- ✅ Modular, testable architecture
- ✅ **Comprehensive Kea REST API support** - Implements all key Kea Control Agent commands:
  - Configuration management (config-get, config-set, config-test, etc.)
  - Server control (shutdown, status-get, version-get, etc.)
  - Lease management (lease4-get, lease4-add, lease4-del, etc.)
  - Subnet management (subnet4-add, subnet4-del, etc.)
  - Reservation management (reservation-add, reservation-del, etc.)
  - Statistics (statistic-get, statistic-get-all, etc.)
  - Utility commands (leases-reclaim, subnet4-select-test, etc.)

See [KEA_COMMANDS.md](./KEA_COMMANDS.md) for complete command reference.

## MAC-keyed reservations & populated ranges (Aether integration)

The DHCP controller is the DCops half of the Aether ↔ DCops IPAM contract, where
**Aether owns the MAC and DCops/Kea own the IP**.

- A `NetBoxIPRange` with `markPopulated: true` is a DHCP pool: the NetBox
  controller does **not** create individual `IPAddress` objects inside it (see
  [`../../docs/NETBOX_IP_RANGE_ANALYSIS.md`](../../docs/NETBOX_IP_RANGE_ANALYSIS.md)).
  The DHCP controller turns the range into a **Kea subnet/pool**.
- A `NetBoxIPAddress` with `status: dhcp` and a `macAddress` (and no `address` —
  the IP is DCops's to assign) becomes a **Kea host reservation keyed on that
  MAC**. When the guest boots with that MAC, Kea hands back a **stable** lease —
  the same address every time.

Because Aether re-uses a workload's MAC across a recovery, the recovered VM hits
the same reservation and gets the same IP back. Full contract:
[`../../docs/AETHER_DCOPS_IPAM_CONTRACT.md`](../../docs/AETHER_DCOPS_IPAM_CONTRACT.md).

