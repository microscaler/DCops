# IP Address Usage Audit

## Executive Summary

This document audits all IP address and prefix usage throughout the codebase, documenting current string-based implementations and proposing migration to the `ipnet` crate for type-safe IP network handling.

**Current State**: All IP addresses and prefixes are represented as `String` types (e.g., `"192.168.1.0/24"`, `"192.168.1.2/24"`)

**Proposed State**: Use `ipnet::IpNet` for IP networks/prefixes and `ipnet::Ipv4Net`/`ipnet::Ipv6Net` for type-specific networks

**Migration Priority**: High - Type safety, validation, and proper network operations are critical for IPAM functionality

---

## 1. NetBox Client Models (`crates/netbox-client/src/models.rs`)

### Current Implementation

| Field/Struct | Type | Usage | Example |
|-------------|------|-------|---------|
| `Prefix.prefix` | `String` | CIDR notation prefix | `"192.168.1.0/24"` |
| `IPAddress.address` | `String` | IP address with CIDR | `"192.168.1.2/24"` |
| `AvailableIP.address` | `String` | Available IP from prefix | `"192.168.1.1/24"` |
| `AllocateIPRequest.address` | `Option<String>` | Optional specific IP to allocate | `Some("192.168.1.10/24")` |
| `Aggregate.prefix` | `String` | Aggregate prefix | `"192.168.0.0/16"` |

### Proposed Implementation

| Field/Struct | Current | Proposed | Migration Notes |
|-------------|---------|----------|-----------------|
| `Prefix.prefix` | `String` | `ipnet::IpNet` | Use custom serde serializer/deserializer for NetBox API compatibility |
| `IPAddress.address` | `String` | `ipnet::IpNet` | NetBox returns IP with CIDR, parse as IpNet |
| `AvailableIP.address` | `String` | `ipnet::IpNet` | Parse from NetBox response |
| `AllocateIPRequest.address` | `Option<String>` | `Option<ipnet::IpNet>` | Convert to string for API call |
| `Aggregate.prefix` | `String` | `ipnet::IpNet` | Same as Prefix |

**Migration Strategy**:
- Add custom `Serialize`/`Deserialize` implementations that convert `IpNet` ↔ `String` for NetBox API
- Keep `family: u8` field for backward compatibility (can derive from `IpNet`)

---

## 2. NetBox Client API Functions (`crates/netbox-client/src/`)

### Current Implementation

| Function | Parameter | Type | Usage |
|----------|-----------|------|-------|
| `create_prefix()` | `prefix` | `&str` | Create prefix in NetBox |
| `update_prefix()` | `prefix` | `Option<&str>` | Update prefix CIDR |
| `create_ip_address()` | `address` | `&str` | Create IP address |
| `create_aggregate()` | `prefix` | `&str` | Create aggregate |
| `allocate_ip()` | `request.address` | `Option<String>` | Optional specific IP |

### Proposed Implementation

| Function | Current | Proposed | Migration Notes |
|----------|---------|----------|-----------------|
| `create_prefix()` | `&str` | `&ipnet::IpNet` | Convert to string for API: `prefix.to_string()` |
| `update_prefix()` | `Option<&str>` | `Option<&ipnet::IpNet>` | Convert to string for API |
| `create_ip_address()` | `&str` | `&ipnet::IpNet` | Convert to string for API |
| `create_aggregate()` | `&str` | `&ipnet::IpNet` | Convert to string for API |
| `allocate_ip()` | `Option<String>` | `Option<ipnet::IpNet>` | Convert to string for API |

**Migration Strategy**:
- Change function signatures to accept `&ipnet::IpNet` or `ipnet::IpNet`
- Convert to string only when making HTTP requests to NetBox API
- Internal logic uses typed `IpNet` for validation and operations

---

## 3. NetBox Client Trait (`crates/netbox-client/src/trait.rs`)

### Current Implementation

All trait methods use `&str` for IP addresses and prefixes.

### Proposed Implementation

| Trait Method | Current | Proposed |
|-------------|---------|----------|
| `create_prefix()` | `prefix: &str` | `prefix: &ipnet::IpNet` |
| `update_prefix()` | `prefix: Option<&str>` | `prefix: Option<&ipnet::IpNet>` |
| `create_ip_address()` | `address: &str` | `address: &ipnet::IpNet` |
| `create_aggregate()` | `prefix: &str` | `prefix: &ipnet::IpNet` |

