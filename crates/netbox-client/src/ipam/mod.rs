//! IPAM (IP Address Management) module
//!
//! This module provides methods for managing NetBox IPAM resources:
//! - Prefixes
//! - IP Addresses
//! - Aggregates
//! - RIRs (Regional Internet Registries)
//! - VLANs

pub mod prefix;
pub mod ip_address;
pub mod aggregate;
pub mod rir;
pub mod vlan;

// Re-export all IPAM functions for convenience
pub use prefix::*;
pub use ip_address::*;
pub use aggregate::*;
pub use rir::*;
pub use vlan::*;

