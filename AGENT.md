# Agent Development Guidelines

This document provides specific guidance for AI agents working on the DCops codebase, with **explicit modularization requirements** from the start.

## Critical Rule: Modularize Immediately

> **MANDATORY:** When creating new code, **always** create proper module structure from the beginning. Never write monolithic files that will need to be refactored later.

**Why:** Refactoring large files into modules is:
- **Expensive:** Takes days of work
- **Risky:** High chance of introducing bugs
- **Unnecessary:** Can be avoided by starting with modules

## Module Creation Checklist

When implementing a new feature or crate, **always**:

1. ✅ **Create module files first** - Before writing any implementation
2. ✅ **Define module boundaries** - What goes in which module?
3. ✅ **Add module documentation** - `//!` docs for each module
4. ✅ **Keep modules small** - Target 200-300 lines, max 500 lines
5. ✅ **One responsibility per module** - Clear, single purpose

## Standard Module Patterns

### Pattern 1: Library Crate Structure

**Always use this structure for `crates/*`:**

```rust
// lib.rs - Re-exports only (< 50 lines)
//! Brief description of the crate.
//!
//! Extended documentation explaining the crate's purpose,
//! when to use it, and key concepts.

pub mod error;
pub mod client;  // or service, controller, etc.
pub mod models;  // or types, domain, etc.

#[doc(inline)]
pub use error::*;
#[doc(inline)]
pub use client::*;
#[doc(inline)]
pub use models::*;
```

**Module breakdown:**
- `error.rs` - All error types for this crate
- `client.rs` - Main client/service implementation
- `models.rs` - Data structures and types

**Example: Creating `pxe-client`**

```rust
// Step 1: Create lib.rs with module structure
//! PXE Boot Service Client
//!
//! Client for interacting with PXE boot services.

pub mod error;
pub mod pixiecore;

pub use error::PxeError;
pub use pixiecore::PixiecoreClient;

// Step 2: Create error.rs
//! PXE client errors

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PxeError {
    // ...
}

// Step 3: Create pixiecore.rs
//! Pixiecore API client

use crate::error::PxeError;

pub struct PixiecoreClient {
    // ...
}
```

### Pattern 2: Controller Crate Structure

**Always use this structure for `controllers/*`:**

```rust
// main.rs - Entry point only (< 100 lines)
//! Controller name and purpose
//!
//! Extended description.

mod controller;
mod reconciler;
mod watcher;
mod error;
mod config;  // Only if config is > 100 lines

use controller::Controller;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    let config = config::load()?;
    let controller = Controller::new(config).await?;
    controller.run().await?;
    
    Ok(())
}
```

**Module breakdown:**
- `controller.rs` - Main controller struct, lifecycle, initialization
- `reconciler.rs` - Reconciliation logic for CRDs
- `watcher.rs` - Kubernetes resource watchers
- `error.rs` - Controller-specific errors
- `config.rs` - Configuration types (only if needed and > 100 lines)

**Example: Creating `pxe-intent-controller`**

```rust
// Step 1: Create main.rs with module structure
mod controller;
mod reconciler;
mod watcher;
mod error;

use controller::Controller;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ...
}

// Step 2: Create each module file immediately
// controller.rs, reconciler.rs, watcher.rs, error.rs
```

### Pattern 3: CRD Crate Structure

**For `crates/crds`, use one file per CRD:**

```rust
// lib.rs
pub mod boot_profile;
pub mod boot_intent;
pub mod ip_pool;
pub mod ip_claim;

pub use boot_profile::*;
pub use boot_intent::*;
pub use ip_pool::*;
pub use ip_claim::*;
```

**Each CRD in its own file:**
- `boot_profile.rs` - BootProfile CRD definition
- `boot_intent.rs` - BootIntent CRD definition
- etc.

## Module Size Rules

### Hard Limits

- **Maximum:** 500 lines per module (excluding tests)
- **Warning threshold:** 400 lines - split immediately
- **Target:** 200-300 lines per module

### When to Split

Split a module when:
1. It exceeds 400 lines
2. It has multiple distinct responsibilities
3. It's hard to understand at a glance
4. Tests are becoming hard to organize

### How to Split

1. **Identify responsibilities** - What distinct concerns exist?
2. **Create new modules** - One per responsibility
3. **Move code** - Keep related code together
4. **Update imports** - Fix all references
5. **Update tests** - Move tests to appropriate modules

## Module Documentation Requirements

### Every Module Must Have

```rust
//! Brief description (< 15 words).
//!
//! Extended documentation explaining:
//! - What this module contains
//! - When to use it
//! - Key concepts or patterns
//! - Examples if helpful
```

