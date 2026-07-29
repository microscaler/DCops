# Architecture

DCops is built with Rust and follows Kubernetes controller patterns for GitOps-driven infrastructure management.

## System Overview

DCops consists of multiple Kubernetes controllers that manage NetBox resources through Custom Resource Definitions (CRDs).

```mermaid
graph TB
    subgraph "Git Repository"
        Git[YAML Files<br/>CRDs]
    end
    
    subgraph "Kubernetes Cluster"
        K8sAPI[Kubernetes API Server]
        Controller[NetBox Controller]
        CRDs[Custom Resources]
    end
    
    subgraph "NetBox"
        NetBoxAPI[NetBox API]
        NetBoxDB[(NetBox Database)]
    end
    
    subgraph "Physical Infrastructure"
        Devices[Servers, Switches, Routers]
        Networks[IP Networks, VLANs]
    end
    
    Git -->|GitOps Sync| K8sAPI
    K8sAPI -->|Watch| Controller
    Controller -->|Reconcile| CRDs
    Controller -->|API Calls| NetBoxAPI
    NetBoxAPI -->|Read/Write| NetBoxDB
    NetBoxDB -->|Manages| Devices
    NetBoxDB -->|Manages| Networks
    
    style Git fill:#e1f5ff
    style Controller fill:#fff4e1
    style NetBoxAPI fill:#ffe1f5
    style Devices fill:#e1ffe1
```

## Controller Architecture

### Watcher Pattern

Each resource type has its own watcher that monitors Kubernetes API for changes:

```mermaid
graph LR
    subgraph "Kubernetes API"
        Events[Watch Events]
    end
    
    subgraph "Controller"
        Debounce[Debounce<br/>5 seconds]
        Queue[Reconciliation Queue]
        Reconciler[Reconciler]
        Status[Update Status]
        EventsOut[Emit Events]
    end
    
    subgraph "NetBox API"
        Create[Create/Update]
        Read[Read State]
    end
    
    Events -->|CRD Changed| Debounce
    Debounce -->|Batch Updates| Queue
    Queue -->|Process| Reconciler
    Reconciler -->|Check State| Read
    Reconciler -->|Sync State| Create
    Reconciler -->|Update| Status
    Reconciler -->|Notify| EventsOut
    
    style Events fill:#e1f5ff
    style Reconciler fill:#fff4e1
    style Create fill:#ffe1f5
```

### Reconciliation Flow

```mermaid
sequenceDiagram
    participant User
    participant Git
    participant K8sAPI
    participant Controller
    participant NetBox
    
    User->>Git: Commit CRD YAML
    Git->>K8sAPI: GitOps syncs to cluster
    K8sAPI->>Controller: Watch event (CRD created)
    Controller->>Controller: Debounce (5s)
    Controller->>Controller: Resolve dependencies
    Controller->>NetBox: Check if resource exists
    alt Resource doesn't exist
        Controller->>NetBox: Create resource
        NetBox-->>Controller: Resource created (ID: 123)
    else Resource exists
        Controller->>NetBox: Compare state
        alt State differs
            Controller->>NetBox: Update resource
            NetBox-->>Controller: Resource updated
        end
    end
    Controller->>K8sAPI: Update CRD status
    Controller->>K8sAPI: Emit Kubernetes event
    K8sAPI-->>User: Status updated
```

## Component Architecture

### NetBox Controller

**Status:** ✅ Fully Implemented

The NetBox Controller manages all NetBox resources:
- **IPAM Resources** - Prefixes, IP addresses, VLANs, aggregates
- **DCIM Resources** - Sites, devices, interfaces, locations
- **Tenancy** - Tenants and tenant groups
- **Extras** - Tags

**Features:**
- Continuous reconciliation
- Drift detection and correction
- Multi-tenant support
- Event emission
- Dependency resolution

```mermaid
graph TB
    subgraph "NetBox Controller"
        Main[Main Entry Point]
        Watchers[17 Resource Watchers]
        Reconciler[Reconciler]
        Helpers[Reconciliation Helpers]
        Events[Event Emitter]
        Errors[Error Handler]
    end
    
    subgraph "Resource Types"
        IPAM[IPAM Resources<br/>9 types]
        DCIM[DCIM Resources<br/>11 types]
        Tenancy[Tenancy Resources<br/>1 type]
        Extras[Extras Resources<br/>1 type]
        Pools[IP Pool Resources<br/>2 types]
    end
    
    Main --> Watchers
    Watchers --> Reconciler
    Reconciler --> Helpers
    Reconciler --> Events
    Reconciler --> Errors
    
    Watchers --> IPAM
    Watchers --> DCIM
    Watchers --> Tenancy
    Watchers --> Extras
    Watchers --> Pools
    
    style Main fill:#fff4e1
    style Reconciler fill:#ffe1f5
```

### Dependency Resolution

Resources are reconciled in dependency order:

```mermaid
graph TD
    Start[CRD Created] --> CheckDeps{Check Dependencies}
    
    CheckDeps -->|Tenant Missing| WaitTenant[Wait for Tenant<br/>Status: Pending]
    WaitTenant -->|Tenant Ready| CheckDeps
    
    CheckDeps -->|Site Missing| WaitSite[Wait for Site<br/>Status: Pending]
    WaitSite -->|Site Ready| CheckDeps
    
    CheckDeps -->|All Dependencies Ready| Reconcile[Reconcile Resource]
    Reconcile -->|Success| Created[Status: Created]
    Reconcile -->|Error| Failed[Status: Failed<br/>Retry with Backoff]
    
    Failed -->|Retry| CheckDeps
    
    style Start fill:#e1f5ff
    style Created fill:#e1ffe1
    style Failed fill:#ffe1e1
```

