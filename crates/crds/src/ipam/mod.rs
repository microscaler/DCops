//! IPAM (IP Address Management) CRDs
//!
//! Resources for managing IP addresses, prefixes, and VLANs:
//! - IP Addresses
//! - IP Ranges
//! - Prefixes
//! - Aggregates
//! - Roles (IPAM roles)
//! - VLANs
//! - RIRs (Regional Internet Registries)

pub mod netbox_ip_address;
pub mod netbox_ip_range;
pub mod netbox_prefix;
pub mod netbox_aggregate;
pub mod netbox_role;
pub mod netbox_vlan;
pub mod netbox_rir;

pub use netbox_ip_address::*;
pub use netbox_ip_range::*;
pub use netbox_prefix::*;
pub use netbox_aggregate::*;
pub use netbox_role::*;
pub use netbox_vlan::*;
pub use netbox_rir::*;

