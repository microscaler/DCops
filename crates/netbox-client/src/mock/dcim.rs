//! DCIM operations for MockNetBoxClient
//!
//! Handles sites, regions, site groups, locations, devices, interfaces, MAC addresses,
//! device roles, manufacturers, platforms, and device types

use super::MockNetBoxClient;
use crate::error::NetBoxError;
use crate::models::*;

pub async fn query_devices(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Device>, NetBoxError> {
        let devices = client.devices.lock().unwrap();
        Ok(devices.values().cloned().collect())
}

pub async fn get_device(client: &MockNetBoxClient, id: u64) -> Result<Device, NetBoxError> {
        client.devices
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Device {} not found", id)))
}

pub async fn get_device_by_mac(client: &MockNetBoxClient, _mac: &str) -> Result<Option<Device>, NetBoxError> {
        Ok(None)
}

pub async fn create_device(client: &MockNetBoxClient, _name: &str, _device_type_id: u64, _device_role_id: u64, _site_id: u64, _location_id: Option<u64>, _tenant_id: Option<u64>, _platform_id: Option<u64>, _serial: Option<&str>, _asset_tag: Option<&str>, _status: &str, _primary_ip4_id: Option<u64>, _primary_ip6_id: Option<u64>, _description: Option<&str>, _comments: Option<&str>) -> Result<Device, NetBoxError> {
        Err(NetBoxError::Api("Not implemented in mock".to_string()))
}

pub async fn update_device(client: &MockNetBoxClient, _id: u64, _name: Option<&str>, _device_type_id: Option<u64>, _device_role_id: Option<u64>, _site_id: Option<u64>, _location_id: Option<u64>, _tenant_id: Option<u64>, _platform_id: Option<u64>, _serial: Option<&str>, _asset_tag: Option<&str>, _status: Option<&str>, _primary_ip4_id: Option<u64>, _primary_ip6_id: Option<u64>, _description: Option<&str>, _comments: Option<&str>) -> Result<Device, NetBoxError> {
        Err(NetBoxError::Api("Not implemented in mock".to_string()))
}

pub async fn query_interfaces(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Interface>, NetBoxError> {
        Ok(vec![])
}

pub async fn get_interface(client: &MockNetBoxClient, id: u64) -> Result<Interface, NetBoxError> {
        client.interfaces
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Interface {} not found", id)))
}

pub async fn create_interface(client: &MockNetBoxClient, _device_id: u64, _name: &str, _interface_type: &str, _enabled: bool, _description: Option<&str>) -> Result<Interface, NetBoxError> {
        Err(NetBoxError::Api("Not implemented in mock".to_string()))
}

pub async fn update_interface(client: &MockNetBoxClient, _id: u64, _name: Option<&str>, _interface_type: Option<&str>, _enabled: Option<bool>, _mac_address: Option<&str>, _description: Option<&str>) -> Result<Interface, NetBoxError> {
        Err(NetBoxError::Api("Not implemented in mock".to_string()))
}

pub async fn query_mac_addresses(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<MACAddress>, NetBoxError> {
        Ok(vec![])
}

pub async fn get_mac_address_by_address(client: &MockNetBoxClient, mac: &str) -> Result<Option<MACAddress>, NetBoxError> {
        Ok(client.mac_addresses.lock().unwrap().get(mac).cloned())
}

pub async fn create_mac_address(client: &MockNetBoxClient, _interface_id: u64, _address: &str, _description: Option<&str>) -> Result<MACAddress, NetBoxError> {
        Err(NetBoxError::Api("Not implemented in mock".to_string()))
}

pub async fn query_sites(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Site>, NetBoxError> {
        let sites = client.sites.lock().unwrap();
        Ok(sites.values().cloned().collect())
}

pub async fn get_site(client: &MockNetBoxClient, id: u64) -> Result<Site, NetBoxError> {
        client.sites
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Site {} not found", id)))
}

pub async fn create_site(client: &MockNetBoxClient, name: &str, slug: Option<&str>, status: &str, region_id: Option<u64>, site_group_id: Option<u64>, tenant_id: Option<u64>, facility: Option<&str>, time_zone: Option<&str>, description: Option<&str>, comments: Option<&str>) -> Result<Site, NetBoxError> {
        let id = client.next_id();
        let slug_value = slug.map(|s| s.to_string()).unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));
        let status_enum = match status {
            "active" => SiteStatus::Active,
            "planned" => SiteStatus::Planned,
            "retired" => SiteStatus::Retired,
            "staging" => SiteStatus::Staging,
            _ => SiteStatus::Active,
        };
        
        let site = Site {
            id,
            url: format!("{}/api/dcim/sites/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug_value,
            status: status_enum,
            region: region_id.map(|id| client.helpers().create_nested_region(id, None)),
            site_group: site_group_id.map(|id| client.helpers().create_nested_site_group(id, None)),
            tenant: tenant_id.map(|id| client.helpers().create_nested_tenant(id, None)),
            facility: facility.map(|s| s.to_string()),
            physical_address: None,
            shipping_address: None,
            latitude: None,
            longitude: None,
            time_zone: time_zone.map(|s| s.to_string()),
            description: description.map(|s| s.to_string()),
            comments: comments.map(|s| s.to_string()),
            tags: vec![],
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.sites.lock().unwrap().insert(id, site.clone());
        Ok(site)
}

pub async fn update_site(client: &MockNetBoxClient, id: u64, name: Option<&str>, slug: Option<&str>, status: Option<&str>, region_id: Option<u64>, site_group_id: Option<u64>, tenant_id: Option<u64>, facility: Option<&str>, time_zone: Option<&str>, description: Option<&str>, comments: Option<&str>) -> Result<Site, NetBoxError> {
        let mut sites = client.sites.lock().unwrap();
        let site = sites
            .get_mut(&id)
            .ok_or_else(|| NetBoxError::NotFound(format!("Site {} not found", id)))?;

        if let Some(name_str) = name {
            site.name = name_str.to_string();
        }
        if let Some(slug_str) = slug {
            site.slug = slug_str.to_string();
        }
        if let Some(status_str) = status {
            site.status = match status_str {
                "active" => SiteStatus::Active,
                "planned" => SiteStatus::Planned,
                "retired" => SiteStatus::Retired,
                "staging" => SiteStatus::Staging,
                _ => SiteStatus::Active,
            };
        }
        if let Some(region) = region_id {
            site.region = Some(client.helpers().create_nested_region(region, None));
        }
        if let Some(site_group) = site_group_id {
            site.site_group = Some(client.helpers().create_nested_site_group(site_group, None));
        }
        if let Some(tenant) = tenant_id {
            site.tenant = Some(client.helpers().create_nested_tenant(tenant, None));
        }
        if let Some(fac) = facility {
            site.facility = Some(fac.to_string());
        }
        if let Some(tz) = time_zone {
            site.time_zone = Some(tz.to_string());
        }
        if let Some(desc) = description {
            site.description = Some(desc.to_string());
        }
        if let Some(comm) = comments {
            site.comments = Some(comm.to_string());
        }

        Ok(site.clone())
}

pub async fn query_regions(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Region>, NetBoxError> {
        let regions = client.regions.lock().unwrap();
        Ok(regions.values().cloned().collect())
}

pub async fn get_region(client: &MockNetBoxClient, id: u64) -> Result<Region, NetBoxError> {
        client.regions
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Region {} not found", id)))
}

pub async fn get_region_by_name(client: &MockNetBoxClient, name: &str) -> Result<Option<Region>, NetBoxError> {
        let regions = client.regions.lock().unwrap();
        Ok(regions.values().find(|r| r.name == name).cloned())
}

pub async fn create_region(client: &MockNetBoxClient, name: &str, slug: &str, description: Option<&str>) -> Result<Region, NetBoxError> {
        let id = client.next_id();
        let region = Region {
            id,
            url: format!("{}/api/dcim/regions/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug.to_string(),
            parent: None,
            description: description.map(|s| s.to_string()),
            comments: None,
            site_count: 0,
            prefix_count: 0,
            _depth: None,
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.regions.lock().unwrap().insert(id, region.clone());
        Ok(region)
}

pub async fn query_site_groups(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<SiteGroup>, NetBoxError> {
        let site_groups = client.site_groups.lock().unwrap();
        Ok(site_groups.values().cloned().collect())
}

pub async fn get_site_group(client: &MockNetBoxClient, id: u64) -> Result<SiteGroup, NetBoxError> {
        client.site_groups
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Site group {} not found", id)))
}

pub async fn get_site_group_by_name(client: &MockNetBoxClient, name: &str) -> Result<Option<SiteGroup>, NetBoxError> {
        let site_groups = client.site_groups.lock().unwrap();
        Ok(site_groups.values().find(|sg| sg.name == name).cloned())
}

pub async fn create_site_group(client: &MockNetBoxClient, name: &str, slug: &str, description: Option<&str>) -> Result<SiteGroup, NetBoxError> {
        let id = client.next_id();
        let site_group = SiteGroup {
            id,
            url: format!("{}/api/dcim/site-groups/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug.to_string(),
            parent: None,
            description: description.map(|s| s.to_string()),
            comments: None,
            site_count: 0,
            prefix_count: 0,
            _depth: None,
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.site_groups.lock().unwrap().insert(id, site_group.clone());
        Ok(site_group)
}

pub async fn query_locations(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Location>, NetBoxError> {
        let locations = client.locations.lock().unwrap();
        Ok(locations.values().cloned().collect())
}

pub async fn get_location(client: &MockNetBoxClient, id: u64) -> Result<Location, NetBoxError> {
        client.locations
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Location {} not found", id)))
}

pub async fn get_location_by_name(client: &MockNetBoxClient, site_id: u64, name: &str) -> Result<Option<Location>, NetBoxError> {
        let locations = client.locations.lock().unwrap();
        Ok(locations.values().find(|l| l.site.id == site_id && l.name == name).cloned())
}

pub async fn create_location(client: &MockNetBoxClient, site_id: u64, name: &str, slug: Option<&str>, parent_id: Option<u64>, description: Option<String>, comments: Option<String>) -> Result<Location, NetBoxError> {
        let id = client.next_id();
        let slug_value = slug.map(|s| s.to_string()).unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));
        let location = Location {
            id,
            url: format!("{}/api/dcim/locations/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug_value,
            site: client.helpers().create_nested_site(site_id, None),
            parent: parent_id.map(|id| NestedLocation {
                id,
                url: format!("{}/api/dcim/locations/{}/", client.base_url, id),
                display: format!("Location {}", id),
                name: format!("Location {}", id),
                slug: format!("location-{}", id),
            }),
            description: description,
            comments: comments,
            device_count: 0,
            rack_count: 0,
            _depth: None,
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.locations.lock().unwrap().insert(id, location.clone());
        Ok(location)
}

pub async fn query_device_roles(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<DeviceRole>, NetBoxError> {
        let device_roles = client.device_roles.lock().unwrap();
        Ok(device_roles.values().cloned().collect())
}

pub async fn get_device_role_by_name(client: &MockNetBoxClient, name: &str) -> Result<Option<DeviceRole>, NetBoxError> {
        Ok(client.device_roles.lock().unwrap().get(name).cloned())
}

pub async fn create_device_role(client: &MockNetBoxClient, name: &str, slug: &str, description: Option<&str>) -> Result<DeviceRole, NetBoxError> {
        let id = client.next_id();
        let device_role = DeviceRole {
            id,
            url: format!("{}/api/dcim/device-roles/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug.to_string(),
            color: None, // DeviceRole.color is Option<String>
            vm_role: false,
            description: description.map(|s| s.to_string()),
            comments: None,
            device_count: 0,
            virtualmachine_count: 0,
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.device_roles.lock().unwrap().insert(name.to_string(), device_role.clone());
        Ok(device_role)
}

pub async fn query_manufacturers(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Manufacturer>, NetBoxError> {
        let manufacturers = client.manufacturers.lock().unwrap();
        Ok(manufacturers.values().cloned().collect())
}

pub async fn get_manufacturer_by_name(client: &MockNetBoxClient, name: &str) -> Result<Option<Manufacturer>, NetBoxError> {
        Ok(client.manufacturers.lock().unwrap().get(name).cloned())
}

pub async fn create_manufacturer(client: &MockNetBoxClient, name: &str, slug: &str, description: Option<&str>) -> Result<Manufacturer, NetBoxError> {
        let id = client.next_id();
        let manufacturer = Manufacturer {
            id,
            url: format!("{}/api/dcim/manufacturers/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug.to_string(),
            description: description.map(|s| s.to_string()),
            devicetype_count: 0,
            inventoryitem_count: 0,
            platform_count: 0,
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.manufacturers.lock().unwrap().insert(name.to_string(), manufacturer.clone());
        Ok(manufacturer)
}

pub async fn query_platforms(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Platform>, NetBoxError> {
        let platforms = client.platforms.lock().unwrap();
        Ok(platforms.values().cloned().collect())
}

pub async fn get_platform_by_name(client: &MockNetBoxClient, name: &str) -> Result<Option<Platform>, NetBoxError> {
        Ok(client.platforms.lock().unwrap().get(name).cloned())
}

pub async fn create_platform(client: &MockNetBoxClient, name: &str, slug: &str, description: Option<&str>) -> Result<Platform, NetBoxError> {
        let id = client.next_id();
        let platform = Platform {
            id,
            url: format!("{}/api/dcim/platforms/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug.to_string(),
            manufacturer: None,
            napalm_driver: None,
            napalm_args: None,
            description: description.map(|s| s.to_string()),
            comments: None,
            device_count: 0,
            virtualmachine_count: 0,
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.platforms.lock().unwrap().insert(name.to_string(), platform.clone());
        Ok(platform)
}

pub async fn query_device_types(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<DeviceType>, NetBoxError> {
        let device_types = client.device_types.lock().unwrap();
        Ok(device_types.values().cloned().collect())
}

pub async fn get_device_type_by_model(client: &MockNetBoxClient, manufacturer_id: u64, model: &str) -> Result<Option<DeviceType>, NetBoxError> {
        Ok(client.device_types.lock().unwrap().get(&(manufacturer_id, model.to_string())).cloned())
}

pub async fn create_device_type(client: &MockNetBoxClient, manufacturer_id: u64, model: &str, slug: Option<&str>, description: Option<&str>) -> Result<DeviceType, NetBoxError> {
        let id = client.next_id();
        let slug_value = slug.map(|s| s.to_string()).unwrap_or_else(|| model.to_lowercase().replace(' ', "-"));
        let device_type = DeviceType {
            id,
            url: format!("{}/api/dcim/device-types/{}/", client.base_url, id),
            display: model.to_string(),
            manufacturer: NestedManufacturer {
                id: manufacturer_id,
                url: format!("{}/api/dcim/manufacturers/{}/", client.base_url, manufacturer_id),
                display: format!("Manufacturer {}", manufacturer_id),
                name: format!("Manufacturer {}", manufacturer_id),
                slug: format!("manufacturer-{}", manufacturer_id),
            },
            model: model.to_string(),
            slug: slug_value,
            part_number: None,
            u_height: 0.0,
            is_full_depth: false,
            description: description.map(|s| s.to_string()),
            comments: None,
            device_count: 0,
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.device_types.lock().unwrap().insert((manufacturer_id, model.to_string()), device_type.clone());
        Ok(device_type)
}

    // Tenancy Operations