**Migration Strategy**:
- Update trait definition first (breaking change)
- Update all implementations (NetBoxClient, MockNetBoxClient, MockNetBoxClientWrapper)
- Update all call sites

---

## 4. Kubernetes CRDs (`crates/crds/src/`)

### Current Implementation

| CRD Field | Type | Usage | Example |
|-----------|------|-------|---------|
| `NetBoxPrefixSpec.prefix` | `String` | CIDR prefix | `"192.168.1.0/24"` |
| `NetBoxAggregateSpec.prefix` | `String` | Aggregate prefix | `"192.168.0.0/16"` |
| `IPClaimSpec.preferred_ip` | `Option<String>` | Preferred IP hint | `Some("192.168.1.10/24")` |
| `IPClaimStatus.ip` | `Option<String>` | Allocated IP | `Some("192.168.1.2/24")` |
| `NetBoxDeviceSpec.primary_ip4` | `Option<String>` | Direct IP fallback | `Some("192.168.1.10/24")` |
| `NetBoxDeviceSpec.primary_ip6` | `Option<String>` | Direct IP fallback | `Some("2001:db8::1/64")` |

### Proposed Implementation

**Decision Required**: CRDs must serialize to JSON/YAML for Kubernetes API. Options:

1. **Keep as String in CRD** (Recommended for Phase 1)
   - CRDs remain `String` for Kubernetes API compatibility
   - Convert to `IpNet` immediately after deserialization
   - Validate using `IpNet::from_str()` in reconciler

2. **Custom JSON Schema with Validation** (Future)
   - Use `schemars` custom schema to validate CIDR format
   - Still serialize as string for Kubernetes API
   - Add validation in `kubebuilder` annotations

**Migration Strategy**:
- **Phase 1**: Keep CRD fields as `String`, validate and convert to `IpNet` in reconciler
- **Phase 2**: Add JSON schema validation for CIDR format
- **Phase 3**: Consider custom types with string serialization (if needed)

---

## 5. Reconciler Logic (`controllers/netbox/src/reconciler/`)

### Current Implementation

| Location | Operation | Current | Issue |
|----------|-----------|---------|-------|
| `prefix.rs:357` | Query prefixes | `&prefix_crd.spec.prefix` (String) | String comparison |
| `prefix.rs:361` | Find existing | `p.prefix == prefix_crd.spec.prefix` | String equality |
| `prefix.rs:373` | Find in all | `p.prefix == prefix_crd.spec.prefix` | String equality |
| `ip_pool.rs:101` | Query IPs | `&prefix.prefix` (String) | String filter |
| `ip_claim.rs:242` | Find IP | `ip.address == *preferred_ip` | String equality |
| `ip_claim.rs:291` | Log allocated | `allocated_ip.address` | String logging |

### Proposed Implementation

| Location | Current | Proposed | Benefit |
|----------|---------|----------|---------|
| `prefix.rs` | String comparison | `IpNet` comparison | Normalized comparison (192.168.1.0/24 == 192.168.001.0/24) |
| `ip_pool.rs` | String filter | `IpNet.contains()` | Proper network containment |
| `ip_claim.rs` | String equality | `IpNet` equality | Type-safe comparison |

**Migration Strategy**:
1. Convert CRD `String` to `IpNet` at start of reconcile function
2. Use `IpNet` for all internal operations
3. Convert back to `String` only for API calls and status updates

**Example**:
```rust
// Current
let prefix_str = &prefix_crd.spec.prefix;
let existing = prefixes.iter().find(|p| p.prefix == prefix_str);

// Proposed
let prefix_net = ipnet::IpNet::from_str(&prefix_crd.spec.prefix)?;
let existing = prefixes.iter().find(|p| {
    ipnet::IpNet::from_str(&p.prefix)
        .map(|net| net == prefix_net)
        .unwrap_or(false)
});
```

---

## 6. Test Utilities (`controllers/netbox/src/test_utils.rs`)

### Current Implementation

