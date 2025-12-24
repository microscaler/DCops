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

## Questions?

If unsure about module organization:
1. Check existing crates (`netbox-client`, `crds`) for patterns
2. Follow the standard patterns in this document
3. **When in doubt, create more modules, not fewer**

**Remember:** Starting with proper modules is free. Refactoring later is expensive.

