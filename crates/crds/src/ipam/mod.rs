//! IPAM (IP Address Management) CRDs
//!
//! Resources for managing IP addresses, prefixes, and VLANs:
//! - Prefixes
//! - Aggregates
//! - Roles (IPAM roles)
//! - VLANs

pub mod netbox_prefix;
pub mod netbox_aggregate;
pub mod netbox_role;
pub mod netbox_vlan;

pub use netbox_prefix::*;
pub use netbox_aggregate::*;
pub use netbox_role::*;
pub use netbox_vlan::*;

