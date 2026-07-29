# Kubernetes API Mocking Strategy

## Problem

Reconciler unit tests are blocked because they require `kube::Api<T>` instances to interact with Kubernetes CRDs. The `kube-rs` library doesn't provide built-in mocks, so we need to create a mocking solution.

## Current State

- ✅ NetBoxClient is mocked via `MockNetBoxClient` (trait-based)
- ✅ All test utilities and structures are ready
- ⚠️ Reconciler tests blocked on Kubernetes API mocking

## Recommended Approach: tower-test

Based on research, the recommended approach is to use `tower-test` crate in conjunction with `kube` to create a mock HTTP service that emulates the Kubernetes API server.

### Implementation Steps

1. **Add tower-test dependency** to `controllers/netbox/Cargo.toml`
2. **Create mock HTTP service** using `tower-test::mock::pair`
3. **Wrap in kube::Client** to create Api instances
4. **Set up expected request/response pairs** for each test

### Example Structure

```rust
use tower_test::mock::{self, Handle};
use kube::Client;
use kube::Api;
use http::{Request, Response};
use hyper::Body;

#[tokio::test]
async fn test_reconciler() {
    // Create mock service and handle
    let (mock_service, mut handle) = mock::pair::<Request<Body>, Response<Body>>();
    
    // Create kube client with mock service
    let client = Client::new(mock_service, "default");
    
    // Set up expected API interactions
    // For example, when reconciler calls api.get("resource-name")
    handle
        .expect_request()
        .and_return(Response::builder()
            .status(200)
            .body(Body::from(serde_json::to_string(&test_resource).unwrap()))
            .unwrap());
    
    // Create Api instances
    let api: Api<NetBoxSite> = Api::namespaced(client, "default");
    
    // Create reconciler with mocked APIs
    let reconciler = create_test_reconciler(mock_netbox_client, client, "default");
    
    // Run test
    reconciler.reconcile_netbox_site(&test_site).await.unwrap();
}
```

## Alternative Approaches

### 1. Custom Mock Wrapper

Create a trait-based wrapper around `Api<T>` that can be mocked:

```rust
trait KubeApiTrait<T> {
    async fn get(&self, name: &str) -> Result<T, kube::Error>;
    async fn patch_status(&self, name: &str, params: &PatchParams, patch: &Patch) -> Result<T, kube::Error>;
}
```

**Pros:**
- Simple to implement
- Easy to use in tests

**Cons:**
- Requires refactoring Reconciler to use trait
- Doesn't test actual kube-rs integration

### 2. Integration Tests with Kind

Use a real Kubernetes cluster (Kind) for testing:

**Pros:**
- Tests real kube-rs integration
- No mocking needed

**Cons:**
- Slower tests
- Requires cluster setup
- Less isolated

## Recommended Path Forward

1. **Short-term**: Document approach and create placeholder structure
2. **Medium-term**: Implement tower-test based mocking for high-priority reconcilers
3. **Long-term**: Consider custom mock wrapper if tower-test proves too complex

## Dependencies Needed

Add to `controllers/netbox/Cargo.toml`:

```toml
[dev-dependencies]
tower-test = "0.4"
hyper = "1.0"
http = "1.0"
```

## Current Status

1. ✅ Document strategy (this file)
2. ✅ Add tower-test dependency
3. ✅ Create modular mock service infrastructure
4. ⚠️ **BLOCKED**: kube 2.0 doesn't expose Client construction from service
5. ⏳ Alternative approaches being evaluated

## Implementation: Trait-Based Wrapper

**Status: ✅ IMPLEMENTED**

We've implemented a trait-based wrapper approach that enables unit testing while
preserving full real cluster operation:

1. **`KubeApiTrait<T>`** - Trait that abstracts Kubernetes API operations
2. **`KubeApiWrapper<T>`** - Wraps real `kube::Api<T>` and implements the trait
3. **`MockKubeApi<T>`** - Mock implementation for unit testing
4. **`Reconciler`** - Now uses `Box<dyn KubeApiTrait<T>>` instead of `Api<T>`

### Critical: Real Cluster Operation Preserved

**The trait-based approach does NOT replace real cluster operation.**

- **Production/Integration**: `KubeApiWrapper` wraps real `Api<T>` instances
- **Unit Tests**: `MockKubeApi` provides in-memory mocking
- **Watcher**: Still uses real `Api<T>` directly (not affected by trait)
- **All real cluster functionality remains intact**

The wrapper is a thin layer that delegates all calls to the underlying `Api<T>`,
ensuring zero performance impact and full compatibility with kube-rs.

### Options Moving Forward

1. **Integration Tests (Current Approach)**
   - Use Kind cluster for testing
   - Tests are slower but test real kube-rs integration
   - Already working and reliable

2. **Trait-Based Wrapper (Requires Refactoring)**
   - Create a `KubeApiTrait<T>` that wraps `Api<T>`
   - Refactor `Reconciler` to use the trait
   - Mock the trait in tests
   - **Pros**: True unit tests, fast, isolated
   - **Cons**: Requires significant refactoring

3. **Wait for kube Enhancement**
   - kube may add service-based Client construction in future versions
   - Monitor kube-rs issues/PRs for this feature

4. **Use kube's Internal APIs (Not Recommended)**
   - Access private kube internals (risky, may break)
   - Not a sustainable solution

## Next Steps

1. ⏳ Evaluate trait-based wrapper approach
2. ⏳ Consider integration test improvements
3. ⏳ Monitor kube-rs for service-based Client construction
4. ⏳ Document decision and proceed with chosen approach

## References

- [kube-rs testing documentation](https://kube.rs/controllers/testing/)
- [tower-test crate](https://docs.rs/tower-test/)

