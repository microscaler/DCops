# NetBox Client Modular Refactoring Plan

**Date:** 2025-12-25  
**Status:** 🚧 **IN PROGRESS**

## Overview

Refactor the monolithic `client.rs` (3460 lines) into a modular structure with:
- **Modular organization** by NetBox API module (IPAM, DCIM, Tenancy, etc.)
- **Single API per file** for maintainability
- **Reusable trait system** for mocking and testing
- **Comprehensive tests** with positive and negative test cases
- **Test data fixtures** for all edge cases

## Current Structure

```
crates/netbox-client/src/
├── client.rs (3460 lines - monolithic)
├── error.rs
├── models.rs
├── trait.rs (74 methods - incomplete)
└── lib.rs
```

## Target Structure

```
crates/netbox-client/src/
├── client.rs (main client, delegates to modules)
├── common/
│   ├── mod.rs (HttpClient, PaginatedResponse)
│   └── query.rs (query utilities)
├── ipam/
│   ├── mod.rs (IPAM module entry)
│   ├── prefix.rs (prefix operations)
│   ├── ip_address.rs (IP address operations)
│   ├── aggregate.rs (aggregate operations)
│   ├── rir.rs (RIR operations)
│   ├── vlan.rs (VLAN operations)
│   ├── role.rs (role operations)
│   ├── asn.rs (ASN operations)
│   ├── vrf.rs (VRF operations)
│   ├── ip_range.rs (IP range operations)
│   └── service.rs (service operations)
├── dcim/
│   ├── mod.rs (DCIM module entry)
│   ├── site.rs (site operations)
│   ├── device.rs (device operations)
│   ├── interface.rs (interface operations)
│   ├── mac_address.rs (MAC address operations)
│   ├── region.rs (region operations)
│   ├── site_group.rs (site group operations)
│   ├── location.rs (location operations)
│   ├── device_role.rs (device role operations)
│   ├── manufacturer.rs (manufacturer operations)
│   ├── platform.rs (platform operations)
│   └── device_type.rs (device type operations)
├── tenancy/
│   ├── mod.rs (Tenancy module entry)
│   ├── tenant.rs (tenant operations)
│   └── tenant_group.rs (tenant group operations)
├── extras/
│   ├── mod.rs (Extras module entry)
│   ├── tag.rs (tag operations)
│   └── role.rs (role operations)
├── virtualization/ (future)
├── circuits/ (future)
├── vpn/ (future)
├── wireless/ (future)
├── core/ (future)
├── users/ (future)
├── error.rs
├── models.rs
├── trait.rs (complete with all 150+ methods)
└── lib.rs
```

## Implementation Phases

### Phase 1: Foundation ✅ (In Progress)
- [x] Create `common/` module with `HttpClient` and `PaginatedResponse`
- [x] Create `common/query.rs` for query utilities
- [x] Update `lib.rs` to export common modules
- [ ] Fix trait module naming issue
- [ ] Update main client to use `HttpClient`

### Phase 2: IPAM Module
- [ ] Create `ipam/mod.rs`
- [ ] Extract prefix operations to `ipam/prefix.rs`
- [ ] Extract IP address operations to `ipam/ip_address.rs`
- [ ] Extract aggregate operations to `ipam/aggregate.rs`
- [ ] Extract RIR operations to `ipam/rir.rs`
- [ ] Extract VLAN operations to `ipam/vlan.rs`
- [ ] Extract role operations to `ipam/role.rs`
- [ ] Add missing IPAM endpoints (ASN, VRF, IP Range, Service)
- [ ] Update main client to delegate IPAM calls
- [ ] Update trait with all IPAM methods

### Phase 3: DCIM Module
- [ ] Create `dcim/mod.rs`
- [ ] Extract site operations to `dcim/site.rs`
- [ ] Extract device operations to `dcim/device.rs`
- [ ] Extract interface operations to `dcim/interface.rs`
- [ ] Extract MAC address operations to `dcim/mac_address.rs`
- [ ] Extract region operations to `dcim/region.rs`
- [ ] Extract site group operations to `dcim/site_group.rs`
- [ ] Extract location operations to `dcim/location.rs`
- [ ] Extract device role operations to `dcim/device_role.rs`
- [ ] Extract manufacturer operations to `dcim/manufacturer.rs`
- [ ] Extract platform operations to `dcim/platform.rs`
- [ ] Extract device type operations to `dcim/device_type.rs`
- [ ] Update main client to delegate DCIM calls
- [ ] Update trait with all DCIM methods