| Function | Parameter | Type | Usage |
|----------|-----------|------|-------|
| `create_test_prefix()` | `prefix` | `&str` | Create test Prefix model |
| `create_test_ip_claim()` | `preferred_ip` | `Option<&str>` | Create test IPClaim |

### Proposed Implementation

| Function | Current | Proposed | Migration Notes |
|----------|---------|----------|-----------------|
| `create_test_prefix()` | `&str` | `&ipnet::IpNet` or keep `&str` | If keep `&str`, validate internally |
| `create_test_ip_claim()` | `Option<&str>` | `Option<&ipnet::IpNet>` or keep | Same as above |

**Migration Strategy**:
- Option A: Keep `&str` in test helpers, convert internally
- Option B: Change to `&IpNet` for type safety in tests

---

## 7. Mock Implementation (`crates/netbox-client/src/mock/`)

### Current Implementation

| Location | Operation | Current | Status |
|----------|-----------|---------|--------|
| `mock/ipam.rs:89-122` | `query_ip_addresses()` filter | **UPDATED** | Now uses `ipnet::IpNet::from_str()` and `contains()` |
| `mock/ipam.rs:211` | `create_prefix()` | `&str` | Needs update |
| `mock/ipam.rs:262` | `update_prefix()` | `Option<&str>` | Needs update |
| `mock/ipam.rs:328` | `create_aggregate()` | `&str` | Needs update |

### Proposed Implementation

| Location | Current | Proposed | Priority |
|----------|---------|----------|----------|
| `query_ip_addresses()` | ✅ Updated | ✅ Complete | Done |
| `create_prefix()` | `&str` | `&ipnet::IpNet` | High |
| `update_prefix()` | `Option<&str>` | `Option<&ipnet::IpNet>` | High |
| `create_aggregate()` | `&str` | `&ipnet::IpNet` | Medium |

**Migration Strategy**:
- Update mock functions to accept `IpNet`
- Store as `String` internally (for compatibility with models)
- Use `IpNet` for filtering and validation

---

## 8. IP Network Operations

### Current String-Based Operations

| Operation | Current Implementation | Issue |
|-----------|----------------------|-------|
| Network containment | `ip.address.starts_with(prefix)` | Incorrect for IPs like `192.168.10.1/24` in `192.168.1.0/24` |
| Prefix comparison | `prefix1 == prefix2` | Doesn't normalize (192.168.001.0/24 != 192.168.1.0/24) |
| IP extraction | `ip.address.split('/').next()` | Manual parsing, error-prone |
| Family detection | `prefix.contains(':')` | String-based, unreliable |

### Proposed IpNet-Based Operations

| Operation | Current | Proposed | Benefit |
|-----------|---------|----------|---------|
| Network containment | `starts_with()` | `prefix_net.contains(&ip_addr)` | ✅ Correct network math |
| Prefix comparison | `==` | `net1 == net2` | ✅ Normalized comparison |
| IP extraction | `split('/')` | `ip_net.addr()` | ✅ Type-safe |
| Family detection | `contains(':')` | `ip_net.is_ipv4()` / `is_ipv6()` | ✅ Reliable |
| Network size | Manual calculation | `ip_net.prefix_len()` | ✅ Built-in |
| Host count | Manual calculation | `ip_net.hosts().count()` | ✅ Built-in |

---

## Migration Plan

### Phase 1: Foundation (Current - ✅ Complete)
- [x] Add `ipnet` crate to dependencies
- [x] Update `query_ip_addresses()` mock filter to use `ipnet`
- [x] Fix tests to work with proper IP network checking

### Phase 2: Models (High Priority)
- [ ] Add custom serde serializers for `IpNet` in models
- [ ] Update `Prefix.prefix` to `ipnet::IpNet` with string serialization
- [ ] Update `IPAddress.address` to `ipnet::IpNet` with string serialization
- [ ] Update `AvailableIP.address` to `ipnet::IpNet` with string serialization
- [ ] Update `AllocateIPRequest.address` to `Option<ipnet::IpNet>`
- [ ] Update `Aggregate.prefix` to `ipnet::IpNet` with string serialization

### Phase 3: API Functions (High Priority)
- [ ] Update `NetBoxClientTrait` to accept `&ipnet::IpNet` for prefix/IP parameters
- [ ] Update `NetBoxClient` implementation
- [ ] Update `MockNetBoxClient` implementation
- [ ] Update `MockNetBoxClientWrapper` implementation
- [ ] Convert `IpNet` to `String` only at HTTP request boundary

