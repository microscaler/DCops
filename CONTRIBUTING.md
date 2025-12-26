# Contributing to DCops

This document outlines development guidelines for the DCops project, with a strong emphasis on **modularization from the start** to avoid expensive refactoring later.

## Core Principle: Modularize Early, Not Later

> **Critical Rule:** It is **too expensive** to modularize huge code later. We must keep modules small and well-organized from the beginning to avoid refactoring costs.

Breaking down a 2000-line `lib.rs` into modules is:
- Time-consuming (days of work)
- Error-prone (easy to miss dependencies)
- Risky (can introduce bugs during refactoring)
- Expensive (blocks feature development)

**Solution:** Start with modules. Even if a module only has 50 lines, if it represents a distinct concept, it should be its own module.

## Module Organization Rules

### 1. Module Size Limits

- **Maximum module size:** 500 lines of code (excluding tests)
- **Target module size:** 200-300 lines
- **When to split:** If a module exceeds 400 lines, split it immediately

### 2. Module Structure Patterns

#### For Library Crates (`crates/*`)

Every library crate should follow this structure from day one:

```rust
// lib.rs - Re-exports only, < 50 lines
pub mod error;
pub mod client;  // or controller, service, etc.
pub mod models; // or types, domain, etc.

#[doc(inline)]
pub use error::*;
#[doc(inline)]
pub use client::*;
#[doc(inline)]
pub use models::*;
```

**Example: `netbox-client`**
```rust
// lib.rs
pub mod client;
pub mod models;
pub mod error;

pub use client::*;
pub use models::*;
pub use error::*;
```

#### For Controller Crates (`controllers/*`)

Controllers should be modularized immediately:

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

**Module breakdown:**
- `controller.rs` - Main controller struct and lifecycle
- `reconciler.rs` - Reconciliation logic
- `watcher.rs` - Kubernetes resource watchers
- `error.rs` - Controller-specific errors
- `config.rs` - Configuration types (if > 100 lines)

### 3. When to Create a New Module

Create a new module when:

1. **Distinct responsibility:** The code handles a different concern
   - ✅ `client.rs` for HTTP client logic
   - ✅ `models.rs` for data structures
   - ✅ `error.rs` for error types
   - ✅ `config.rs` for configuration

2. **Logical grouping:** Related types/functions belong together
   - ✅ All NetBox API models in `models.rs`
   - ✅ All error types in `error.rs`
   - ✅ All configuration in `config.rs`

3. **Size threshold:** Module exceeds 400 lines
   - Split immediately, don't wait

4. **Testability:** Module needs isolated testing
   - Easier to test small, focused modules

### 4. Module Naming Conventions

- Use **singular nouns** for module names: `error`, `client`, `model` (not `errors`, `clients`, `models`)
- Use **descriptive names**: `reconciler`, `watcher`, `validator` (not `util`, `helper`, `misc`)
- Avoid **generic names**: No `common`, `shared`, `utils` modules
  - If you need shared code, put it in a specific module or create a dedicated crate

### 5. File Organization

```
crates/my-crate/
├── Cargo.toml
└── src/
    ├── lib.rs          # Re-exports only (< 50 lines)
    ├── error.rs        # Error types
    ├── client.rs       # Main client/service logic
    ├── models.rs       # Data structures
    └── config.rs       # Configuration (if needed)
```

## Code Organization Guidelines

### Follow Rust Guidelines

We follow the [Pragmatic Rust Guidelines](./rust-guidelines.txt). Key points:

- **M-SMALLER-CRATES**: If in doubt, split the crate
- **M-MODULE-DOCS**: Every public module must have `//!` documentation
- **M-FIRST-DOC-SENTENCE**: First sentence < 15 words
- **M-CANONICAL-DOCS**: Use canonical doc sections (Examples, Errors, Panics, Safety)

### Error Handling

- **Library crates:** Use `thiserror` for structured error types (see `M-ERRORS-CANONICAL-STRUCTS`)
- **Application crates:** Use `anyhow` for application-level errors (see `M-APP-ERROR`)
- **Each module** should have its own error module if errors are module-specific

### Testing

- **Unit tests:** In the same file as the code (`#[cfg(test)] mod tests`)
- **Integration tests:** In `tests/` directory
- **Test utilities:** Behind `test-util` feature flag (see `M-TEST-UTIL`)
- **Test coverage:** Minimum 65% coverage required, target 80% coverage
- **Coverage tooling:** Use `cargo-llvm-cov` for coverage reports
- **Coverage verification:** Run `just test-coverage` before committing

