//! Tenancy CRDs
//!
//! Resources for managing tenants and organizational structure:
//! - Tenants
//! - Tenant Groups

pub mod netbox_tenant;
pub mod netbox_tenant_group;

pub use netbox_tenant::*;
pub use netbox_tenant_group::*;

