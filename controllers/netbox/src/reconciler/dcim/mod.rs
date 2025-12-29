//! DCIM (Data Center Infrastructure Management) reconcilers
//! 
//! Handles: Site, Region, SiteGroup, Location, Device*, Interface, MAC, VLAN

pub mod site;
#[cfg(test)]
pub mod site_test;
pub mod region;
pub mod site_group;
pub mod location;
pub mod device_role;
#[cfg(test)]
pub mod device_role_test;
pub mod manufacturer;
#[cfg(test)]
pub mod manufacturer_test;
pub mod platform;
#[cfg(test)]
pub mod platform_test;
pub mod device_type;
#[cfg(test)]
pub mod device_type_test;
pub mod device;
#[cfg(test)]
pub mod device_test;
pub mod interface;
pub mod mac_address;
pub mod vlan;
#[cfg(test)]
pub mod vlan_test;
