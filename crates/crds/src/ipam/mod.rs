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
//! - VRFs (Virtual Routing and Forwarding)
//! - Route Targets
//! - IPPool (Kubernetes-native IP pool backed by NetBox prefixes)
//! - IPClaim (Kubernetes-native IP claim from an IPPool)

pub mod netbox_ip_address;
pub mod netbox_ip_range;
pub mod netbox_prefix;
pub mod netbox_aggregate;
pub mod netbox_role;
pub mod netbox_vlan;
pub mod netbox_rir;
pub mod netbox_vrf;
pub mod netbox_route_target;
pub mod ip_pool;
pub mod ip_claim;

pub use netbox_ip_address::*;
pub use netbox_ip_range::*;
pub use netbox_prefix::*;
pub use netbox_aggregate::*;
pub use netbox_role::*;
pub use netbox_vlan::*;
pub use netbox_rir::*;
pub use netbox_vrf::*;
pub use netbox_route_target::*;
pub use ip_pool::*;
pub use ip_claim::*;

