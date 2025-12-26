//! IPAM (IP Address Management) reconcilers
//! 
//! Handles: IPClaim, IPPool, NetBoxPrefix, NetBoxAggregate

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