### Critical Rule: Compilation ≠ Working

> **MANDATORY:** Code that compiles is NOT considered working. You MUST verify functionality.

**Verification Requirements:**

1. ✅ **Code compiles** - `cargo check` passes
2. ✅ **Tests pass** - `cargo test` passes with adequate coverage
3. ✅ **Integration verification** - For controllers, verify CRs reconcile correctly
4. ✅ **Database verification** - For NetBox resources, verify they exist in the database
5. ✅ **End-to-end verification** - Use `scripts/verify_netbox_crs.py` to verify reconciliation

**Never claim code is working just because it compiles.**

**For NetBox Controllers specifically:**
- After implementing reconciliation logic, verify:
  - CRD exists: `kubectl get crd <crd-name>`
  - CR has status: `kubectl get <crd> <name> -o jsonpath='{.status}'`
  - Resource in NetBox: Use `python3 scripts/verify_netbox_crs.py --crd <crd> --name <name>`
- Use the verification script: `just verify-netbox-crs` or `python3 scripts/verify_netbox_crs.py --all`

### Documentation

Every public item must have:
- Summary sentence (< 15 words)
- Extended documentation
- Examples (for public APIs)
- Error documentation (if returns `Result`)

```rust
/// Allocates an IP address from a prefix.
///
/// This function queries NetBox for available IPs in the specified prefix
/// and allocates the first available address.
///
/// # Examples
///
/// ```no_run
/// use netbox_client::NetBoxClient;
///
/// let client = NetBoxClient::new("https://netbox.example.com", "token")?;
/// let ip = client.allocate_ip(prefix_id).await?;
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The prefix does not exist
/// - No IPs are available in the prefix
/// - The NetBox API request fails
pub async fn allocate_ip(&self, prefix_id: u64) -> Result<IPAddress, NetBoxError> {
    // ...
}
```

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
   - Run `just test-coverage` regularly to check coverage

5. **Verify functionality, not just compilation**
   - After implementing a feature, verify it actually works
   - For controllers: Verify CRs reconcile correctly
   - Use verification scripts: `just verify-netbox-crs`
   - Check coverage meets minimum requirements

### Code Review Checklist

- [ ] Module structure is clear and logical
- [ ] No module exceeds 500 lines
- [ ] Each module has a single, clear responsibility
- [ ] All public items are documented
- [ ] Error types are properly structured
- [ ] Tests are included and passing
- [ ] Test coverage meets minimum (65%, target 80%)
- [ ] Coverage report generated and reviewed (`just test-coverage`)
- [ ] **Functionality verified** - Not just compilation
- [ ] For controllers: CRs verified to reconcile correctly
- [ ] For NetBox resources: Verified in database using verification script
- [ ] No `util` or `common` modules

## Examples

### ✅ Good: Modular from the Start

```rust
// crates/netbox-client/src/lib.rs
pub mod client;
pub mod models;
pub mod error;

pub use client::NetBoxClient;
pub use models::*;
pub use error::NetBoxError;
```

```rust
// crates/netbox-client/src/client.rs
use crate::error::NetBoxError;
use crate::models::*;

pub struct NetBoxClient { /* ... */ }

impl NetBoxClient {
    pub fn new(base_url: String, token: String) -> Result<Self, NetBoxError> { /* ... */ }
    pub async fn get_prefix(&self, id: u64) -> Result<Prefix, NetBoxError> { /* ... */ }
}
```

### ❌ Bad: Monolithic File

```rust
// crates/netbox-client/src/lib.rs (2000 lines)
// Error types, client logic, models, everything in one file
pub struct NetBoxError { /* ... */ }
pub struct NetBoxClient { /* ... */ }
pub struct Prefix { /* ... */ }
// ... 2000 more lines
```

### ✅ Good: Controller with Modules

```rust
// controllers/pxe-intent/src/main.rs
mod controller;
mod reconciler;
mod watcher;
mod error;

use controller::Controller;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ...
}
```

```rust
// controllers/pxe-intent/src/controller.rs
use crate::reconciler::Reconciler;
use crate::watcher::Watcher;