## Data Flow

### IP Address Allocation Flow

```mermaid
sequenceDiagram
    participant User
    participant Git
    participant K8sAPI
    participant Controller
    participant NetBox
    
    User->>Git: Create IPPool CRD
    Git->>K8sAPI: Sync to cluster
    K8sAPI->>Controller: Watch event
    Controller->>NetBox: Check prefix exists
    NetBox-->>Controller: Prefix found (ID: 123)
    Controller->>K8sAPI: Update IPPool status
    
    User->>Git: Create IPClaim CRD
    Git->>K8sAPI: Sync to cluster
    K8sAPI->>Controller: Watch event
    Controller->>NetBox: Allocate IP from prefix
    NetBox-->>Controller: IP allocated (192.168.1.10/24)
    Controller->>K8sAPI: Update IPClaim status
    K8sAPI-->>User: IP allocated
```

### Drift Detection Flow

```mermaid
sequenceDiagram
    participant NetBox
    participant Controller
    participant K8sAPI
    participant Git
    
    Note over NetBox: Manual change in NetBox UI
    NetBox->>NetBox: Resource modified
    
    Controller->>NetBox: Periodic reconciliation
    NetBox-->>Controller: Resource state differs
    Controller->>Controller: Detect drift
    Controller->>NetBox: Update to match Git state
    NetBox-->>Controller: Resource updated
    Controller->>K8sAPI: Emit drift event
    K8sAPI-->>Git: Event logged
```

## Multi-Tenant Architecture

```mermaid
graph TB
    subgraph "Tenant A"
        TenantA[NetBoxTenant A]
        SecretA[Secret: token-a]
        ResourcesA[Resources A<br/>Sites, Devices, IPs]
    end
    
    subgraph "Tenant B"
        TenantB[NetBoxTenant B]
        SecretB[Secret: token-b]
        ResourcesB[Resources B<br/>Sites, Devices, IPs]
    end
    
    subgraph "Shared Resources"
        Platform[Platform Resources<br/>Manufacturer, DeviceType]
    end
    
    subgraph "Controller"
        Resolver[Token Resolver]
        Reconciler[Reconciler]
    end
    
    TenantA --> SecretA
    TenantB --> SecretB
    SecretA --> Resolver
    SecretB --> Resolver
    Resolver --> Reconciler
    Reconciler --> ResourcesA
    Reconciler --> ResourcesB
    Reconciler --> Platform
    
    style TenantA fill:#e1f5ff
    style TenantB fill:#ffe1f5
    style Platform fill:#fff4e1
```

## Technology Stack

### Rust

- **High Performance** - Compiled language with zero-cost abstractions
- **Memory Safe** - Prevents common memory errors
- **Type Safe** - Strong type system catches errors at compile time
- **Concurrent** - Built-in support for async/await

### Kubernetes

- **Controller Runtime** - Uses kube-rs for Kubernetes integration
- **CRD Support** - Full CRD generation and validation
- **Event System** - Emits Kubernetes events
- **RBAC** - Role-based access control

### NetBox

- **IPAM** - IP address management
- **DCIM** - Data center infrastructure management
- **REST API** - Well-documented REST API
- **Multi-Tenant** - Built-in tenant support

## Code Organization

### Controllers

```
controllers/
├── netbox/              # NetBox controller (fully implemented)
│   ├── src/
│   │   ├── main.rs     # Entry point, watcher setup
│   │   ├── controller.rs # Controller logic
│   │   ├── reconciler/ # Resource reconcilers
│   │   ├── error.rs    # Error types
│   │   └── events.rs   # Event emission
│   └── Cargo.toml
├── pxe-intent/         # PXE controller (stub)
└── routeros/           # RouterOS controller (stub)
```

### Shared Libraries

```
crates/
├── crds/               # CRD definitions
│   ├── src/
│   │   ├── dcim/       # DCIM CRDs
│   │   ├── ipam/       # IPAM CRDs
│   │   └── tenancy/    # Tenancy CRDs
│   └── Cargo.toml
├── netbox-client/      # NetBox API client
│   ├── src/
│   │   ├── client.rs   # Main client
│   │   ├── dcim/       # DCIM API
│   │   ├── ipam/       # IPAM API
│   │   └── tenancy/    # Tenancy API
│   └── Cargo.toml
└── ...
```

## Testing Architecture

### Trait-Based Mocking

DCops uses trait-based mocking for testing:

```rust
pub trait NetBoxClientTrait {
    async fn get_site(&self, id: u64) -> Result<NetBoxSite, NetBoxError>;
    // ...
}
```

This allows:
- Easy mocking in tests
- Testable code
- No runtime overhead in production

### Test Organization

- **Unit Tests** - In same file as code
- **Integration Tests** - In `*_test.rs` files
- **Mock NetBox API** - Trait-based mocks

## Deployment

### Development

- **Tilt** - Automatic rebuilding and deployment
- **Kind Cluster** - Local Kubernetes cluster
- **NetBox** - Deployed in cluster

### Production

- **Kubernetes Deployment** - Standard Kubernetes deployment
- **NetBox** - External NetBox instance
- **RBAC** - Role-based access control
- **Secrets** - Kubernetes Secrets for tokens

## Next Steps

- [Development Setup](./setup.md) - Set up your development environment
- [Testing](./testing.md) - Learn about testing practices
- [Contributing Guide](../contributing/contributing-guide.md) - Contribution guidelines
- [CONTRIBUTING.md](../../../../CONTRIBUTING.md) - Complete contributing guide
