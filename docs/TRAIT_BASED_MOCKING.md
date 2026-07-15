# Trait-Based Kubernetes API Mocking

## Overview

This document explains the trait-based approach for enabling unit testing of reconcilers
while preserving full real cluster operation.

## Architecture

### Components

1. **`KubeApiTrait<T>`** - Trait that abstracts Kubernetes API operations
   - `get(name)` - Get a resource by name
   - `patch_status(name, params, patch)` - Patch the status subresource
   - `list(params)` - List resources

2. **`KubeApiWrapper<T>`** - Wraps real `kube::Api<T>` for production use
   - Thin wrapper that delegates all calls to the underlying `Api<T>`
   - Zero performance overhead
   - Full compatibility with kube-rs

3. **`MockKubeApi<T>`** - In-memory mock for unit testing
   - Stores resources in a `HashMap`
   - Implements the same trait interface
   - Enables isolated unit testing without a cluster

### Production Flow

```
Controller::new()
  └─> Creates real Api<T> instances
  └─> Wraps each in KubeApiWrapper<T>
  └─> Passes to Reconciler::new()
  └─> Reconciler uses Box<dyn KubeApiTrait<T>>
  └─> All calls delegate to real Api<T> via wrapper
  └─> Real Kubernetes cluster operations work normally
```

### Test Flow

```
create_test_reconciler()
  └─> Creates MockKubeApi<T> instances
  └─> Passes to Reconciler::new()
  └─> Reconciler uses Box<dyn KubeApiTrait<T>>
  └─> All calls go to in-memory mocks
  └─> No cluster required
```

## Critical: Real Cluster Operation

**The trait-based approach does NOT replace real cluster operation.**

### Production Code Path

- **Controller**: Creates real `Api<T>` instances from `kube::Client`
- **Wrapper**: `KubeApiWrapper` is a thin delegation layer
- **Delegation**: All trait method calls forward directly to `Api<T>`
- **Performance**: Zero overhead - just a function call indirection
- **Compatibility**: 100% compatible with kube-rs behavior

### What Changed

**Before:**
```rust
pub struct Reconciler {
    netbox_prefix_api: Api<NetBoxPrefix>,
    // ...
}
```

**After:**
```rust
pub struct Reconciler {
    netbox_prefix_api: Box<dyn KubeApiTrait<NetBoxPrefix>>,
    // ...
}
```

**Production Usage:**
```rust
// Real Api<T> wrapped in KubeApiWrapper
let api: Api<NetBoxPrefix> = Api::namespaced(kube_client, "default");
let reconciler = Reconciler::new(
    netbox_client,
    KubeApiWrapper::new(api), // Wraps real Api<T>
    // ...
);
```

**Test Usage:**
```rust
// Mock implementation
let reconciler = create_test_reconciler(mock_netbox_client);
// Uses MockKubeApi internally
```

## Verification

### Real Cluster Operation

1. **Controller initialization**: Creates real `Api<T>` instances
2. **Wrapper delegation**: All calls forward to real `Api<T>`
3. **Watcher**: Still uses real `Api<T>` directly (unchanged)
4. **Integration tests**: Continue to work with real clusters

### Unit Testing

1. **Mock creation**: `MockKubeApi::new()` creates in-memory store
2. **Resource storage**: `mock_api.store(name, resource)` for test setup
3. **Trait implementation**: Same interface as real wrapper
4. **Isolation**: No cluster required

## Benefits

1. **Unit Testing**: Fast, isolated tests without cluster
2. **Real Cluster**: Full production functionality preserved
3. **No Performance Impact**: Wrapper is just a function call
4. **Type Safety**: Same trait interface for both real and mock
5. **Maintainability**: Single code path for reconciler logic

## Files

- `controllers/netbox/src/kube_api_trait.rs` - Trait and wrapper definitions
- `controllers/netbox/src/kube_api_trait/mock.rs` - Mock implementation
- `controllers/netbox/src/reconciler/mod.rs` - Reconciler using trait
- `controllers/netbox/src/controller.rs` - Production wrapper usage
- `controllers/netbox/src/test_utils.rs` - Test utilities with mocks