pub struct Controller {
    reconciler: Reconciler,
    watcher: Watcher,
}
```

### ❌ Bad: Everything in main.rs

```rust
// controllers/pxe-intent/src/main.rs (1500 lines)
// Controller, reconciler, watcher, error types all in one file
```

## Enforcement

- **Pre-commit hooks:** Check for module size limits and test coverage
- **CI/CD:** Fail builds if:
  - Any module exceeds 500 lines
  - Test coverage is below 65%
  - Tests fail
- **Code review:** Reject PRs that:
  - Add large monolithic files
  - Have insufficient test coverage (< 65%)
  - Claim functionality works without verification
  - Don't include verification steps for controllers

## Questions?

If you're unsure about module organization:
1. Check existing crates for patterns (`netbox-client`, `crds`)
2. Ask in code review
3. When in doubt, **create more modules, not fewer**

Remember: **It's cheaper to have too many small modules than one huge file.**

---

## CRD Generation and Management

### ⚠️ Critical: CRDs are Ephemeral

**IMPORTANT:** CRDs in `config/crd/all-crds.yaml` are **ephemeral** and **automatically generated** from Rust code in `crates/crds/src/`. They should **never** be manually edited.

### How CRD Generation Works

1. **Source of Truth:** CRD definitions are in `crates/crds/src/` (Rust code using `kube::CustomResource`)
2. **Generation Tool:** `crates/crds/src/bin/crdgen.rs` generates YAML from Rust types
3. **Generation Script:** `scripts/generate_crds.py` builds the binary and generates CRDs
4. **Output:** Generated YAML is written to `config/crd/all-crds.yaml`
5. **Tilt Integration:** Tilt's `generate-crds` resource automatically regenerates CRDs when:
   - CRD code in `crates/crds/src/` changes
   - `crates/crds/Cargo.toml` changes
   - `scripts/generate_crds.py` changes

### Rules for Working with CRDs

**✅ DO:**
- Edit CRD definitions in `crates/crds/src/` (Rust code)
- Use `#[derive(CustomResource, ...)]` to define CRDs
- Run `python3 scripts/generate_crds.py` to regenerate CRDs manually
- Let Tilt automatically regenerate CRDs during development (`tilt up`)
- Commit changes to `crates/crds/src/` (the source code)

**❌ DON'T:**
- Manually edit `config/crd/all-crds.yaml` (it will be overwritten)
- Commit manual changes to `config/crd/all-crds.yaml`
- Assume CRD YAML files are the source of truth
- Try to fix CRD issues by editing YAML directly
- Manually apply CRDs when using Tilt (Tilt handles this automatically)

### CRD Generation Workflow

When modifying CRDs:

1. **Edit Rust code** in `crates/crds/src/`:
   ```rust
   // crates/crds/src/dcim/netbox_device.rs
   #[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
   #[kube(group = "dcops.microscaler.io", version = "v1alpha1", kind = "NetBoxDevice")]
   pub struct NetBoxDeviceSpec {
       // Your changes here
   }
   ```

2. **Regenerate CRDs**:
   ```bash
   # Recommended: Use the generation script
   python3 scripts/generate_crds.py
   
   # Alternative: Use cargo directly
   cargo run -p crds --bin crdgen > config/crd/all-crds.yaml
   ```

3. **Tilt automatically regenerates** when you run `tilt up`:
   - Tilt watches `crates/crds/src/` for changes
   - Automatically runs `generate-crds` resource
   - Applies updated CRDs to the cluster
   - **Any manual edits to YAML will be lost**

### Why CRDs are Ephemeral

- **Single Source of Truth:** Rust code is the authoritative definition
- **Type Safety:** Rust types ensure consistency between code and CRDs
- **Automatic Updates:** Tilt ensures CRDs stay in sync with code during development
- **No Drift:** Prevents manual YAML edits from diverging from code
- **Consistency:** All CRDs are generated using the same process

### Troubleshooting CRD Issues

If CRDs aren't working as expected:

1. **Check the source code** in `crates/crds/src/` - this is what matters
2. **Regenerate CRDs** manually: `python3 scripts/generate_crds.py`
3. **Check Tilt logs** for `generate-crds` resource errors
4. **Verify CRD code compiles**: `cargo check -p crds`
5. **Never edit YAML directly** - fix the Rust code instead
6. **Check CRD generation**: `cargo run -p crds --bin crdgen` should output valid YAML

### Tilt Override Behavior

