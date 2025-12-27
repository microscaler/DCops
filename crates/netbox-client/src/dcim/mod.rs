//! DCIM module for NetBox API client
//!
//! This module re-exports client methods for DCIM-related resources.

pub mod device;
pub mod device_role;
pub mod device_type;
pub mod interface;
pub mod location;
pub mod mac_address;
pub mod manufacturer;
pub mod platform;
pub mod region;
pub mod site;
pub mod site_group;

// Re-export functions
pub use device::{query_devices, get_device, get_device_by_mac, create_device, update_device};
pub use device_role::{query_device_roles, get_device_role_by_name, create_device_role};
pub use device_type::{query_device_types, get_device_type_by_model, create_device_type};
pub use interface::{query_interfaces, get_interface, create_interface, update_interface};
pub use location::{query_locations, get_location, get_location_by_name, create_location};
pub use mac_address::{query_mac_addresses, get_mac_address_by_address, create_mac_address};
pub use manufacturer::{query_manufacturers, get_manufacturer_by_name, create_manufacturer};
pub use platform::{query_platforms, get_platform_by_name, create_platform};
pub use region::{query_regions, get_region, get_region_by_name, create_region};
pub use site::{query_sites, get_site, create_site, update_site};
pub use site_group::{query_site_groups, get_site_group, get_site_group_by_name, create_site_group};

