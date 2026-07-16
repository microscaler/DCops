# Code Style

DCops follows Rust best practices and project-specific guidelines.

## Formatting

Always format code before committing:

```bash
cargo fmt
```

This ensures consistent formatting across the codebase.

## Linting

Run clippy to catch common issues:

```bash
cargo clippy
```

Fix all warnings before submitting PRs.

## Project Guidelines

DCops has specific guidelines in `rust-guidelines.txt`:

📖 **See [`rust-guidelines.txt`](../../../../rust-guidelines.txt) for complete guidelines.**

Key points:
- Follow Rust naming conventions
- Use meaningful variable names
- Add comments for complex logic
- Keep functions focused and small
- Use type-safe enums for states
- Prefer trait-based mocking for tests

## Code Organization

### Controller Structure

```
controllers/netbox/src/
├── main.rs              # Entry point, watcher setup
├── controller.rs       # Controller logic
├── reconciler/          # Resource reconcilers
│   ├── mod.rs          # Main reconciler
│   ├── dcim/           # DCIM resources
│   ├── ipam/           # IPAM resources
│   └── tenancy.rs      # Tenancy resources
├── error.rs            # Error types
├── events.rs           # Event emission
└── reconcile_helpers.rs # Shared reconciliation helpers
```

### CRD Structure

```
crates/crds/src/
├── lib.rs              # Re-exports
├── dcim/               # DCIM CRDs
├── ipam/               # IPAM CRDs
├── tenancy/            # Tenancy CRDs
└── references.rs       # Common reference types
```

## Naming Conventions

### CRDs

- Use `NetBox` prefix for NetBox resources: `NetBoxSite`, `NetBoxDevice`
- Use descriptive names: `IPPool`, `IPClaim`
- Match NetBox API naming where possible

### Functions

- Use `snake_case` for functions
- Use descriptive names: `reconcile_netbox_site` not `reconcile_site`
- Prefix helpers: `create_resource_status_patch`

### Types

- Use `PascalCase` for types
- Use descriptive names: `NetBoxResourceReference`
- Use enums for states: `ResourceState`, `PrefixState`

## Error Handling

### Error Types

Use `ControllerError` for all controller errors:

```rust
use crate::error::ControllerError;

fn my_function() -> Result<(), ControllerError> {
    // ...
}
```

### Error Propagation

Use `?` operator for error propagation:

```rust
let result = some_operation().await?;
```

### Error Messages

Provide clear, actionable error messages:

```rust
Err(ControllerError::NetBox(
    NetBoxError::NotFound(format!("Site {} not found", site_name))
))
```

## Testing

### Test Organization

- Unit tests in same file: `mod tests { ... }`
- Integration tests in `*_test.rs` files
- Use trait-based mocking

### Test Naming

- Use descriptive names: `test_netbox_site_creates_in_netbox`
- Group related tests: `test_netbox_site_*`

### Test Structure

```rust
#[tokio::test]
async fn test_netbox_site_creates_in_netbox() {
    // Arrange
    let (apis, mock_token_resolver) = setup_test_environment();
    
    // Act
    let result = reconcile_netbox_site(&site_crd).await;
    
    // Assert
    assert!(result.is_ok());
    // ...
}
```

## Documentation

### Doc Comments

Add doc comments to public APIs:

```rust
/// Reconciles a NetBoxSite resource.
///
/// This function:
/// 1. Resolves dependencies (tenant, region)
/// 2. Creates or updates the site in NetBox
/// 3. Updates the CRD status
///
/// # Errors
///
/// Returns `ControllerError` if reconciliation fails.
pub async fn reconcile_netbox_site(&self, site: &NetBoxSite) -> Result<(), ControllerError> {
    // ...
}
```

### Inline Comments

Add comments for complex logic:

```rust
// Use tenant-specific token for NetBox API calls
// This ensures proper multi-tenant isolation
let token = resolve_tenant_token(&tenant_ref).await?;
```

## Resources

- [Rust Guidelines](../../../../rust-guidelines.txt) - Complete project guidelines
- [Development Setup](../development/setup.md) - Development environment
- [Testing](../development/testing.md) - Testing practices
- [Contributing Guide](./contributing-guide.md) - Contribution process
