# Drift Detection Pattern

## Current Implementation

When drift is detected (a resource was deleted in NetBox), each reconciler follows this pattern:

```rust
Ok(None) => {
    // Drift detected - resource was deleted, clear status and recreate
    warn!("NetBoxResource {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
    let status_patch = Self::create_resource_status_patch(
        0, // Clear netbox_id
        String::new(), // Clear URL
        ResourceState::Pending, // Type-safe enum
        Some("Resource was deleted in NetBox, will recreate".to_string()),
    );
    let pp = kube::api::PatchParams::default();
    if let Err(e) = self.netbox_resource_api
        .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
        .await
    {
        warn!("Failed to clear NetBoxResource status after drift detection: {}", e);
    }
    // Fall through to creation
    None
}
```

## Why Not Use Generic Helpers?

We have generic helper functions in `reconcile_helpers.rs`:
- `create_pending_status_patch()`
- `create_drift_status_patch()`

**These aren't used because:**

1. **Type Safety**: Each CRD has a different state enum:
   - Most CRDs: `ResourceState::Pending`
   - `NetBoxPrefix`: `PrefixState::Pending`
   - `IPClaim`: `AllocationState::Pending`
   - `IPPool`: Different status structure

2. **CRD Schema Validation**: The generic helpers return JSON with hardcoded `"Pending"` string, but CRD validation schemas expect PascalCase enum values that match the specific state enum type.

3. **Type-Specific Methods**: Each reconciler has its own status patch creation method:
   - `create_resource_status_patch()` - for most CRDs
   - `create_prefix_status_patch()` - for `NetBoxPrefix`
   - `create_ipclaim_status_patch()` - for `IPClaim`

## Consistency

All reconcilers follow the same pattern:
1. Call `check_existing()` or `check_and_update_existing()` helper
2. On `Ok(None)` (drift detected), create a type-specific status patch
3. Clear `netboxId` (set to 0)
4. Clear `netboxUrl` (set to empty string)
5. Set state to `Pending` (using type-safe enum)
6. Set error message: "Resource was deleted in NetBox, will recreate"
7. Patch the status
8. Fall through to creation logic

## Future Improvement

We could create a generic helper that accepts the state enum as a parameter:

```rust
pub fn create_drift_status_patch<S: Into<String>>(state: S) -> serde_json::Value {
    serde_json::json!({
        "status": {
            "netboxId": 0,
            "netboxUrl": "",
            "state": state.into(),
            "error": "Resource was deleted in NetBox, will recreate"
        }
    })
}
```

But this would still require each reconciler to pass the correct enum variant, so the current pattern is actually cleaner and more type-safe.

## Conclusion

The current pattern is correct and consistent. The generic helpers exist but aren't used because:
- Type safety is more important than code reuse here
- Each CRD has different status structures
- The current pattern is clear and maintainable

The generic helpers serve as documentation of the pattern, but the type-specific methods ensure correctness.

