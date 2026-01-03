# Contributing to DCops - Complete Guide for Developers and AI Agents

This document provides comprehensive guidance for contributing to the DCops project, including complete controller reconciler architecture, development patterns, and AI agent-specific workflows.

## Table of Contents

1. [Controller Reconciler Architecture](#controller-reconciler-architecture)
2. [Development Workflow](#development-workflow)
3. [Project Structure](#project-structure)
4. [Code Organization Guidelines](#code-organization-guidelines)
5. [Testing Requirements](#testing-requirements)
6. [Adding New Reconcilers](#adding-new-reconcilers)
7. [AI Agent Guidelines](#ai-agent-guidelines)
8. [Common Tasks](#common-tasks)

---

## Controller Reconciler Architecture

This section provides a complete end-to-end overview of what a controller reconciler looks like, from CRD definition through to tests.

### 1. CRD Definition

The Custom Resource Definition (CRD) is defined in the `crds` crate, typically in a file like `crates/crds/src/dcim/netbox_site.rs`.

#### Example: NetBoxSite CRD

```rust
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxSite",
    namespaced,
    status = "NetBoxSiteStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxSiteSpec {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub tenant: NetBoxResourceReference,  // Required dependency
    pub region: Option<NetBoxResourceReference>,  // Optional dependency
    pub status: SiteStatus,
    // ... other fields
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxSiteStatus {
    pub netbox_id: Option<u64>,
    pub netbox_url: Option<String>,
    pub state: ResourceState,  // Pending, Created, Updated, Failed
    pub error: Option<String>,
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}
```

**Key Points:**
- Uses `kube::CustomResource` derive macro for automatic CRD generation
- Uses `schemars` for JSON schema generation (required for Kubernetes CRD validation)
- Uses `serde` for serialization/deserialization
- Status is optional (None = resource not yet created)
- Spec contains desired state
- Status contains actual state (netbox_id, netbox_url, state)

### 2. CR Structure

A Custom Resource (CR) is an instance of the CRD, created by users via YAML or kubectl.

#### Example: NetBoxSite CR (YAML)

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxSite
metadata:
  name: datacenter-1
  namespace: default
spec:
  name: "Data Center 1"
  slug: "datacenter-1"
  description: "Primary datacenter"
  tenant:
    apiGroup: dcops.microscaler.io
    kind: NetBoxTenant
    name: datacenter-tenant
    namespace: default
  status: Active
status:
  netboxId: 42
  netboxUrl: "http://netbox/api/dcim/sites/42/"
  state: Created
  error: null
```

**Key Points:**
- `metadata.name` and `metadata.namespace` identify the resource
- `spec` contains desired state (what user wants)
- `status` contains actual state (what controller has created)
- Dependencies are referenced via `NetBoxResourceReference` (tenant, region, etc.)

### 3. Reconciler Implementation

The reconciler is the core logic that reconciles desired state (CR) with actual state (NetBox).

#### File Structure

```
controllers/netbox/src/reconciler/
├── mod.rs              # Main Reconciler struct, backoff logic
├── dcim/
│   ├── mod.rs
│   └── site.rs         # NetBoxSite reconciler
└── dcim/
    └── site_test.rs     # Tests
```

#### Example: NetBoxSite Reconciler Pattern

```rust
// controllers/netbox/src/reconciler/dcim/site.rs

impl Reconciler {
    /// Reconciles a NetBoxSite CR with NetBox
    pub async fn reconcile_netbox_site(
        &self,
        site_crd: &NetBoxSite,
    ) -> Result<(), ControllerError> {
        // 1. Extract name and namespace
        let (name, namespace) = extract_name_and_namespace(site_crd, "NetBoxSite")?;
        
        // 2. Resolve tenant dependency (required)
        let tenant_id = resolve_required_dependency_id(
            &self.netbox_tenant_api,
            &site_crd.spec.tenant.name,
            "NetBoxTenant",
            name,
            |crd| crd.status.as_ref(),
        ).await?;
        
        // 3. Resolve token for tenant (multi-tenant support)
        let token = self.token_resolver
            .resolve_token_for_tenant(tenant_id)
            .await?;
        let client = self.token_resolver.get_client_for_tenant(tenant_id)?;
        
        // 4. Validate status and check for drift
        let result = validate_status_and_drift(
            site_crd.status.as_ref(),
            "NetBoxSite",
            namespace,
            name,
            |id| async move {
                client.get_site(SiteId(id)).await
            },
        ).await?;
        
        match result {
            DriftCheckResult::UseExisting(site) => {
                // 5a. Resource exists - check if update needed
                if site_needs_update(&site, site_crd, tenant_id, None, None, "active") {
                    let updated = update_site(&client, SiteId(site.id), site_crd, tenant_id).await?;
                    self.update_site_status(name, namespace, &updated, ResourceState::Updated).await?;
                    self.record_event_normal(
                        reasons::UPDATED,
                        &format!("Updated site '{}' in NetBox", name),
                        site_crd,
                    ).await;
                }
            }
            DriftCheckResult::Recreate | DriftCheckResult::StatusCleared { .. } => {
                // 5b. Resource doesn't exist or status cleared - create it
                match client.create_site(create_site_request(site_crd, tenant_id)).await {
                    Ok(site) => {
                        self.update_site_status(name, namespace, &site, ResourceState::Created).await?;
                        self.record_event_normal(
                            reasons::CREATED,
                            &format!("Created site '{}' in NetBox (ID: {})", name, site.id),
                            site_crd,
                        ).await;
                    }
                    Err(NetBoxError::Conflict(_)) => {
                        // GitOps compliance: query for existing and use it
                        if let Ok(existing) = client.get_site_by_name(&site_crd.spec.name).await {
                            self.update_site_status(name, namespace, &existing, ResourceState::Created).await?;
                        } else {
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                    Err(e) => {
                        self.update_site_status_error(name, namespace, &format!("{}", e)).await?;
                        return Err(ControllerError::NetBox(e));
                    }
                }
            }
        }
        
        Ok(())
    }
}
```

**Key Patterns:**
1. **Dependency Resolution**: Use `resolve_required_dependency_id` or `resolve_optional_dependency_id`
2. **Token Resolution**: Use `token_resolver.resolve_token_for_tenant()` for multi-tenant support
3. **Drift Detection**: Use `validate_status_and_drift` to check if resource exists
4. **Status Updates**: Always update CR status after operations
5. **Error Handling**: Update status with error message on failure
6. **GitOps Compliance**: Handle conflicts by querying for existing resources
7. **Event Emission**: Emit events for SRE visibility

### 4. Watcher Setup

The watcher monitors Kubernetes for CR changes and triggers reconciliation.

#### File: `controllers/netbox/src/watcher.rs`

```rust
impl Watcher {
    /// Watches NetBoxSite resources
    pub async fn watch_netbox_sites(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.netbox_site_api.clone(),
            self.reconciler.clone(),
            |reconciler: Arc<Reconciler>, site: Arc<NetBoxSite>| {
                let reconciler = reconciler.clone();
                let site = site.clone();
                async move {
                    reconciler.reconcile_netbox_site(&site).await
                }
            },
            "NetBoxSite",
        ).await
    }
}
```

#### Generic `watch_resource` Helper

The `watch_resource` helper provides:
- Automatic reconnection via `kube_runtime::Controller`
- Fibonacci backoff error policy
- Debounce (5s) to batch status updates
- Concurrency limits (3 per watcher)
- Periodic reconciliation (10s requeue) for drift detection

**Key Points:**
- Uses `kube_runtime::Controller` for automatic reconnection
- Error policy implements Fibonacci backoff
- Debounce batches status updates (reduces API load)
- Concurrency limits prevent resource exhaustion
- Always requeues on success (enables periodic reconciliation for drift detection)

### 5. Event Emission

Events provide visibility into reconciliation operations for SREs.

#### Event Reasons (Defined in `events.rs`)

```rust
pub mod reasons {
    pub const CREATED: &str = "Created";
    pub const UPDATED: &str = "Updated";
    pub const RECONCILIATION_FAILED: &str = "ReconciliationFailed";
    pub const DEPENDENCY_NOT_FOUND: &str = "DependencyNotFound";
    pub const DRIFT_DETECTED: &str = "DriftDetected";
    pub const RETRY_ATTEMPT: &str = "RetryAttempt";
    pub const TOKEN_RESOLUTION_FAILED: &str = "TokenResolutionFailed";
}
```

#### Event Recording in Reconciler

```rust
impl Reconciler {
    /// Record a Normal event
    pub async fn record_event_normal<K>(
        &self,
        reason: &str,
        message: &str,
        resource: &K,
    ) where
        K: kube::Resource + Send + Sync,
    {
        if let Some(recorder) = &self.event_recorder {
            recorder.record_normal(reason, message, resource).await;
        }
    }
    
    /// Record a Warning event
    pub async fn record_event_warning<K>(
        &self,
        reason: &str,
        message: &str,
        resource: &K,
    ) where
        K: kube::Resource + Send + Sync,
    {
        if let Some(recorder) = &self.event_recorder {
            recorder.record_warning(reason, message, resource).await;
        }
    }
}
```

**Key Points:**
- Events are optional (can be None for testing)
- Normal events for successful operations
- Warning events for errors that will be retried
- Events are visible via `kubectl get events` in the resource's namespace
- Uses trait-based `EventRecorderTrait` for testability

### 6. Error Handling & Backoff

The reconciler implements sophisticated error handling with exponential backoff.

#### Backoff State Management

```rust
// In Reconciler struct
backoff_states: Arc<Mutex<HashMap<String, BackoffState>>>,

struct BackoffState {
    backoff: FibonacciBackoff,  // 1 min min, 10 min max
    error_count: u32,
}

impl Reconciler {
    /// Get backoff duration for a resource
    pub fn get_backoff_for_resource(&self, resource_key: &str) -> (u64, u32) {
        let mut states = self.backoff_states.lock().unwrap();
        let state = states.entry(resource_key.to_string())
            .or_insert_with(BackoffState::new);
        (state.backoff.next_backoff_seconds(), state.error_count)
    }
    
    /// Increment error count (called by error policy)
    pub fn increment_error(&self, resource_key: &str) {
        let mut states = self.backoff_states.lock().unwrap();
        let state = states.entry(resource_key.to_string())
            .or_insert_with(BackoffState::new);
        state.increment_error();
    }
    
    /// Reset error count (called on successful reconciliation)
    pub fn reset_error(&self, resource_key: &str) {
        let mut states = self.backoff_states.lock().unwrap();
        if let Some(state) = states.get_mut(resource_key) {
            state.reset();
        }
    }
}
```

#### Fibonacci Backoff Sequence

```
Error 1: 60s  (1 minute)
Error 2: 60s  (1 minute)
Error 3: 120s (2 minutes)
Error 4: 180s (3 minutes)
Error 5: 300s (5 minutes)
Error 6: 480s (8 minutes)
Error 7: 600s (10 minutes - max cap)
Error 8+: 600s (stays at max)
```

**Key Points:**
- Each resource has independent backoff state
- Backoff resets on successful reconciliation
- Max cap prevents excessive delays
- Error policy in watcher uses backoff for requeue delays

### 7. Testing Strategy

Comprehensive testing at multiple levels ensures reliability.

#### Unit Tests: `site_test.rs`

```rust
#[cfg(test)]
mod tests {
    use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
    use crate::test_utils::create_test_netbox_site;
    use crate::kube_api_trait::KubeApiTrait;
    
    #[tokio::test]
    async fn test_reconcile_site_create() {
        // 1. Setup: Create mocks
        let mock_token_resolver = Arc::new(MockTokenResolver::new("http://test-netbox".to_string()));
        let (reconciler, apis, mock_event_recorder, _mock_secret_fetcher) = 
            create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // 2. Setup: Create test data
        let site = create_test_netbox_site("test-site", "default", None, None);
        let tenant = create_test_netbox_tenant("datacenter-tenant", "default", Some(1), Some("http://...".to_string()));
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.site_api.store("test-site".to_string(), site.clone());
        
        // 3. Setup: Add NetBox resources to mock
        let mock_client = mock_token_resolver.mock_client();
        mock_client.add_tenant(/* ... */);
        
        // 4. Execute: Reconcile
        let result = reconciler.reconcile_netbox_site(&site).await;
        
        // 5. Assert: Verify success
        assert!(result.is_ok());
        
        // 6. Assert: Verify status updated
        let updated = apis.site_api.get("test-site").await.unwrap();
        assert_eq!(updated.status.unwrap().netbox_id, Some(42));
        
        // 7. Assert: Verify event emitted
        assert_normal_event_emitted(&mock_event_recorder, reasons::CREATED)
            .expect("CREATED event should be emitted");
    }
}
```

#### Test Utilities

- **`create_test_reconciler_with_mock_token_resolver`**: Creates fully mocked reconciler
- **`MockTokenResolver`**: Mocks NetBox client and token resolution
- **`MockKubeApi`**: Mocks Kubernetes API operations
- **`MockEventRecorder`**: Captures events for assertions
- **`MockSecretFetcher`**: Mocks Kubernetes Secret fetching
- **Event test helpers**: Assert event emission and content

**Key Testing Patterns:**
1. **Mock Everything**: TokenResolver, KubeApi, EventRecorder, SecretFetcher
2. **Test All Paths**: Create, Update, Delete, Drift, Errors
3. **Verify Status**: Always check status updates
4. **Verify Events**: Assert events are emitted correctly
5. **Test Error Handling**: Dependency not found, network errors, conflicts

### 8. Integration Points

#### Controller Initialization

```rust
// controllers/netbox/src/controller.rs

impl Controller {
    pub async fn new(
        netbox_url: String,
        namespace: Option<String>,
    ) -> Result<Self, ControllerError> {
        // 1. Create Kubernetes client
        let kube_client = Client::try_default().await?;
        
        // 2. Create TokenResolver (multi-tenant support)
        let token_resolver = Arc::new(TokenResolver::new(kube_client.clone(), netbox_url));
        
        // 3. Create API clients for all CRD types (19 APIs)
        let netbox_site_api: Api<NetBoxSite> = Api::namespaced(kube_client.clone(), ns);
        // ... 18 more APIs
        
        // 4. Create SecretFetcher and EventRecorder
        let secret_fetcher = Arc::new(RealSecretFetcher::new(kube_client.clone()));
        let event_recorder = Some(Arc::new(RecorderWrapper::new(Recorder::new(kube_client.clone(), reporter))));
        
        // 5. Create Reconciler
        let reconciler = Reconciler::new(
            token_resolver,
            Some(secret_fetcher),
            event_recorder,
            // ... all 19 API wrappers
        );
        
        // 6. Run startup reconciliation (map existing NetBox resources to CRs)
        reconciler.startup_reconciliation().await?;
        
        // 7. Create Watcher
        let watcher = Watcher::new(reconciler.clone(), /* ... all APIs ... */);
        
        // 8. Spawn watcher tasks (one per CRD type - 20 total)
        let netbox_site_watcher = tokio::spawn(async move {
            watcher.watch_netbox_sites().await
        });
        // ... 19 more watchers
        
        // 9. Return Controller with all JoinHandles
        Ok(Controller {
            netbox_site_watcher,
            // ... 19 more watchers
        })
    }
}
```

#### Main Entry Point

```rust
// controllers/netbox/src/main.rs

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize logging
    tracing_subscriber::fmt::init();
    
    // 2. Get configuration
    let netbox_url = env::var("NETBOX_URL")?;
    let namespace = env::var("NAMESPACE").ok();
    
    // 3. Create and run controller
    let controller = Controller::new(netbox_url, namespace).await?;
    controller.run().await?;
    
    Ok(())
}
```

### Complete Flow

1. **User creates CR** → Kubernetes API server stores it
2. **Watcher detects change** → `kube_runtime::Controller` triggers reconciliation
3. **Reconciler runs** → Resolves dependencies, checks drift, creates/updates NetBox resource
4. **Status updated** → CR status patched with netbox_id, netbox_url, state
5. **Event emitted** → Kubernetes event created for SRE visibility
6. **Success/Error** → Error policy handles retries with backoff, or resets on success
7. **Periodic reconciliation** → Watcher requeues every 10s to detect drift

**Key Architectural Principles:**
- **GitOps Compliance**: Handle conflicts by querying for existing resources
- **Multi-Tenant**: TokenResolver resolves tenant-specific NetBox tokens
- **Observability**: Events provide visibility into all operations
- **Resilience**: Fibonacci backoff prevents thundering herd
- **Testability**: All dependencies are trait-based for easy mocking

---

## Development Workflow

### Before Writing Code

1. **Plan the module structure first**
   - What modules will this feature need?
   - What are the distinct responsibilities?
   - How will modules interact?

2. **Create module files immediately**
   - Don't write everything in one file and split later
   - Create empty modules with `todo!()` if needed

3. **Follow TDD principles**
   - Write tests first
   - Keep test modules small and focused
   - Aim for 65%+ coverage minimum, 80% target

### During Development

1. **Monitor module size**
   - If a module exceeds 300 lines, consider splitting
   - If it exceeds 400 lines, split immediately

2. **Keep modules focused**
   - One responsibility per module
   - Related code together, unrelated code separate

3. **Document as you go**
   - Add module docs (`//!`) immediately
   - Add function docs before implementation

4. **Write tests as you implement**
   - Don't wait until the end to write tests
   - Test each function/module as you complete it

5. **Verify functionality, not just compilation**
   - After implementing a feature, verify it actually works
   - For controllers: Verify CRs reconcile correctly
   - Use verification scripts

### Critical Rule: Compilation ≠ Working

> **MANDATORY:** Code that compiles is NOT considered working. You MUST verify functionality.

**Verification Requirements:**

1. ✅ **Code compiles** - Use `python3 scripts/host_aware_build.py --release -p netbox-controller` for comprehensive error checking
2. ✅ **Tests pass** - `cargo test` passes with adequate coverage
3. ✅ **Integration verification** - For controllers, verify CRs reconcile correctly
4. ✅ **Database verification** - For NetBox resources, verify they exist in the database
5. ✅ **End-to-end verification** - Use `scripts/verify_netbox_crs.py` to verify reconciliation

**⚠️ CRITICAL - Compilation Verification:**
- **MUST** use `python3 scripts/host_aware_build.py --release -p netbox-controller` to check for compilation errors
- **DO NOT** use `cargo check` - it may not catch all compilation errors
- **DO NOT** use `cargo build` - it may not catch all compilation errors
- **ONLY** the build script provides comprehensive error checking

**Never claim code is working just because it compiles with `cargo check`.**

---

## Project Structure

```
DCops/
├── crates/                    # Library crates
│   ├── crds/                  # CRD definitions (source of truth)
│   ├── netbox-client/         # NetBox API client
│   └── ...
├── controllers/               # Kubernetes controllers
│   ├── netbox/                # NetBox controller (main)
│   ├── pxe-intent/            # PXE boot intent controller
│   └── routeros/              # RouterOS controller
├── config/                    # Kubernetes manifests
│   ├── crd/                   # Generated CRDs (ephemeral - don't edit!)
│   ├── examples/              # Example CRs
│   └── netbox-controller/     # Controller deployment
├── scripts/                   # Python development scripts
├── dockerfiles/               # Docker build files
└── docs/                      # Documentation (agentic planning docs)
```

### Key Directories

- **`crates/crds/src/`**: CRD definitions (Rust code using `kube::CustomResource`)
- **`config/crd/all-crds.yaml`**: Generated CRDs (ephemeral - auto-generated)
- **`controllers/netbox/src/reconciler/`**: Reconciliation logic organized by NetBox API sections
- **`controllers/netbox/src/test_utils/`**: Test utilities and mocks

---

## Code Organization Guidelines

### Module Organization Rules

#### 1. Module Size Limits

- **Maximum module size:** 500 lines of code (excluding tests)
- **Target module size:** 200-300 lines
- **When to split:** If a module exceeds 400 lines, split it immediately

#### 2. Module Structure Patterns

**For Library Crates (`crates/*`):**

```rust
// lib.rs - Re-exports only, < 50 lines
pub mod error;
pub mod client;
pub mod models;

#[doc(inline)]
pub use error::*;
#[doc(inline)]
pub use client::*;
#[doc(inline)]
pub use models::*;
```

**For Controller Crates (`controllers/*`):**

```rust
// main.rs - Entry point only, < 100 lines
mod controller;
mod reconciler;
mod watcher;
mod error;

use controller::Controller;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    // Parse config
    // Start controller
    Ok(())
}
```

#### 3. Module Naming Conventions

- Use **singular nouns** for module names: `error`, `client`, `model` (not `errors`, `clients`, `models`)
- Use **descriptive names**: `reconciler`, `watcher`, `validator` (not `util`, `helper`, `misc`)
- Avoid **generic names**: No `common`, `shared`, `utils` modules

### Follow Rust Guidelines

We follow the [Pragmatic Rust Guidelines](./rust-guidelines.txt). Key points:

- **M-SMALLER-CRATES**: If in doubt, split the crate
- **M-MODULE-DOCS**: Every public module must have `//!` documentation
- **M-FIRST-DOC-SENTENCE**: First sentence < 15 words
- **M-CANONICAL-DOCS**: Use canonical doc sections (Examples, Errors, Panics, Safety)

### Error Handling

- **Library crates:** Use `thiserror` for structured error types
- **Application crates:** Use `anyhow` for application-level errors
- **Each module** should have its own error module if errors are module-specific

### Documentation

Every public item must have:
- Summary sentence (< 15 words)
- Extended documentation
- Examples (for public APIs)
- Error documentation (if returns `Result`)

---

## Testing Requirements

### Coverage Tooling

We use `cargo-llvm-cov` for LLVM-based code coverage analysis.

**Installation:**
```bash
cargo install cargo-llvm-cov --locked
```

**Usage:**
```bash
# Generate coverage report
cargo llvm-cov --package netbox-controller --bin netbox-controller

# Generate HTML report
cargo llvm-cov --package netbox-controller --bin netbox-controller --html --output-dir target/llvm-cov/html
```

### Coverage Targets

- **Minimum:** 65% line coverage
- **Target:** 80% line coverage
- **Enforcement:** CI/CD will fail if coverage is below 65%

### What to Test

- **All public APIs** - Every public function should have tests
- **Error paths** - Test error conditions and edge cases
- **Integration points** - Test interactions between modules
- **Controller reconciliation** - Test reconciliation logic thoroughly
- **Event emission** - Verify events are emitted correctly

### Test Organization

- **Unit tests:** In the same file as the code (`#[cfg(test)] mod tests`)
- **Integration tests:** In `tests/` directory
- **Test utilities:** Behind `test-util` feature flag
- **Mock everything:** Use trait-based mocks for all dependencies

---

## Adding New Reconcilers

When adding a new NetBox CRD reconciler, follow this **complete** checklist. **Check off each item as you complete it** to avoid missing steps.

**⚠️ CRITICAL GITOPS PRINCIPLE: Complete Field Reconciliation**

> **MANDATORY:** Every reconciler MUST check EVERY field in the CRD spec that maps to a NetBox field. This is the core principle of GitOps - Git is the source of truth, and ALL fields must be reconciled in every reconciliation sweep. Do NOT skip fields, do NOT assume fields "rarely change", do NOT implement partial drift detection. **EVERY field must be checked, EVERY time.**

**📋 Implementation Checklists**

> **MANDATORY:** For each CRD, create and maintain an implementation checklist in `docs/implementationChecklists/<CRD>.md`. This checklist ensures:
> - All fields are documented and checked
> - Reusable helpers are used (DRY principle)
> - No fields are skipped
> - See `docs/implementationChecklists/README.md` for details and `docs/implementationChecklists/TEMPLATE.md` for the template.

### 1. CRD Definition (`crates/crds/src/`)

- [ ] Create file in appropriate module directory (`dcim/`, `ipam/`, `tenancy/`, `extras/`)
- [ ] Define `NetBox<Resource>Spec` struct with `#[derive(CustomResource, ...)]`
- [ ] Add `driftDetection: Option<bool>` field to spec (defaults to `true`)
- [ ] Add `comments: Option<String>` field to spec (if NetBox API supports it)
- [ ] Define `NetBox<Resource>Status` struct with required fields:
  - [ ] `netbox_id: Option<u64>`
  - [ ] `netbox_url: Option<String>`
  - [ ] `state: ResourceState`
  - [ ] `error: Option<String>`
  - [ ] `last_reconciled: Option<chrono::DateTime<chrono::Utc>>`
- [ ] Update module file (`<module>/mod.rs`) to export the new CRD:
  - [ ] Add `pub mod netbox_<resource>;`
  - [ ] Add `pub use netbox_<resource>::*;`
- [ ] Add to `crates/crds/src/bin/crdgen.rs`:
  - [ ] Add `use crds::NetBox<Resource>;` import
  - [ ] Add `crds.push(NetBox<Resource>::crd());` in the `main()` function

**⚠️ Important:** CRDs in `config/crd/all-crds.yaml` are **ephemeral** and automatically generated. Never edit them manually - they will be overwritten.

### 2. NetBox Client Models (`crates/netbox-client/src/`)

- [ ] Add NetBox API model struct in appropriate module (`dcim/`, `ipam/`, `tenancy/`, `extras/`)
- [ ] Include all fields from NetBox API response
- [ ] Add nested reference types if needed
- [ ] Update module `mod.rs` to export the model

### 3. NetBox Client Methods (`crates/netbox-client/src/`)

- [ ] Add `query_<resources>()` method in appropriate module
- [ ] Add `get_<resource>(id: <Resource>Id)` method
- [ ] Add `get_<resource>_by_name(name: &str)` method (if applicable)
- [ ] Add `create_<resource>(request: Create<Resource>Request)` method
- [ ] Add `update_<resource>(id: <Resource>Id, request: Update<Resource>Request)` method
- [ ] Handle pagination in query methods
- [ ] Add methods to `NetBoxClientTrait` in `crates/netbox-client/src/trait.rs`
- [ ] Implement methods in `crates/netbox-client/src/client.rs` (delegate to module)
- [ ] Add mock implementations in `crates/netbox-client/src/mock/mod.rs`
- [ ] Add mock implementations in `crates/netbox-client/src/mock/<module>.rs`

### 4. Reconciliation Logic (`controllers/netbox/src/reconciler/`)

#### 4.1. Module Organization

- [ ] Determine if reconciler goes in existing module or new module directory
- [ ] If new module: Create `<module>/mod.rs` and update parent `mod.rs`
- [ ] Create `<module>/<resource>.rs` file for reconciler implementation
- [ ] Update `<module>/mod.rs` to export:
  - [ ] `pub mod <resource>;`
  - [ ] `pub use <resource>::*;`

#### 4.2. Reconciler Struct Updates (`controllers/netbox/src/reconciler/mod.rs`)

- [ ] Add `use crds::NetBox<Resource>;` to imports
- [ ] Add `pub(crate) netbox_<resource>_api: Box<dyn KubeApiTrait<NetBox<Resource>> + Send + Sync>,` to `Reconciler` struct
- [ ] Update `Reconciler::new()` signature to accept `netbox_<resource>_api` parameter
- [ ] Store API client in `Reconciler::new()`: `Box::new(netbox_<resource>_api)`
- [ ] Add `create_typed_<resource>_status_patch()` helper function (see existing examples)
- [ ] Add `update_<resource>_status()` helper function (if needed)

#### 4.3. Reconciler Implementation (`controllers/netbox/src/reconciler/<module>/<resource>.rs`)

- [ ] Create `reconcile_netbox_<resource>()` method signature
- [ ] Implement dependency resolution (required and optional dependencies)
- [ ] Implement token resolution via `token_resolver.resolve_token_for_tenant()`
- [ ] Implement drift detection using `validate_status_and_drift()` helper
- [ ] Implement create logic with GitOps conflict handling
- [ ] **⚠️ CRITICAL: Implement COMPLETE field-level drift detection** (if `driftDetection` is enabled):
  - [ ] Create `<resource>_needs_update()` helper function
  - [ ] **MANDATORY: Check EVERY field in the CRD spec that maps to a NetBox field**
  - [ ] **MANDATORY: Compare ALL CRD spec fields with ALL NetBox resource fields**
  - [ ] **MANDATORY: Include ALL dependency fields (tenant, region, group, etc.) in comparison**
  - [ ] **MANDATORY: Do NOT skip any fields - GitOps requires complete reconciliation**
  - [ ] **MANDATORY: Fields to check (verify against CRD spec and NetBox model):**
    - [ ] `name` (if applicable)
    - [ ] `slug` (if applicable)
    - [ ] `description` (if applicable)
    - [ ] `comments` (if applicable)
    - [ ] ALL dependency references (tenant, region, site, group, manufacturer, etc.)
    - [ ] ALL enum fields (status, role, type, etc.)
    - [ ] ALL optional fields (latitude, longitude, physical_address, etc.)
    - [ ] ALL custom fields specific to the resource type
  - [ ] **Fields to EXCLUDE from drift detection (controller config only):**
    - [ ] `drift_detection` (controller config, not a NetBox field)
    - [ ] `reconcile_interval` (controller config, not a NetBox field)
    - [ ] `token_secret` (controller config, not a NetBox field)
    - [ ] `tags` (handled separately via `update_tags_if_differ()`)
  - [ ] Call `update_<resource>()` if drift detected
  - [ ] Emit `DRIFT_DETECTED` event
- [ ] Implement tag reconciliation using `update_tags_if_differ()` helper
- [ ] Add status update calls after create/update operations
- [ ] Add event emission for create/update/drift operations
- [ ] Handle errors and update status with error messages

**⚠️ CRITICAL GITOPS REQUIREMENT: Complete Field Reconciliation**

> **MANDATORY:** Every reconciler MUST check EVERY field in the CRD spec that maps to a NetBox field. This is the core principle of GitOps - Git is the source of truth, and ALL fields must be reconciled.

**Verification Checklist:**
1. [ ] List ALL fields in the CRD spec (from `crates/crds/src/<module>/netbox_<resource>.rs`)
2. [ ] List ALL fields in the NetBox model (from `crates/netbox-client/src/models.rs`)
3. [ ] For each CRD spec field, verify it's checked in `*_needs_update()`:
   - [ ] If it maps to a NetBox field → MUST be checked
   - [ ] If it's controller config (drift_detection, reconcile_interval, token_secret) → OK to skip
   - [ ] If it's tags → MUST be handled via `update_tags_if_differ()`
4. [ ] For each NetBox model field, verify it's checked:
   - [ ] If it's in the CRD spec → MUST be checked
   - [ ] If it's read-only (id, url, display, created, last_updated, *_count) → OK to skip
   - [ ] If it's computed (display) → OK to skip
5. [ ] Test that changing ANY field in NetBox UI triggers drift detection
6. [ ] Test that changing ANY field in CRD spec updates NetBox

**Example: Complete Field Check Pattern**
```rust
fn resource_needs_update(
    spec: &NetBoxResourceSpec,
    existing: &netbox_client::Resource,
    resolved_dependency_ids: ResolvedDependencies,
) -> bool {
    // 1. Check name (if applicable)
    let name_changed = spec.name != existing.name;
    
    // 2. Check slug (if applicable)
    let slug_changed = /* ... */;
    
    // 3. Check description (if applicable)
    let description_changed = spec.description.as_deref() != existing.description.as_deref();
    
    // 4. Check comments (if applicable)
    let comments_changed = spec.comments.as_deref() != existing.comments.as_deref();
    
    // 5. Check ALL dependencies (tenant, region, group, etc.)
    let tenant_changed = resolved_dependency_ids.tenant_id != existing.tenant.as_ref().map(|t| t.id);
    let region_changed = resolved_dependency_ids.region_id != existing.region.as_ref().map(|r| r.id);
    // ... check ALL dependencies
    
    // 6. Check ALL enum fields (status, role, type, etc.)
    let status_changed = /* ... */;
    
    // 7. Check ALL optional fields
    let latitude_changed = spec.latitude != existing.latitude;
    let longitude_changed = spec.longitude != existing.longitude;
    // ... check ALL optional fields
    
    // 8. Check ALL resource-specific fields
    // ... check ALL fields specific to this resource type
    
    // Tags are handled separately via update_tags_if_differ()
    
    // Return true if ANY field changed
    name_changed || slug_changed || description_changed || comments_changed
        || tenant_changed || region_changed || status_changed
        || latitude_changed || longitude_changed
        || /* ... all other fields ... */
}
```

**Common Mistakes to Avoid:**
- ❌ **Skipping optional fields** - Even optional fields must be checked (None vs Some(value))
- ❌ **Skipping dependency fields** - All dependencies must be compared
- ❌ **Assuming fields "rarely change"** - Check EVERY field, always
- ❌ **Only checking "important" fields** - GitOps requires ALL fields
- ❌ **Forgetting to handle None vs Some()** - Optional fields need special comparison logic

#### 4.4. Trait Implementations (`controllers/netbox/src/reconcile_helpers.rs`)

- [ ] Implement `NetBoxStatusCheck` trait for `crds::NetBox<Resource>Status`:
  ```rust
  impl NetBoxStatusCheck for crds::NetBox<Resource>Status {
      fn netbox_id(&self) -> Option<u64> { self.netbox_id }
      fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
  }
  ```
- [ ] If resource has tags: Implement `HasTags` trait for `netbox_client::<Resource>`:
  ```rust
  impl HasTags for netbox_client::<Resource> {
      fn tags(&self) -> &[netbox_client::NestedTag] { &self.tags }
  }
  ```

### 5. Watcher Setup (`controllers/netbox/src/watcher.rs`)

- [ ] Add `use crds::NetBox<Resource>;` to imports
- [ ] Add `netbox_<resource>_api: Api<NetBox<Resource>>` to `Watcher` struct
- [ ] Update `Watcher::new()` to accept `netbox_<resource>_api` parameter
- [ ] Store API client in `Watcher::new()`
- [ ] Create `watch_netbox_<resources>()` method:
  ```rust
  pub async fn watch_netbox_<resources>(&self) -> Result<(), ControllerError> {
      watch_resource(
          self.netbox_<resource>_api.clone(),
          self.reconciler.clone(),
          |reconciler, resource| {
              Box::pin(async move {
                  match reconciler.reconcile_netbox_<resource>(&*resource).await {
                      Ok(()) => Ok(Action::await_change()),
                      Err(e) => Err(e),
                  }
              })
          },
          "NetBox<Resource>",
      ).await
  }
  ```

### 6. Controller Integration (`controllers/netbox/src/controller.rs`)

- [ ] Add `use crds::NetBox<Resource>;` to imports
- [ ] Create API client in `Controller::new()`:
  ```rust
  let netbox_<resource>_api: Api<NetBox<Resource>> = Api::namespaced(kube_client.clone(), ns);
  ```
- [ ] Pass API client to `Reconciler::new()`: `KubeApiWrapper::new(netbox_<resource>_api.clone())`
- [ ] Pass API client to `Watcher::new()`: `netbox_<resource>_api.clone()`
- [ ] Add `netbox_<resource>_watcher: JoinHandle<Result<(), ControllerError>>` to `Controller` struct
- [ ] Spawn watcher task in `Controller::new()`:
  ```rust
  let netbox_<resource>_watcher = {
      let watcher = watcher_instance.clone();
      tokio::spawn(async move {
          watcher.watch_netbox_<resources>().await
      })
  };
  ```
- [ ] Store watcher in `Controller` struct initialization
- [ ] Add branch to `tokio::select!` in `Controller::run()`:
  ```rust
  result = &mut self.netbox_<resource>_watcher => {
      result.map_err(|e| ControllerError::Watch(format!("NetBox<Resource> watcher panicked: {}", e)))?
          .map_err(|e| ControllerError::Watch(format!("NetBox<Resource> watcher error: {}", e)))?;
  }
  ```

### 7. RBAC (`config/netbox-controller/role.yaml`)

- [ ] Add CRD resource name to main resources list (lowercase, plural):
  - [ ] Add `- netbox<resources>` (e.g., `- netboxtenantgroups`)
- [ ] Add status subresource to status resources list:
  - [ ] Add `- netbox<resources>/status` (e.g., `- netboxtenantgroups/status`)
- [ ] Verify verbs are correct:
  - [ ] Main resources: `get`, `list`, `watch`, `update`, `patch`, `create`, `delete`
  - [ ] Status resources: `get`, `update`, `patch`

**⚠️ Critical:** RBAC must be updated or the controller will fail with 403 Forbidden errors when trying to watch resources.

### 8. Example CR (`config/examples/`)

Example CRs are organized in subdirectories:
- `config/examples/platform/` - Platform-level resources (manufacturer, device-type, device-role, platform)
- `config/examples/tenant-<name>/` - Tenant-specific resources (site, device, IP addresses, etc.)

Example files follow the pattern: `netbox-<resource>-example.yaml` or `<resource>-example.yaml`

- [ ] Create example CR file with complete `spec`
- [ ] Include `driftDetection: true` in spec
- [ ] Include `comments` field if supported
- [ ] Include all required fields with realistic values
- [ ] Include optional fields with examples
- [ ] Add comments explaining each field
- [ ] Include tag references if applicable
- [ ] Include dependency references (tenant, etc.)

### 9. Tests (`controllers/netbox/src/reconciler/<module>/<resource>_test.rs`)

- [ ] Create test file `controllers/netbox/src/reconciler/<module>/<resource>_test.rs`
- [ ] Test create path (new resource)
- [ ] Test update path (existing resource with changes)
- [ ] Test drift detection (resource exists but fields differ)
- [ ] Test dependency not found (required dependency missing)
- [ ] Test optional dependency handling
- [ ] Test error handling (NetBox API errors, network errors)
- [ ] Test event emission (verify CREATED, UPDATED, DRIFT_DETECTED events)
- [ ] Test status updates (verify netbox_id, netbox_url, state are set correctly)
- [ ] Test tag reconciliation
- [ ] Test driftDetection flag (enabled vs disabled)
- [ ] Verify all tests pass: `cargo test --package netbox-controller <resource>`

### 10. Verification

- [ ] **Compilation**: `python3 scripts/host_aware_build.py --release -p netbox-controller` (DO NOT use `cargo check` or `cargo build`)
- [ ] **CRD Generation**: `python3 scripts/generate_crds.py` generates valid YAML
- [ ] **CRD Applied**: `kubectl apply -f config/crd/all-crds.yaml` succeeds
- [ ] **RBAC Applied**: `kubectl apply -f config/netbox-controller/role.yaml` succeeds
- [ ] **Example CR Applied**: `kubectl apply -f config/examples/.../netbox-<resource>-example.yaml` succeeds
- [ ] **Tests Pass**: `cargo test --package netbox-controller` passes
- [ ] **Coverage Meets Minimum**: `cargo llvm-cov --package netbox-controller --bin netbox-controller` shows ≥65% coverage
- [ ] **Controller Starts**: Controller pod starts without errors
- [ ] **Watcher Logs**: Controller logs show "Starting NetBox<Resource> watcher"
- [ ] **Reconciliation Works**: CR status shows `state: Created` and `netboxId` is populated
- [ ] **NetBox Resource Created**: Verify resource exists in NetBox via API or UI
- [ ] **Drift Detection Works**: Manually modify resource in NetBox, verify reconciler detects and fixes drift

### 11. Common Pitfalls to Avoid

- [ ] **Missing RBAC**: Forgetting to add CRD to `role.yaml` causes 403 errors
- [ ] **Missing Trait Implementation**: Forgetting `NetBoxStatusCheck` causes compilation errors
- [ ] **Missing Status Patch Helper**: Forgetting `create_typed_<resource>_status_patch()` causes compilation errors
- [ ] **Missing Import**: Forgetting to add `use crds::NetBox<Resource>;` in multiple files
- [ ] **Missing Module Export**: Forgetting to add `pub use` in module `mod.rs` files
- [ ] **Missing Watcher Spawn**: Forgetting to spawn watcher task in `Controller::new()`
- [ ] **Missing Select Branch**: Forgetting to add branch to `tokio::select!` in `Controller::run()`
- [ ] **Missing Client Methods**: Forgetting to add methods to `NetBoxClientTrait` and implementations
- [ ] **Missing Mock Methods**: Forgetting to add mock implementations causes test failures

**Remember:** This checklist must be completed in a single pass. Do not submit PRs with partial implementations. Use this checklist as a working document - check off each item as you complete it.

---

## Interconnected Resources - Dependency Management

When resources depend on each other (e.g., Tenant → TenantGroup, Device → Site, IPAddress → Interface), special care must be taken to ensure:

1. **Dependency changes are detected** - When a dependency is updated, dependent resources must be reconciled
2. **Field-level drift detection includes dependencies** - When checking if a resource needs updating, compare dependency fields
3. **Dependent resources update when dependencies change** - If a Tenant's group changes, the Tenant must be updated in NetBox

### Checklist: Resource with Dependencies

When creating a reconciler for a resource that **depends on** other resources:

- [ ] **Identify all dependencies** (required and optional):
  - [ ] List all `NetBoxResourceReference` fields in the CRD spec
  - [ ] Mark which are required vs optional
  - [ ] Document dependency relationships

- [ ] **Implement dependency resolution**:
  - [ ] Use `resolve_required_dependency_id()` for required dependencies
  - [ ] Use `resolve_optional_dependency_id()` for optional dependencies
  - [ ] Handle dependency not found errors gracefully
  - [ ] Emit `DEPENDENCY_NOT_FOUND` events when dependencies are missing

- [ ] **Include dependencies in drift detection**:
  - [ ] In `<resource>_needs_update()` function, compare dependency fields:
    ```rust
    // Example: Compare tenant group
    let group_changed = {
        let spec_group_id = if let Some(group_ref) = &spec.group {
            // Resolve group ID from CRD reference
            resolve_group_id_from_crd(group_ref).await?
        } else {
            None
        };
        let netbox_group_id = netbox_resource.group.as_ref().map(|g| g.id);
        spec_group_id != netbox_group_id
    };
    ```
  - [ ] Include dependency comparisons in the `needs_update` boolean
  - [ ] Test that dependency changes trigger updates

- [ ] **Update dependencies when creating/updating**:
  - [ ] Resolve dependency IDs before create/update calls
  - [ ] Pass resolved dependency IDs to NetBox API calls
  - [ ] Handle dependency resolution failures appropriately

- [ ] **Test dependency scenarios**:
  - [ ] Test create with required dependency
  - [ ] Test create with optional dependency
  - [ ] Test create without optional dependency
  - [ ] Test update when dependency changes
  - [ ] Test update when dependency is removed (if allowed)
  - [ ] Test drift detection when dependency changes in NetBox UI
  - [ ] Test error handling when dependency not found

### Checklist: Resource that is a Dependency

When creating a reconciler for a resource that **other resources depend on** (e.g., TenantGroup, Site, Device):

- [ ] **Identify all dependent resources**:
  - [ ] Search codebase for CRDs that reference this resource type
  - [ ] Document which resources depend on this one
  - [ ] Understand the dependency relationship (required vs optional)

- [ ] **Ensure dependent resources reconcile when dependency changes**:
  - [ ] When dependency resource is updated, dependent resources should detect drift
  - [ ] Periodic reconciliation (every 10s) will catch dependency changes
  - [ ] Field-level drift detection in dependent resources must compare dependency fields

- [ ] **Test dependency propagation**:
  - [ ] Create dependency resource (e.g., TenantGroup)
  - [ ] Create dependent resource (e.g., Tenant) referencing dependency
  - [ ] Update dependency resource (e.g., change TenantGroup name)
  - [ ] Verify dependent resource detects drift and updates (if applicable)
  - [ ] Test that dependent resources can be updated when dependency changes

### Checklist: Field-Level Drift Detection for Dependencies

When implementing drift detection for a resource with dependencies:

- [ ] **Compare dependency fields in `*_needs_update()` function**:
  ```rust
  fn tenant_needs_update(spec: &NetBoxTenantSpec, netbox: &Tenant, group_id: Option<u64>) -> bool {
      // Compare name
      let name_changed = spec.name != netbox.name;
      
      // Compare slug
      let slug_changed = /* ... */;
      
      // Compare description
      let description_changed = /* ... */;
      
      // Compare comments
      let comments_changed = /* ... */;
      
      // ⚠️ CRITICAL: Compare group dependency
      let group_changed = {
          let spec_group_id = spec.group.as_ref().map(|g| group_id); // Resolved group ID
          let netbox_group_id = netbox.group.as_ref().map(|g| g.id);
          spec_group_id != netbox_group_id
      };
      
      // Include ALL fields in the check
      name_changed || slug_changed || description_changed || comments_changed || group_changed
  }
  ```

- [ ] **Resolve dependency IDs before comparison**:
  - [ ] Don't compare CRD references directly (they're just names)
  - [ ] Resolve CRD references to NetBox IDs first
  - [ ] Compare resolved IDs with NetBox resource IDs

- [ ] **Handle optional dependencies correctly**:
  - [ ] If spec has no dependency but NetBox has one → drift detected
  - [ ] If spec has dependency but NetBox has none → drift detected
  - [ ] If both have dependencies but IDs differ → drift detected
  - [ ] If both have no dependency → no drift

- [ ] **Test all dependency drift scenarios**:
  - [ ] Dependency added in spec (NetBox has none)
  - [ ] Dependency removed from spec (NetBox has one)
  - [ ] Dependency changed in spec (NetBox has different one)
  - [ ] Dependency unchanged (no drift)
  - [ ] Optional dependency scenarios

### Checklist: Ensuring Dependent Resources Update

When a dependency resource changes, dependent resources must be updated:

- [ ] **Verify periodic reconciliation works**:
  - [ ] Watcher requeues every 10s (enabled by default)
  - [ ] Each reconciliation checks for drift
  - [ ] Field-level drift detection includes dependency fields

- [ ] **Verify drift detection includes dependencies**:
  - [ ] `*_needs_update()` function compares dependency fields
  - [ ] Dependency changes trigger updates
  - [ ] Updates include resolved dependency IDs

- [ ] **Test end-to-end dependency updates**:
  - [ ] Create dependency (e.g., TenantGroup "default")
  - [ ] Create dependent resource (e.g., Tenant) without dependency
  - [ ] Update dependent resource CR to include dependency
  - [ ] Verify dependent resource is updated in NetBox
  - [ ] Manually change dependency in NetBox UI
  - [ ] Verify dependent resource detects drift and corrects it

### Common Pitfalls: Interconnected Resources

- [ ] **Missing dependency in drift detection**: Forgetting to compare dependency fields in `*_needs_update()` causes dependent resources to not update when dependencies change
- [ ] **Comparing references instead of IDs**: Comparing `NetBoxResourceReference` objects directly instead of resolving to NetBox IDs first
- [ ] **Not handling optional dependencies**: Forgetting to handle `None` cases for optional dependencies
- [ ] **Not resolving dependencies before update**: Trying to update with unresolved dependency references instead of NetBox IDs
- [ ] **Assuming dependencies don't change**: Not implementing drift detection for dependency fields because "they rarely change"
- [ ] **Circular dependency issues**: Creating circular dependencies (e.g., TenantGroup → Tenant → TenantGroup) without proper handling

### Example: Tenant → TenantGroup Dependency

**Problem**: Tenants depend on TenantGroups. When a TenantGroup is created, existing Tenants should be updated to reference it if their CR specifies it.

**Solution**:
1. Tenant reconciler's `tenant_needs_update()` must compare `group` field
2. Resolve TenantGroup CRD reference to NetBox ID before comparison
3. Compare resolved ID with NetBox Tenant's group ID
4. If different, trigger update with resolved group ID

**Code Pattern**:
```rust
// In tenant_needs_update() or similar
let group_changed = {
    // Resolve group ID from CRD reference
    let spec_group_id = if let Some(group_ref) = &spec.group {
        // Resolve via CRD → NetBox ID
        resolve_tenant_group_id_from_crd(group_ref).await?
    } else {
        None
    };
    
    // Get current group ID from NetBox
    let netbox_group_id = netbox_tenant.group.as_ref().map(|g| g.id);
    
    // Compare
    spec_group_id != netbox_group_id
};

// Include in needs_update check
let needs_update = name_changed || slug_changed || description_changed || comments_changed || group_changed;
```

---

## AI Agent Guidelines

### 🚨 CRITICAL: Check for Existing Helpers/Traits First

> **MANDATORY:** Before creating ANY new function, method, or helper, you MUST:
> 1. **Search the codebase** for existing helpers, traits, or utilities that already do what you need
> 2. **Check if existing code can be extended** rather than duplicated
> 3. **Use existing patterns** - don't reinvent the wheel
> 4. **Refactor to use helpers** - if helpers exist but aren't being used, fix the code to use them

**Why:** The whole point of creating a client/library is to have **DRY (Don't Repeat Yourself)** code, not to move calls out of reconcilers into a WET (Write Everything Twice) mess.

### Before Creating New Code Checklist

**ALWAYS ask yourself:**
- [ ] Does a helper function already exist that does this?
- [ ] Is there a trait that can be extended?
- [ ] Can I refactor existing code to use a helper instead of duplicating?
- [ ] Have I searched the codebase for similar patterns?
- [ ] Am I following the DRY principle?

### Search Strategy

Before writing new code:
1. **Grep for similar patterns:**
   ```bash
   grep -r "pattern" crates/
   grep -r "pattern" controllers/
   ```

2. **Check for existing helpers:**
   ```bash
   grep -r "fn.*helper\|fn.*add_\|fn.*generate" crates/
   ```

3. **Look for traits:**
   ```bash
   grep -r "trait.*Trait" crates/
   grep -r "trait.*Trait" controllers/
   ```

4. **Check documentation:**
   - Read existing reconciler implementations for patterns
   - Check `reconcile_helpers.rs` for common patterns
   - Review existing tests for testing patterns

### Critical Rule: Modularize Immediately

> **MANDATORY:** When creating new code, **always** create proper module structure from the beginning. Never write monolithic files that will need to be refactored later.

**Why:** Refactoring large files into modules is:
- **Expensive:** Takes days of work
- **Risky:** High chance of introducing bugs
- **Unnecessary:** Can be avoided by starting with modules

### Module Creation Checklist

When implementing a new feature or crate, **always**:

1. ✅ **Create module files first** - Before writing any implementation
2. ✅ **Define module boundaries** - What goes in which module?
3. ✅ **Add module documentation** - `//!` docs for each module
4. ✅ **Keep modules small** - Target 200-300 lines, max 500 lines
5. ✅ **One responsibility per module** - Clear, single purpose

### Standard Module Patterns

**Library Crate Structure:**

```rust
// lib.rs - Re-exports only (< 50 lines)
//! Brief description of the crate.
//!
//! Extended documentation explaining the crate's purpose,
//! when to use it, and key concepts.

pub mod error;
pub mod client;
pub mod models;

#[doc(inline)]
pub use error::*;
#[doc(inline)]
pub use client::*;
#[doc(inline)]
pub use models::*;
```

**Controller Crate Structure:**

```rust
// main.rs - Entry point only (< 100 lines)
mod controller;
mod reconciler;
mod watcher;
mod error;

use controller::Controller;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    // Parse config
    // Start controller
    Ok(())
}
```

### Memory Bank Usage

AI agents should use the Memory Bank system for context persistence:

```bash
# Always start by loading Memory Bank context
farm agent startup

# Check Memory Bank status
farm agent memory-bank status

# Save status after completing tasks
# (Update Memory Bank with completed steps)
```

**Key Points:**
- Always load Memory Bank at session start
- Update Memory Bank after completing significant work
- Memory Bank maintains context across sessions
- Never allow Memory Bank context to become stale

### Farm CLI Commands

The project uses the `farm` CLI for all operations. **NEVER write shell scripts** - use `farm` commands instead.

**Most Frequently Used Commands:**

```bash
# Session Management
farm agent startup              # Load Memory Bank context (ALWAYS start here)
farm agent memory-bank status   # Check Memory Bank health

# Development Workflow
farm ai "your question"         # Intelligent AI routing to best model
farm test python test_file.py   # Run Python tests with rich output
farm git preflight             # Pre-commit quality checks
farm preflight                 # Full preflight checks

# Code Quality
farm lint python --fix         # Fix Python linting issues
farm coverage python           # Generate coverage reports
farm migration status          # Check shell→Python migration progress

# Environment & Setup
farm env decrypt               # Decrypt SOPS environment variables
farm setup models             # Configure Ollama models
farm docker health            # Check Docker infrastructure
```

**Essential Patterns:**

```bash
# Session Startup (MANDATORY)
farm agent startup  # Always run first - loads Memory Bank context

# Testing Workflow
farm test python test_file.py -v    # Run specific test with verbose output
farm test python --coverage         # Run with coverage analysis
farm coverage python                # Generate detailed coverage report

# Git Workflow
farm git preflight --fix           # Fix formatting before commit
farm git create-pr                 # Create pull request
farm git workflow-status           # Check CI/CD status
```

**Key Rules:**
- **NEVER write Python scripts to bypass CLI commands** - This defeats the purpose of having a robust CLI system
- **ALWAYS use `farm` commands for all operations** - Every operation must go through the proper CLI interface
- **CLI commands are the ONLY acceptable interface** - Direct module imports are forbidden for operational tasks
- **Fix CLI issues, don't bypass them** - If CLI commands have errors, FIX THE CLI, don't work around it

### Zero Shell Script Policy

**MANDATORY:** Shell scripts are banned in this repository.

- The agent must not write shell scripts
- The agent must not use shell scripts
- The agent must not execute shell scripts
- Don't create shell scripts cause they then have to be migrated, resulting in unnecessary expense

**Why:** All functionality should be in Python via the `farm` CLI for better maintainability and consistency.

### Test Driven Development (TDD)

**MANDATORY:** The agent must follow test driven development (TDD) principles.

- [ ] The agent must write tests before writing code
- [ ] The agent must ensure that all tests pass before committing code
- [ ] The agent must ensure all code is covered by tests
- [ ] The agent must ensure that all code has a minimum of 65% test coverage
- [ ] The agent must target 80% test coverage

### Code Location Rules

**Rust Code:**
- All Rust code must be in the `components/` directory (if applicable)
- Under no circumstances should the agent write Rust code outside the components directory
- If the agent needs to write experimental code, it should create an experiments module in the components directory
- Access to the experiments module must be via the farm CLI
- When experiments are no longer needed, they should be removed from the experiments module

**TypeScript Code:**
- TypeScript code is only allowed as part of the UI portal in `/ui`
- The agent must not write TypeScript code outside the UI portal
- The agent must not write TypeScript that is not part of the UI portal
- The agent must not write experimental code for troubleshooting in TypeScript

### GitHub Access

**Core Rules:**
- [ ] The agent must access GitHub via the minion-farm tools
- [ ] The agent may not use '--no-verify' or '--no-verify-commit' flags when committing code
- [ ] The agent MUST use the farm CLI to access GitHub
- [ ] The agent MUST use the farm CLI to commit code

The agent has access to the GitHub repository and can:
- Read and write to the repository
- Create and manage branches
- Create and manage pull requests
- Manage issues and comments
- Manage labels and milestones
- Manage project boards
- Manage workflows and actions

### Environment Variables

- Environment variables are stored in `.env` file
- Use `just decrypt-dev` to write out encrypted environment variables to `.env` file
- Environment variables are to be reloaded frequently
- Environment variables are used to configure the agent's access to external services

---

## Common Tasks

### Adding a New Reconciler

See [Adding New Reconcilers](#adding-new-reconcilers) section above for complete checklist.

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests for specific package
cargo test --package netbox-controller

# Run tests with coverage
cargo llvm-cov --package netbox-controller --bin netbox-controller

# Run specific test
cargo test --package netbox-controller test_reconcile_site_create
```

### Generating CRDs

```bash
# Generate CRDs from Rust code
python3 scripts/generate_crds.py

# Or use cargo directly
cargo run -p crds --bin crdgen > config/crd/all-crds.yaml
```

**⚠️ Important:** CRDs in `config/crd/all-crds.yaml` are **ephemeral** and automatically generated. Never edit them manually.

### Building

```bash
# Build all (Rust binary + Docker image)
just build

# Build Rust binary (debug)
just build-rust

# Build Rust binary (release)
just build-release

# Comprehensive error checking - THIS IS THE ONLY ACCEPTABLE WAY TO CHECK COMPILATION
# DO NOT use cargo check or cargo build - they may miss errors
python3 scripts/host_aware_build.py --release -p netbox-controller
```

**⚠️ IMPORTANT:** 
- **DO NOT** use `cargo check` - it may not catch all compilation errors
- **DO NOT** use `cargo build` - it may not catch all compilation errors  
- **ONLY** use `python3 scripts/host_aware_build.py --release -p netbox-controller` to verify compilation

### Verifying Functionality

```bash
# Verify NetBox CRs are reconciled correctly
python3 scripts/verify_netbox_crs.py --all

# Verify specific CRD
python3 scripts/verify_netbox_crs.py --crd netboxsites --name datacenter-1

# Check controller logs
kubectl logs -n dcops-system -l app=netbox-controller

# Check CR status
kubectl get netboxsite datacenter-1 -o yaml
```

### Code Review Checklist

- [ ] Module structure is clear and logical
- [ ] No module exceeds 500 lines
- [ ] Each module has a single, clear responsibility
- [ ] All public items are documented
- [ ] Error types are properly structured
- [ ] Tests are included and passing
- [ ] Test coverage meets minimum (65%, target 80%)
- [ ] **Functionality verified** - Not just compilation
- [ ] For controllers: CRs verified to reconcile correctly
- [ ] For NetBox resources: Verified in database using verification script
- [ ] No `util` or `common` modules
- [ ] Existing helpers/traits used instead of duplicating code

---

## Summary: Complete Flow

1. **User creates CR** → Kubernetes API server stores it
2. **Watcher detects change** → `kube_runtime::Controller` triggers reconciliation
3. **Reconciler runs** → Resolves dependencies, checks drift, creates/updates NetBox resource
4. **Status updated** → CR status patched with netbox_id, netbox_url, state
5. **Event emitted** → Kubernetes event created for SRE visibility
6. **Success/Error** → Error policy handles retries with backoff, or resets on success
7. **Periodic reconciliation** → Watcher requeues every 10s to detect drift

**Key Architectural Principles:**
- **GitOps Compliance**: Handle conflicts by querying for existing resources
- **Multi-Tenant**: TokenResolver resolves tenant-specific NetBox tokens
- **Observability**: Events provide visibility into all operations
- **Resilience**: Fibonacci backoff prevents thundering herd
- **Testability**: All dependencies are trait-based for easy mocking
- **Modularity**: Small, focused modules from the start
- **DRY**: Reuse helpers and traits, don't duplicate code

This architecture ensures reliable, observable, and maintainable reconciliation of Kubernetes CRs with NetBox resources.
