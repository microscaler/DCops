//! IPAM (IP Address Management) reconcilers
//! 
//! Handles: IPClaim, IPPool, NetBoxPrefix, NetBoxAggregate, NetBoxRIR, NetBoxIPAddress

pub mod ip_claim;
#[cfg(test)]
pub mod ip_claim_test;
pub mod ip_pool;
#[cfg(test)]
pub mod ip_pool_test;
pub mod prefix;
#[cfg(test)]
pub mod prefix_test;
pub mod aggregate;
#[cfg(test)]
pub mod aggregate_test;
pub mod rir;
#[cfg(test)]
pub mod rir_test;
pub mod ip_address;
#[cfg(test)]
pub mod ip_address_test;
pub mod ip_range;
#[cfg(test)]
pub mod ip_range_test;
