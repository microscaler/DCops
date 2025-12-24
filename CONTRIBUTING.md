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

### Code Review Checklist

- [ ] Module structure is clear and logical
- [ ] No module exceeds 500 lines
- [ ] Each module has a single, clear responsibility
- [ ] All public items are documented
- [ ] Error types are properly structured
- [ ] Tests are included and passing
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

- **Pre-commit hooks:** Check for module size limits
- **CI/CD:** Fail builds if any module exceeds 500 lines
- **Code review:** Reject PRs that add large monolithic files

## Questions?

If you're unsure about module organization:
1. Check existing crates for patterns (`netbox-client`, `crds`)
2. Ask in code review
3. When in doubt, **create more modules, not fewer**

Remember: **It's cheaper to have too many small modules than one huge file.**