### Phase 4: Tenancy Module
- [ ] Create `tenancy/mod.rs`
- [ ] Extract tenant operations to `tenancy/tenant.rs`
- [ ] Extract tenant group operations to `tenancy/tenant_group.rs`
- [ ] Update main client to delegate Tenancy calls
- [ ] Update trait with all Tenancy methods

### Phase 5: Extras Module
- [ ] Create `extras/mod.rs`
- [ ] Extract tag operations to `extras/tag.rs`
- [ ] Extract role operations to `extras/role.rs`
- [ ] Update main client to delegate Extras calls
- [ ] Update trait with all Extras methods

### Phase 6: Complete Trait
- [ ] Add all missing methods to trait (150+ total)
- [ ] Ensure trait methods match implementation
- [ ] Document trait methods

### Phase 7: Tests
- [ ] Create test structure: `tests/`
- [ ] Create mock implementation using `mockall`
- [ ] Create test fixtures: `tests/fixtures/`
- [ ] Unit tests for each module
- [ ] Positive test cases
- [ ] Negative test cases (errors, edge cases)
- [ ] Integration tests (optional, require NetBox instance)

## Module Implementation Pattern

Each API file follows this pattern:

```rust
//! Prefix operations for NetBox IPAM API

use crate::common::HttpClient;
use crate::error::NetBoxError;
use crate::models::*;

/// Get a prefix by ID
pub async fn get_prefix(
    http: &HttpClient,
    id: u64,
) -> Result<Prefix, NetBoxError> {
    let path = format!("/api/ipam/prefixes/{}/", id);
    http.get(&path).await
}

/// Query prefixes with filters
pub async fn query_prefixes(
    http: &HttpClient,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<Prefix>, NetBoxError> {
    use crate::common::query::query_resources;
    query_resources(http, "ipam/prefixes", filters, fetch_all).await
}

// ... more operations
```

## Test Structure

```
crates/netbox-client/tests/
├── common/
│   └── mod.rs (test utilities)
├── fixtures/
│   ├── ipam/
│   │   ├── prefix.json
│   │   ├── ip_address.json
│   │   └── ...
│   ├── dcim/
│   │   ├── site.json
│   │   ├── device.json
│   │   └── ...
│   └── ...
├── ipam/
│   ├── prefix_test.rs
│   ├── ip_address_test.rs
│   └── ...
├── dcim/
│   ├── site_test.rs
│   ├── device_test.rs
│   └── ...
└── integration/
    └── client_test.rs (optional)
```

## Test Cases

### Positive Test Cases
- Successful GET requests
- Successful POST requests (create)
- Successful PATCH requests (update)
- Successful DELETE requests
- Successful query with filters
- Successful pagination
- Successful custom actions (available-ips, etc.)

### Negative Test Cases
- 404 Not Found errors
- 400 Bad Request errors
- 401 Unauthorized errors
- 403 Forbidden errors
- 500 Internal Server Error
- Network timeouts
- Invalid JSON responses
- Missing required fields
- Invalid field values
- Duplicate resource creation
- Resource dependencies (missing parent)
- Pagination edge cases (empty results, single page)

## Progress Tracking

- [x] Common module created
- [ ] IPAM module (0/10 files)
- [ ] DCIM module (0/11 files)
- [ ] Tenancy module (0/2 files)
- [ ] Extras module (0/2 files)
- [ ] Trait completion (24/150+ methods)
- [ ] Tests (0% coverage)

## Next Steps

1. Fix trait module naming
2. Extract IPAM prefix operations as proof of concept
3. Create IPAM module structure
4. Extract all IPAM operations
5. Create comprehensive tests for IPAM
6. Repeat for DCIM, Tenancy, Extras
7. Complete trait with all methods
8. Add remaining modules (virtualization, circuits, etc.)