When using Tilt:
- Tilt's `generate-crds` resource **overrides** any manual changes to `config/crd/all-crds.yaml`
- Tilt watches for changes and regenerates CRDs automatically
- Manual edits to YAML will be lost on the next Tilt update
- Always edit the Rust source code, not the generated YAML

---

## Test Coverage Requirements

### Coverage Tooling

We use `cargo-llvm-cov` for LLVM-based code coverage analysis.

**Installation:**
```bash
cargo install cargo-llvm-cov --locked
```

**Usage:**
```bash
# Generate coverage report
just test-coverage

# Open HTML report
just test-coverage-open
```

### Coverage Targets

- **Minimum:** 65% line coverage
- **Target:** 80% line coverage
- **Enforcement:** CI/CD will fail if coverage is below 65%

### Coverage Reports

Coverage reports are generated in:
- **HTML:** `target/llvm-cov/html/index.html` (open with `just test-coverage-open`)
- **LCOV:** `lcov.info` (for CI/CD integration)

### What to Test

- **All public APIs** - Every public function should have tests
- **Error paths** - Test error conditions and edge cases
- **Integration points** - Test interactions between modules
- **Controller reconciliation** - Test reconciliation logic thoroughly

### Coverage Exclusions

Some code may be excluded from coverage:
- Generated code (if marked appropriately)
- Platform-specific code that can't be tested
- Main entry points (if they just delegate)

**Note:** Exclusions should be documented and justified.

## Complete CRD Implementation Checklist

When adding a new NetBox CRD, you **MUST** implement all of the following components in a single pass. This checklist ensures nothing is missed and avoids expensive back-and-forth iterations.

### 1. CRD Definition (`crates/crds/src/`)

#### 1.1 Create CRD Module File
- [ ] Create file in appropriate module directory:
  - `dcim/` for DCIM resources (sites, devices, etc.)
  - `ipam/` for IPAM resources (prefixes, IPs, VLANs, etc.)
  - `tenancy/` for tenancy resources (tenants, tenant groups)
  - `extras/` for extras (tags, custom fields)
- [ ] File name: `netbox_<resource_name>.rs` (e.g., `netbox_region.rs`)

#### 1.2 Define CRD Struct
- [ ] Define `NetBox<Resource>Spec` struct with:
  - `#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]`
  - `#[kube(group = "dcops.microscaler.io", version = "v1alpha1", kind = "NetBox<Resource>", namespaced, status = "NetBox<Resource>Status")]`
  - `#[serde(rename_all = "camelCase")]`
  - All required fields from NetBox API
  - Optional fields with `#[serde(skip_serializing_if = "Option::is_none")]`
  - Default values where appropriate

#### 1.3 Define Status Struct
- [ ] Define `NetBox<Resource>Status` struct with:
  - `#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]`
  - `#[serde(rename_all = "camelCase")]`
  - `netbox_id: Option<u64>`
  - `netbox_url: Option<String>`
  - `state: ResourceState` (or `PrefixState` for prefixes, `AllocationState` for IP claims)
  - `error: Option<String>`
  - `last_reconciled: Option<DateTime<Utc>>`

#### 1.4 Define Enums (if needed)
- [ ] Define status enums with `#[serde(rename_all = "PascalCase")]` (e.g., `Created`, `Failed`, not `created`, `failed`)

### 2. CRD Module Registration

#### 2.1 Update Module File (`crates/crds/src/<module>/mod.rs`)
- [ ] Add `pub mod netbox_<resource_name>;`
- [ ] Add `pub use netbox_<resource_name>::*;`

#### 2.2 Update Main Library (`crates/crds/src/lib.rs`)
- [ ] Ensure module is exported (already done if module file is correct)

### 3. CRD Generation (`crates/crds/src/bin/crdgen.rs`)

- [ ] Add import: `use crds::<module>::NetBox<Resource>;`
- [ ] Add to CRD list: `crds.push(NetBox<Resource>::crd());`
- [ ] Run `python3 scripts/generate_crds.py` to regenerate CRDs (or `cargo run -p crds --bin crdgen > config/crd/all-crds.yaml`)

**⚠️ Important:** CRDs in `config/crd/all-crds.yaml` are **ephemeral** and automatically generated. Never edit them manually - they will be overwritten by Tilt or the generation script.

### 4. NetBox Client Models (`crates/netbox-client/src/models.rs`)