### Example

```rust
//! NetBox REST API client implementation.
//!
//! This module provides the `NetBoxClient` type for interacting with
//! the NetBox API. It handles authentication, request building, and
//! response parsing.
//!
//! # Examples
//!
//! ```no_run
//! use netbox_client::NetBoxClient;
//!
//! let client = NetBoxClient::new("https://netbox.example.com", "token")?;
//! let prefix = client.get_prefix(1).await?;
//! ```
```

## Error Module Pattern

**Every crate with errors should have an `error.rs` module:**

```rust
//! Error types for this crate.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CrateError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("API error: {0}")]
    Api(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
```

## Implementation Workflow

### Step-by-Step: Creating a New Crate

1. **Create crate directory and Cargo.toml**
2. **Create `src/` directory**
3. **Create `lib.rs` with module declarations** (empty modules are fine)
4. **Create each module file** with `todo!()` placeholders
5. **Add module documentation** to each file
6. **Implement modules one at a time**
7. **Write tests as you implement** - Don't wait until the end
8. **Verify coverage** - Run `just test-coverage` regularly
9. **Verify functionality** - Not just compilation, actually test it works
7. **Write tests as you implement** - Don't wait until the end
8. **Verify coverage** - Run `just test-coverage` regularly
9. **Verify functionality** - Not just compilation, actually test it works

### Example Workflow

```bash
# 1. Create crate
mkdir -p crates/my-crate/src

# 2. Create Cargo.toml (with dependencies)

# 3. Create lib.rs with structure
cat > crates/my-crate/src/lib.rs << 'EOF'
//! My crate description

pub mod error;
pub mod client;
pub mod models;

pub use error::*;
pub use client::*;
pub use models::*;
EOF

# 4. Create module files
touch crates/my-crate/src/{error,client,models}.rs

# 5. Add documentation and placeholders to each
```

## Anti-Patterns to Avoid

### ❌ Monolithic Files

```rust
// BAD: Everything in one file
// lib.rs (2000 lines)
pub struct Error { }
pub struct Client { }
pub struct Model1 { }
pub struct Model2 { }
// ... 2000 lines
```

### ❌ Generic Module Names

```rust
// BAD: Generic, unclear purpose
pub mod util;
pub mod common;
pub mod helper;
pub mod misc;
```

### ❌ Waiting to Split

```rust
// BAD: "I'll split this later"
// lib.rs - 800 lines, "I'll refactor it next week"
```

### ❌ No Module Documentation

```rust
// BAD: No module docs
pub mod client;

// GOOD: Has module docs
//! Client implementation for API interactions.
pub mod client;
```

## Code Review Checklist for Agents

When reviewing agent-generated code, check:

- [ ] **Module structure exists** - Not everything in one file
- [ ] **Module size** - No module > 500 lines
- [ ] **Module documentation** - Every module has `//!` docs
- [ ] **Clear responsibilities** - Each module has one purpose
- [ ] **Error module** - Errors in dedicated `error.rs` if needed
- [ ] **Re-exports** - `lib.rs` properly re-exports public items
- [ ] **No generic names** - No `util`, `common`, `helper` modules
- [ ] **Tests exist** - All public APIs have tests
- [ ] **Test coverage** - Minimum 65%, target 80% (run `just test-coverage`)
- [ ] **Functionality verified** - Not just compilation, actually works
- [ ] **Controller verification** - For controllers, CRs verified to reconcile
- [ ] **Database verification** - For NetBox resources, verified in database

## Examples from DCops

### ✅ Good: `netbox-client`

```
crates/netbox-client/
├── Cargo.toml
└── src/
    ├── lib.rs      # Re-exports
    ├── error.rs    # Error types
    ├── client.rs   # Client implementation
    └── models.rs   # Data structures
```

### ✅ Good: `crds`

```
crates/crds/
├── Cargo.toml
└── src/
    ├── lib.rs          # Re-exports
    ├── boot_profile.rs # One CRD per file
    ├── boot_intent.rs
    ├── ip_pool.rs
    └── ip_claim.rs
```

### ✅ Good: Controller Structure (Target)

```
controllers/pxe-intent/
├── Cargo.toml
└── src/
    ├── main.rs      # Entry point
    ├── controller.rs
    ├── reconciler.rs
    ├── watcher.rs
    └── error.rs
```

## Enforcement

Agents must:
1. **Always create module structure first** - Before any implementation
2. **Never create files > 500 lines** - Split immediately
3. **Always add module documentation** - `//!` docs required
4. **Follow standard patterns** - Use established structures
5. **Never claim code works just because it compiles** - Must verify functionality
6. **Write tests with adequate coverage** - Minimum 65%, target 80%
7. **Verify controller reconciliation** - Use verification scripts for NetBox CRs

## Critical Rule: Compilation ≠ Working

> **MANDATORY:** Code that compiles successfully is NOT considered working. You MUST verify functionality before claiming completion.

### Verification Requirements

**Before claiming code is complete, verify:**

1. ✅ **Compilation** - `cargo check` passes
2. ✅ **Tests pass** - `cargo test` passes
3. ✅ **Test coverage** - Minimum 65% coverage, target 80%
4. ✅ **Integration works** - For controllers, CRs actually reconcile
5. ✅ **Database verification** - For NetBox resources, they exist in the database

### For NetBox Controllers

After implementing reconciliation logic:

```bash
# 1. Verify CRD exists
kubectl get crd netboxprefixes.dcops.microscaler.io

# 2. Verify CR has status
kubectl get netboxprefix default/control-plane-prefix -o jsonpath='{.status}'

# 3. Verify resource in NetBox database
python3 scripts/verify_netbox_crs.py --crd netboxprefixes --name control-plane-prefix

# Or verify all CRs
just verify-netbox-crs
```

**Never claim reconciliation works just because:**
- ❌ Code compiles
- ❌ Controller starts without errors
- ❌ No obvious errors in logs

**Always verify:**
- ✅ CR status has `netboxId` populated
- ✅ Resource exists in NetBox database
- ✅ Status state is `Created`
- ✅ No errors in status

### Test Coverage Requirements

- **Minimum:** 65% coverage
- **Target:** 80% coverage
- **Tool:** `cargo-llvm-cov` (LLVM-based coverage)
- **Command:** `just test-coverage`

**Coverage must be verified before:**
- Marking a feature as complete
- Submitting code for review
- Claiming code is working

## Questions?

If unsure about module organization:
1. Check existing crates (`netbox-client`, `crds`) for patterns
2. Follow the standard patterns in this document
3. **When in doubt, create more modules, not fewer**

**Remember:** Starting with proper modules is free. Refactoring later is expensive.

**Remember:** Compilation is the first step, not the last. Always verify functionality.

---

## CRD and CR Verification Workflow

When implementing or modifying NetBox CRDs and their corresponding CRs, agents must verify the complete reconciliation flow:

### Verification Checklist

For **every CRD and its corresponding CR**, verify:

1. ✅ **CRD exists in Kubernetes**
2. ✅ **CR has been created and reconciled**
3. ✅ **CR has status populated**
4. ✅ **Resource exists in NetBox database**

### Step-by-Step Verification

#### 1. Verify CRD Exists

**Quick commands:**
```bash
# List all NetBox CRDs
kubectl get crd | grep netbox

# Check specific CRD
kubectl get crd netboxprefixes.dcops.microscaler.io -o yaml
```

**Or use the quick verification script:**
```bash
./scripts/verify_netbox_crs_quick.sh
```

**Expected:** CRD should exist with proper schema and status subresource enabled.

#### 2. Verify CR Exists and Has Status

```bash
# List all CRs of a type
kubectl get netboxprefixes -A

# Get specific CR with status
kubectl get netboxprefix default/control-plane-prefix -o yaml

# Check status specifically
kubectl get netboxprefix default/control-plane-prefix -o jsonpath='{.status}'
```

**Expected:**
- CR should exist
- `status.netboxId` should be populated (non-null)
- `status.state` should be `Created`
- `status.netboxUrl` should be populated
- `status.lastReconciled` should have a timestamp
- `status.error` should be null/empty if successful

#### 3. Verify Resource in NetBox Database

Use the PostgreSQL database query pattern to verify the resource actually exists in NetBox:

```bash
# Get PostgreSQL pod
POSTGRES_POD=$(kubectl get pod -n netbox -l app=postgres -o jsonpath='{.items[0].metadata.name}')

# Query for a prefix (example)
kubectl exec -n netbox $POSTGRES_POD -- psql -U netbox -d netbox -c \
  "SELECT id, prefix, status, description FROM ipam_prefix WHERE prefix = '192.168.1.0/24';"

# Query for a tenant (example)
kubectl exec -n netbox $POSTGRES_POD -- psql -U netbox -d netbox -c \
  "SELECT id, name, slug FROM tenancy_tenant WHERE name = 'Data Center Operations';"

# Query for a site (example)
kubectl exec -n netbox $POSTGRES_POD -- psql -U netbox -d netbox -c \
  "SELECT id, name, slug, status FROM dcim_site WHERE name = 'datacenter-1';"
```

**Expected:** Database query should return a row with matching data.

### Database Table Mapping

Common NetBox tables for verification:

| CRD Type | NetBox Table | Key Fields |
|----------|--------------|------------|
| `NetBoxPrefix` | `ipam_prefix` | `id`, `prefix`, `status`, `description` |
| `NetBoxTenant` | `tenancy_tenant` | `id`, `name`, `slug`, `description` |
| `NetBoxSite` | `dcim_site` | `id`, `name`, `slug`, `status`, `description` |
| `NetBoxRole` | `ipam_role` | `id`, `name`, `slug`, `description` |
| `NetBoxTag` | `extras_tag` | `id`, `name`, `slug`, `color` |
| `NetBoxAggregate` | `ipam_aggregate` | `id`, `prefix`, `rir_id`, `description` |
| `NetBoxVLAN` | `ipam_vlan` | `id`, `vid`, `name`, `site_id`, `status` |
| `NetBoxDeviceRole` | `dcim_devicerole` | `id`, `name`, `slug`, `color` |
| `NetBoxManufacturer` | `dcim_manufacturer` | `id`, `name`, `slug`, `description` |
| `NetBoxPlatform` | `dcim_platform` | `id`, `name`, `slug`, `manufacturer_id` |
| `NetBoxDeviceType` | `dcim_devicetype` | `id`, `manufacturer_id`, `model`, `slug` |
| `NetBoxRegion` | `dcim_region` | `id`, `name`, `slug`, `parent_id` |
| `NetBoxSiteGroup` | `dcim_sitegroup` | `id`, `name`, `slug`, `parent_id` |
| `NetBoxLocation` | `dcim_location` | `id`, `name`, `slug`, `site_id`, `parent_id` |

### Verification Script Pattern

Create a verification script for each CRD type:

### Example Verification Script

A simple example script demonstrating basic verification is available at:
- **`scripts/verify_netbox_prefix_simple.py`** - Simple example for verifying a single NetBoxPrefix CR

This example script demonstrates:
- How to verify CRD exists
- How to check CR status
- How to query NetBox database

**Usage:**
```bash
# Verify default control-plane-prefix
python3 scripts/verify_netbox_prefix_simple.py

# Verify specific CR
python3 scripts/verify_netbox_prefix_simple.py my-prefix default
```

**Note:** For comprehensive verification of all CRs, use `scripts/verify_netbox_crs.py` instead.

### When to Verify

Agents must verify **after**:
1. Creating a new CRD
2. Modifying reconciliation logic
3. Adding a new CR type
4. Fixing reconciliation bugs
5. Before marking a feature as complete

### Common Issues

**Issue:** CR exists but no status
- **Check:** Controller is running and has RBAC permissions
- **Fix:** Ensure RBAC includes the CRD and status subresource

**Issue:** Status exists but `netboxId` is null
- **Check:** Controller logs for reconciliation errors
- **Fix:** Check NetBox API connectivity and token validity

**Issue:** Status has `netboxId` but not in database
- **Check:** NetBox API returned success but resource wasn't created
- **Fix:** Check NetBox API logs and verify create operation succeeded

**Issue:** Resource in database but CR status is wrong
- **Check:** Startup reconciliation logic
- **Fix:** Ensure startup reconciliation maps existing resources correctly

### Quick Verification Commands

Quick bash commands for manual verification are available in:
- **`scripts/verify_netbox_crs_quick.sh`** - Quick verification script with kubectl commands

**Usage:**
```bash
# Run quick verification
./scripts/verify_netbox_crs_quick.sh
```

Or run the commands manually:
```bash
# Verify all NetBox CRDs exist
kubectl get crd | grep netbox

# Verify all CRs have status
for crd in netboxprefixes netboxtenants netboxsites netboxroles netboxtags netboxaggregates netboxvlans; do
  echo "Checking $crd..."
  kubectl get $crd -A -o jsonpath='{range .items[*]}{.metadata.namespace}/{.metadata.name}: {.status.netboxId}{"\n"}{end}'
done

# Verify specific resource in database
POSTGRES_POD=$(kubectl get pod -n netbox -l app=postgres -o jsonpath='{.items[0].metadata.name}')
kubectl exec -n netbox $POSTGRES_POD -- psql -U netbox -d netbox -c "SELECT id, name FROM tenancy_tenant;"
```

### Automated Verification Scripts

**Comprehensive Verification:**
- **`scripts/verify_netbox_crs.py`** - Full-featured verification script for all NetBox CRs

**Usage:**
```bash
# Verify all NetBox CRDs and CRs
python3 scripts/verify_netbox_crs.py --all

# Verify specific CRD type
python3 scripts/verify_netbox_crs.py --crd netboxprefixes

# Verify specific CR
python3 scripts/verify_netbox_crs.py --crd netboxprefixes --name control-plane-prefix

# Verify with custom namespaces
python3 scripts/verify_netbox_crs.py --all --namespace default --netbox-namespace netbox

# Or use the justfile command
just verify-netbox-crs
```

**Available Verification Scripts:**
- **`scripts/verify_netbox_crs.py`** - Comprehensive verification (recommended)
- **`scripts/verify_netbox_prefix_simple.py`** - Simple example for single CR
- **`scripts/verify_netbox_crs_quick.sh`** - Quick bash commands

The comprehensive script automatically:
- ✅ Verifies CRD exists
- ✅ Checks CR has status with netboxId
- ✅ Queries NetBox database to confirm resource exists
- ✅ Reports comprehensive status for all resources

**Example output:**
```
============================================================
Verifying All NetBox CRDs
============================================================
ℹ️  Using PostgreSQL pod: postgres-7d8f9c4b5-abc123

============================================================
Verifying netboxprefixes
============================================================
✅ CRD netboxprefixes.dcops.microscaler.io exists

  Checking default/control-plane-prefix...
✅ CR default/control-plane-prefix has status (netboxId: 1, state: Created)
✅ Resource exists in NetBox database (ID: 1, prefix: 192.168.1.0/24)

============================================================
✅ All verifications passed!
```

### Integration with Tilt

When developing with Tilt, verification should happen automatically:

1. Tilt applies CRDs via `generate-crds` resource
2. Tilt applies example CRs
3. Controller reconciles CRs
4. **Agent verifies** all CRs have status and exist in NetBox

Add verification as a Tilt local_resource if needed for continuous validation.

## CRD Generation and Management

### ⚠️ Critical: CRDs are Ephemeral

**IMPORTANT:** CRDs in `config/crd/all-crds.yaml` are **ephemeral** and **automatically generated** from Rust code. They should **never** be manually edited.

### How CRD Generation Works

1. **Source of Truth:** CRD definitions are in `crates/crds/src/` (Rust code)
2. **Generation:** CRDs are generated by running `cargo run -p crds --bin crdgen`
3. **Output:** Generated YAML is written to `config/crd/all-crds.yaml`
4. **Tilt Integration:** Tilt automatically regenerates CRDs when:
   - CRD code in `crates/crds/src/` changes
   - `crates/crds/Cargo.toml` changes
   - `scripts/generate_crds.py` changes

### Rules for Working with CRDs

**✅ DO:**
- Edit CRD definitions in `crates/crds/src/` (Rust code)
- Run `python3 scripts/generate_crds.py` to regenerate CRDs
- Let Tilt automatically regenerate CRDs during development
- Commit changes to `crates/crds/src/` (the source code)

**❌ DON'T:**
- Manually edit `config/crd/all-crds.yaml` (it will be overwritten)
- Commit manual changes to `config/crd/all-crds.yaml`
- Assume CRD YAML files are the source of truth
- Try to fix CRD issues by editing YAML directly

### CRD Generation Workflow

When modifying CRDs:

1. **Edit Rust code** in `crates/crds/src/`:
   ```rust
   // crates/crds/src/dcim/netbox_device.rs
   #[derive(CustomResource, ...)]
   pub struct NetBoxDeviceSpec {
       // Your changes here
   }
   ```

2. **Regenerate CRDs**:
   ```bash
   # Manual generation
   python3 scripts/generate_crds.py
   
   # Or via cargo directly
   cargo run -p crds --bin crdgen > config/crd/all-crds.yaml
   ```

3. **Tilt will automatically regenerate** when you run `tilt up`:
   - Tilt watches `crates/crds/src/` for changes
   - Automatically runs `generate-crds` resource
   - Applies updated CRDs to the cluster

### Why CRDs are Ephemeral

- **Single Source of Truth:** Rust code is the authoritative definition
- **Type Safety:** Rust types ensure consistency between code and CRDs
- **Automatic Updates:** Tilt ensures CRDs stay in sync with code
- **No Drift:** Prevents manual YAML edits from diverging from code

### Troubleshooting CRD Issues

If CRDs aren't working as expected:

1. **Check the source code** in `crates/crds/src/` - this is what matters
2. **Regenerate CRDs** manually: `python3 scripts/generate_crds.py`
3. **Check Tilt logs** for `generate-crds` resource errors
4. **Verify CRD code compiles**: `cargo check -p crds`
5. **Never edit YAML directly** - fix the Rust code instead

