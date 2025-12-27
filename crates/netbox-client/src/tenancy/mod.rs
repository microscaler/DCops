//! Tenancy module for NetBox API client
//!
//! This module re-exports client methods for Tenancy-related resources.

pub mod tenant;
pub mod tenant_group;

// Re-export functions
pub use tenant::{query_tenants, get_tenant, create_tenant};
pub use tenant_group::{query_tenant_groups, get_tenant_group_by_name, create_tenant_group};