- [ ] Add NetBox API model struct (e.g., `Region`, `SiteGroup`, `Location`)
- [ ] Include all fields from NetBox API response
- [ ] Use `#[serde(rename_all = "snake_case")]` for NetBox API models
- [ ] Add nested reference types if needed (e.g., `NestedRegion`, `NestedSiteGroup`)

### 5. NetBox Client Methods (`crates/netbox-client/src/client.rs`)

- [ ] Add `query_<resources>(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Resource>, NetBoxError>`
- [ ] Add `get_<resource>(&self, id: u64) -> Result<Resource, NetBoxError>`
- [ ] Add `get_<resource>_by_name(&self, name: &str) -> Result<Option<Resource>, NetBoxError>`
- [ ] Add `create_<resource>(&self, ...) -> Result<Resource, NetBoxError>`
- [ ] Add `update_<resource>(&self, id: u64, ...) -> Result<Resource, NetBoxError>` (if needed)
- [ ] Handle pagination in query methods (`fetch_all: true` should get all pages)
- [ ] Handle required fields (e.g., auto-generate `slug` if missing)

### 6. Reconciliation Logic (`controllers/netbox/src/reconciler.rs`)

#### 6.1 Add API Client to Reconciler Struct
- [ ] Add `netbox_<resource>_api: Api<NetBox<Resource>>` to `Reconciler` struct
- [ ] Update `Reconciler::new()` to accept and store the API client

#### 6.2 Implement Reconciliation Method
- [ ] Create `pub async fn reconcile_netbox_<resource>(&self, crd: &NetBox<Resource>) -> Result<(), ControllerError>`
- [ ] Extract `name` and `namespace` from CRD metadata
- [ ] Check if resource already exists in NetBox (by ID in status or by querying)
- [ ] If exists, verify it still exists in NetBox
- [ ] If not exists or deleted, create it in NetBox
- [ ] Update CRD status with NetBox ID, URL, state (`Created`), and `last_reconciled`
- [ ] Handle all error cases and update status to `Failed` with error message

#### 6.3 Add Error Status Helper
- [ ] Add `async fn update_status_error()` helper function inside reconciliation method
- [ ] Helper should:
  - Check if error is already set (avoid unnecessary updates)
  - Create error status with `state: ResourceState::Failed`
  - Patch CRD status with error
  - Log success/failure

#### 6.4 Update Startup Reconciliation
- [ ] Add to `startup_reconciliation()` method:
  - Query all CRs of this type
  - Query all resources from NetBox
  - Map NetBox resources to CRs (by name or other identifier)
  - Update CR status with NetBox ID if found

### 7. Watcher (`controllers/netbox/src/watcher.rs`)

#### 7.1 Add to Watcher Struct
- [ ] Add `netbox_<resource>_api: Api<NetBox<Resource>>` field
- [ ] Add `netbox_<resource>_state: Arc<Mutex<ReconciliationState>>` field

#### 7.2 Update Watcher::new()
- [ ] Accept `netbox_<resource>_api` parameter
- [ ] Initialize `netbox_<resource>_state` with `Arc::new(Mutex::new(ReconciliationState::new()))`

#### 7.3 Implement Watcher Method
- [ ] Create `async fn watch_netbox_<resources>(&self) -> Result<(), ControllerError>`
- [ ] Create watcher with `watcher(api, watcher::Config::default())`
- [ ] Handle events:
  - `watcher::Event::Apply(crd)` - Check generation, reconcile if changed
  - `watcher::Event::InitApply(crd)` - Always reconcile (initial sync)
  - `watcher::Event::Delete(crd)` - Log deletion
  - `watcher::Event::Init` - Log initialization
  - `watcher::Event::InitDone` - Log initialization complete
- [ ] On reconciliation error, call `self.reconciler.increment_error(&resource_key)`
- [ ] On success, call `self.reconciler.reset_error(&resource_key)`

### 8. Controller Integration (`controllers/netbox/src/controller.rs`)

#### 8.1 Add API Client
- [ ] Add `netbox_<resource>_api: Api<NetBox<Resource>>` to `Controller::new()` parameters
- [ ] Store in `Controller` struct

#### 8.2 Add Watcher Handle
- [ ] Add `netbox_<resource>_watcher: JoinHandle<Result<(), ControllerError>>` to `Controller` struct
- [ ] In `Controller::new()`, spawn watcher: `tokio::spawn(watcher.watch_netbox_<resources>())`
- [ ] Store handle in struct

