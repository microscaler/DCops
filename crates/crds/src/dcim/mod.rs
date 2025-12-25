//! DCIM (Data Center Infrastructure Management) CRDs
//!
//! Resources for managing physical infrastructure:
//! - Regions (hierarchical site organization)
//! - Site Groups (alternative to regions)
//! - Locations (nested locations within sites)
//! - Sites
//! - Device Roles
//! - Manufacturers
//! - Platforms
//! - Device Types
//! - Devices
//! - Interfaces
//! - MAC Addresses

pub mod netbox_region;
pub mod netbox_site_group;
pub mod netbox_location;
pub mod netbox_site;
pub mod netbox_device_role;
pub mod netbox_manufacturer;
pub mod netbox_platform;
pub mod netbox_device_type;
pub mod netbox_device;
pub mod netbox_interface;
pub mod netbox_mac_address;

pub use netbox_region::*;
pub use netbox_site_group::*;
pub use netbox_location::*;
pub use netbox_site::*;
pub use netbox_device_role::*;
pub use netbox_manufacturer::*;
pub use netbox_platform::*;
pub use netbox_device_type::*;
pub use netbox_device::*;
pub use netbox_interface::*;
pub use netbox_mac_address::*;

