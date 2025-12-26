//! IPAM (IP Address Management) reconcilers
//! 
//! Handles: IPClaim, IPPool, NetBoxPrefix, NetBoxAggregate

pub mod ip_claim;
pub mod ip_pool;
#[cfg(test)]
mod ip_pool_test;
pub mod prefix;
pub mod aggregate;