### Phase 4: Reconciler Logic (High Priority)
- [ ] Convert CRD `String` fields to `IpNet` at start of reconcile functions
- [ ] Replace string comparisons with `IpNet` comparisons
- [ ] Use `IpNet.contains()` for network containment checks
- [ ] Update logging to use `IpNet::to_string()` for consistency

### Phase 5: CRD Validation (Medium Priority)
- [ ] Add JSON schema validation for CIDR format in CRDs
- [ ] Add validation in reconciler using `IpNet::from_str()`
- [ ] Return clear error messages for invalid CIDR

### Phase 6: Test Utilities (Low Priority)
- [ ] Update test helpers to use `IpNet` or validate strings
- [ ] Add helper functions for creating `IpNet` in tests

---

## Benefits of Migration

1. **Type Safety**: Compile-time validation of IP addresses and prefixes
2. **Correctness**: Proper network containment checks (no more string matching bugs)
3. **Normalization**: Automatic normalization (192.168.001.0/24 == 192.168.1.0/24)
4. **Rich API**: Built-in methods for network operations (contains, hosts, prefix_len, etc.)
5. **Error Prevention**: Invalid IPs caught at parse time, not runtime
6. **Better Testing**: Type-safe test data creation

---

## Risks and Considerations

1. **Breaking Changes**: Trait and function signature changes will require updates across codebase
2. **Serialization**: Need custom serde implementations for NetBox API compatibility
3. **CRD Compatibility**: Must maintain string serialization for Kubernetes API
4. **Migration Effort**: Significant refactoring across multiple layers
5. **Testing**: All tests need updates to use new types

---

## Recommendations

1. **Start with Phase 2 (Models)**: This provides the foundation for all other changes
2. **Use Custom Serde**: Implement `Serialize`/`Deserialize` for `IpNet` that converts to/from strings
3. **Gradual Migration**: Update one module at a time, ensuring tests pass at each step
4. **Keep CRDs as String**: For Kubernetes API compatibility, validate and convert in reconciler
5. **Add Validation**: Use `IpNet::from_str()` with proper error handling throughout

---

## Files Requiring Changes

### High Priority
- `crates/netbox-client/src/models.rs` - Model definitions
- `crates/netbox-client/src/trait.rs` - Trait definitions
- `crates/netbox-client/src/client.rs` - Client implementation
- `crates/netbox-client/src/mock/mod.rs` - Mock implementation
- `controllers/netbox/src/reconciler/ipam/prefix.rs` - Prefix reconciler
- `controllers/netbox/src/reconciler/ipam/ip_pool.rs` - IP pool reconciler
- `controllers/netbox/src/reconciler/ipam/ip_claim.rs` - IP claim reconciler

### Medium Priority
- `crates/netbox-client/src/ipam/prefix.rs` - Prefix API functions
- `crates/netbox-client/src/ipam/ip_address.rs` - IP address API functions
- `crates/netbox-client/src/mock/ipam.rs` - Mock IPAM functions
- `controllers/netbox/src/test_utils.rs` - Test utilities

### Low Priority
- `crates/crds/src/ipam/netbox_prefix.rs` - Add validation
- `crates/crds/src/ipam/netbox_aggregate.rs` - Add validation
- `crates/crds/src/ip_claim.rs` - Add validation
- All test files - Update to use new types

---

## Example: Custom Serde Implementation

```rust
use ipnet::IpNet;
use serde::{Deserialize, Deserializer, Serializer};

pub fn serialize_ipnet<S>(ipnet: &IpNet, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&ipnet.to_string())
}

pub fn deserialize_ipnet<'de, D>(deserializer: D) -> Result<IpNet, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    IpNet::from_str(&s).map_err(serde::de::Error::custom)
}

// Usage in model:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prefix {
    pub id: u64,
    #[serde(serialize_with = "serialize_ipnet", deserialize_with = "deserialize_ipnet")]
    pub prefix: IpNet,
    // ... other fields
}
```

---

*Last Updated: 2025-01-28*
*Status: Audit Complete - Ready for Migration Planning*