#### 8.3 Add to Select Loop
- [ ] Add branch to `tokio::select!` in `Controller::run()`:
  ```rust
  result = &mut self.netbox_<resource>_watcher => {
      if let Err(e) = result {
          error!("NetBox<Resource> watcher error: {}", e);
      }
  }
  ```

### 9. RBAC (`config/netbox-controller/role.yaml`)

- [ ] Add permissions for the CRD:
  ```yaml
  - apiGroups: ["dcops.microscaler.io"]
    resources: ["netbox<resources>"]  # lowercase, plural
    verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
  - apiGroups: ["dcops.microscaler.io"]
    resources: ["netbox<resources>/status"]  # status subresource
    verbs: ["get", "patch", "update"]
  ```

### 10. Example CR (`config/examples/netbox-<resource>-example.yaml`)

- [ ] Create example CR file with:
  - Complete `spec` with all required fields
  - Realistic values
  - Comments explaining each field
  - Reference to dependencies (e.g., `site: "datacenter-1"` references `NetBoxSite`)

### 11. Verification Script (`scripts/verify_netbox_crs.py`)

- [ ] Add CRD to `CRD_TO_DB_MAP` dictionary:
  ```python
  'netbox<resources>': {
      'table': '<netbox_table_name>',
      'id_field': 'id',
      'name_field': '<name_field>',
      'spec_field': '<spec_field_to_match>',
  }
  ```

### 12. Testing Checklist

- [ ] **Compilation:** `cargo check --workspace` passes
- [ ] **CRD Generation:** `cargo run -p crds --bin crdgen` generates valid YAML
- [ ] **CRD Applied:** `kubectl apply -f config/crd/all-crds.yaml` succeeds
- [ ] **RBAC:** Controller can list/watch CRs (check logs for 403 errors)
- [ ] **Reconciliation:** CR is created in NetBox when applied
- [ ] **Status Update:** CR status shows `Created` state with NetBox ID
- [ ] **Error Handling:** Failed reconciliation updates status to `Failed` with error
- [ ] **Database Verification:** `scripts/verify_netbox_crs.py` confirms resource exists in NetBox DB
- [ ] **Startup Reconciliation:** Controller maps existing NetBox resources to CRs on startup

### 13. Documentation

- [ ] Update `docs/NETBOX_API_AUDIT.md` with implementation status
- [ ] Update `docs/PXE_CLUSTER_IMPLEMENTATION.md` if resource is needed for PXE/Pi clusters
- [ ] Add inline documentation to reconciliation method explaining:
  - What the resource represents
  - Dependencies on other resources
  - Special handling or edge cases

### Common Pitfalls to Avoid

1. **Missing Status Update on Error:** Always call `update_status_error()` before returning `Err()`
2. **Wrong Serialization Format:** Use `PascalCase` for state enums (`Created`, `Failed`), not `kebab-case` or `snake_case`
3. **Missing RBAC:** Controller will fail with 403 errors if RBAC is missing
4. **Missing CRD Generation:** CRD won't exist in cluster if not added to `crdgen.rs`
5. **Missing Watcher:** CRs won't be reconciled if watcher isn't spawned
6. **Missing Controller Integration:** Watcher won't run if not added to `tokio::select!`
7. **Missing Module Export:** CRD won't be accessible if not exported in module file
8. **Missing NetBox Client Methods:** Reconciliation will fail if client can't query/create resources

### Verification Command

After implementing all components, run:

```bash
# 1. Check compilation
cargo check --workspace

# 2. Generate CRDs
cargo run -p crds --bin crdgen > config/crd/all-crds.yaml

# 3. Apply CRDs (if testing in cluster)
kubectl apply -f config/crd/all-crds.yaml

# 4. Apply example CR
kubectl apply -f config/examples/netbox-<resource>-example.yaml

# 5. Check controller logs
kubectl logs -n dcops-system -l app=netbox-controller

# 6. Verify CR status
kubectl get netbox<resource> <name> -o yaml

# 7. Verify in NetBox DB
python3 scripts/verify_netbox_crs.py --crd netbox<resources> --name <name>
```

**Remember:** This checklist must be completed in a single pass. Do not submit PRs with partial implementations. Missing components will cause the controller to fail silently or with cryptic errors.

