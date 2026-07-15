//! IPAM (IP Address Management) reconcilers
//! 
//! Handles: NetBoxPrefix, NetBoxAggregate, NetBoxRIR, NetBoxIPAddress,
//! NetBoxIPRange, NetBoxRouteTarget, NetBoxVRF, IPPool, IPClaim
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
pub mod route_target;
// #[cfg(test)]
// pub mod route_target_test;
pub mod vrf;
// #[cfg(test)]
// pub mod vrf_test;
pub mod ip_pool;
pub mod ip_claim;
