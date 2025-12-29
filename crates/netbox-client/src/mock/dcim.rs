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

pub async fn get_device_by_mac(_client: &MockNetBoxClient, _mac: &str) -> Result<Option<Device>, NetBoxError> {
        Ok(None)
}

pub async fn create_device(client: &MockNetBoxClient, device_type_id: u64, device_role_id: u64, site_id: u64, name: Option<&str>, tenant_id: Option<u64>, platform_id: Option<u64>, location_id: Option<u64>, serial: Option<&str>, asset_tag: Option<&str>, status: Option<&str>, primary_ip4_id: Option<u64>, primary_ip6_id: Option<u64>, description: Option<String>, comments: Option<String>) -> Result<Device, NetBoxError> {
        let id = client.next_id();
        let status_enum = match status {
            Some("active") => DeviceStatus::Active,
            Some("offline") => DeviceStatus::Offline,
            Some("planned") => DeviceStatus::Planned,
            Some("staged") => DeviceStatus::Staged,
            Some("failed") => DeviceStatus::Failed,
            Some("inventory") => DeviceStatus::Inventory,
            Some("decommissioning") => DeviceStatus::Decommissioning,
            _ => DeviceStatus::Active,
        };
        
        // Get device type to extract manufacturer_id and model
        let device_type = client.device_types.lock().unwrap()
            .values()
            .find(|dt| dt.id == device_type_id)
            .cloned();
        
        let (manufacturer_id, model) = if let Some(dt) = device_type {
            (dt.manufacturer.id, dt.model)
        } else {
            (1, "Unknown Model".to_string())
        };
        
        let display = name.map(|n| n.to_string()).unwrap_or_else(|| format!("Device {}", id));
        
        let device = Device {
            id,
            url: format!("{}/api/dcim/devices/{}/", client.base_url, id),
            display: display.clone(),
            name: name.map(|s| s.to_string()),
            device_type: client.helpers().create_nested_device_type(device_type_id, Some(model), Some(manufacturer_id)),
            device_role: Some(client.helpers().create_nested_device_role(device_role_id, None)),
            tenant: tenant_id.map(|tid| client.helpers().create_nested_tenant(tid, None)),
            platform: platform_id.map(|pid| client.helpers().create_nested_platform(pid, None)),
            site: Some(client.helpers().create_nested_site(site_id, None)),
            location: location_id.map(|lid| client.helpers().create_nested_location(lid, None)),
            status: status_enum,
            serial: serial.map(|s| s.to_string()),
            asset_tag: asset_tag.map(|s| s.to_string()),
            primary_ip4: primary_ip4_id.and_then(|ip_id| {
                client.ip_addresses.lock().unwrap()
                    .get(&ip_id)
                    .map(|ip| client.helpers().create_nested_ip_address(ip_id, ip.address))
            }),
            primary_ip6: primary_ip6_id.and_then(|ip_id| {
                client.ip_addresses.lock().unwrap()
                    .get(&ip_id)
                    .map(|ip| client.helpers().create_nested_ip_address(ip_id, ip.address))
            }),
            description,
            comments,
            tags: vec![],
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };
        
        client.devices.lock().unwrap().insert(id, device.clone());
        Ok(device)
    }

pub async fn update_device(client: &MockNetBoxClient, id: u64, name: Option<&str>, tenant_id: Option<u64>, platform_id: Option<u64>, location_id: Option<u64>, serial: Option<&str>, asset_tag: Option<&str>, status: Option<&str>, primary_ip4_id: Option<u64>, primary_ip6_id: Option<u64>, description: Option<String>, comments: Option<String>) -> Result<Device, NetBoxError> {
        let mut devices = client.devices.lock().unwrap();
        let device = devices
            .get_mut(&id)
            .ok_or_else(|| NetBoxError::NotFound(format!("Device {} not found", id)))?;
        
        if let Some(name_str) = name {
            device.name = Some(name_str.to_string());
            device.display = name_str.to_string();
        }
        if let Some(tenant) = tenant_id {
            device.tenant = Some(client.helpers().create_nested_tenant(tenant, None));
        }
        if let Some(platform) = platform_id {
            device.platform = Some(client.helpers().create_nested_platform(platform, None));
        }
        if let Some(location) = location_id {
            device.location = Some(client.helpers().create_nested_location(location, None));
        }
        if let Some(serial_str) = serial {
            device.serial = Some(serial_str.to_string());
        }
        if let Some(asset_tag_str) = asset_tag {
            device.asset_tag = Some(asset_tag_str.to_string());
        }
        if let Some(status_str) = status {
            device.status = match status_str {
                "active" => DeviceStatus::Active,
                "offline" => DeviceStatus::Offline,
                "planned" => DeviceStatus::Planned,
                "staged" => DeviceStatus::Staged,
                "failed" => DeviceStatus::Failed,
                "inventory" => DeviceStatus::Inventory,
                "decommissioning" => DeviceStatus::Decommissioning,
                _ => DeviceStatus::Active,
            };
        }
        if let Some(ip_id) = primary_ip4_id {
            device.primary_ip4 = client.ip_addresses.lock().unwrap()
                .get(&ip_id)
                .map(|ip| client.helpers().create_nested_ip_address(ip_id, ip.address));
        }
        if let Some(ip_id) = primary_ip6_id {
            device.primary_ip6 = client.ip_addresses.lock().unwrap()
                .get(&ip_id)
                .map(|ip| client.helpers().create_nested_ip_address(ip_id, ip.address));
        }
        if let Some(desc) = description {
            device.description = Some(desc);
        }
        if let Some(comm) = comments {
            device.comments = Some(comm);
        }
        
        device.last_updated = chrono::Utc::now().to_rfc3339();
        Ok(device.clone())
    }

pub async fn query_interfaces(client: &MockNetBoxClient, filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Interface>, NetBoxError> {
        let interfaces = client.interfaces.lock().unwrap();
        let mut results: Vec<Interface> = interfaces.values().cloned().collect();
        
        // Apply filters
        for (key, value) in filters {
            match *key {
                "device_id" => {
                    let device_id: u64 = value.parse().unwrap_or(0);
                    results.retain(|i| i.device.id == device_id);
                }
                "name" => {
                    results.retain(|i| i.name == *value);
                }
                _ => {}
            }
        }
        
        Ok(results)
    }

pub async fn get_interface(client: &MockNetBoxClient, id: u64) -> Result<Interface, NetBoxError> {
        client.interfaces
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Interface {} not found", id)))
}

pub async fn create_interface(client: &MockNetBoxClient, device_id: u64, name: &str, interface_type: &str, enabled: Option<bool>, mac_address: Option<&str>, mtu: Option<u16>, description: Option<String>) -> Result<Interface, NetBoxError> {
        use chrono::Utc;
        
        // Verify device exists
        let device = client.devices
            .lock()
            .unwrap()
            .get(&device_id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Device {} not found", device_id)))?;
        
        let id = client.next_id();
        let interface = Interface {
            id,
            url: format!("{}/api/dcim/interfaces/{}/", client.base_url, id),
            display: name.to_string(),
            device: crate::models::NestedDevice {
                id: device_id,
                url: format!("{}/api/dcim/devices/{}/", client.base_url, device_id),
                display: device.display.clone(),
                name: device.name.clone().unwrap_or_default(),
            },
            vdcs: vec![],
            module: None,
            name: name.to_string(),
            label: None,
            r#type: interface_type.to_string(),
            enabled: enabled.unwrap_or(true),
            parent: None,
            bridge: None,
            lag: None,
            mac_address: mac_address.map(|s| s.to_string()),
            mtu,
            description,
            ip_addresses: vec![],
            tags: vec![],
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        };
        
        client.interfaces.lock().unwrap().insert(id, interface.clone());
        Ok(interface)
    }

pub async fn update_interface(_client: &MockNetBoxClient, _id: u64, _name: Option<&str>, _interface_type: Option<&str>, _enabled: Option<bool>, _mac_address: Option<&str>, _mtu: Option<u16>, _description: Option<String>) -> Result<Interface, NetBoxError> {
        Err(NetBoxError::Api("Not implemented in mock".to_string()))
    }

pub async fn query_mac_addresses(_client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<MACAddress>, NetBoxError> {
        Ok(vec![])
}

pub async fn get_mac_address_by_address(client: &MockNetBoxClient, mac: &str) -> Result<Option<MACAddress>, NetBoxError> {
        Ok(client.mac_addresses.lock().unwrap().get(mac).cloned())
}

pub async fn create_mac_address(client: &MockNetBoxClient, mac_address: &str, _assigned_object_type: &str, assigned_object_id: u64, description: Option<String>, comments: Option<String>) -> Result<MACAddress, NetBoxError> {
        use chrono::Utc;
        
        // Verify interface exists (for assigned_object_id)
        let _interface = client.interfaces
            .lock()
            .unwrap()
            .get(&assigned_object_id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Interface {} not found", assigned_object_id)))?;
        
        let id = client.next_id();
        let mac = MACAddress {
            id,
            url: format!("{}/api/dcim/mac-addresses/{}/", client.base_url, id),
            display: mac_address.to_string(),
            mac_address: mac_address.to_string(),
            assigned_object_type: Some(_assigned_object_type.to_string()),
            assigned_object_id: Some(assigned_object_id),
            assigned_object: None,
            description,
            comments,
            tags: vec![],
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        };
        
        client.mac_addresses.lock().unwrap().insert(mac_address.to_string(), mac.clone());
        Ok(mac)
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

pub async fn create_site(client: &MockNetBoxClient, name: &str, slug: Option<&str>, description: Option<String>, physical_address: Option<String>, shipping_address: Option<String>, latitude: Option<f64>, longitude: Option<f64>, tenant_id: Option<u64>, region_id: Option<u64>, site_group_id: Option<u64>, status: Option<&str>, facility: Option<String>, time_zone: Option<String>, comments: Option<String>) -> Result<Site, NetBoxError> {
        let id = client.next_id();
        let slug_value = slug.map(|s| s.to_string()).unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));
        let status_enum = match status {
            Some("active") | Some("planned") | Some("retired") | Some("staging") => {
                match status.unwrap() {
                    "active" => SiteStatus::Active,
                    "planned" => SiteStatus::Planned,
                    "retired" => SiteStatus::Retired,
                    "staging" => SiteStatus::Staging,
                    _ => SiteStatus::Active,
                }
            }
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
            facility: facility.clone(),
            physical_address: physical_address.clone(),
            shipping_address: shipping_address.clone(),
            latitude,
            longitude,
            time_zone: time_zone.clone(),
            description: description.clone(),
            comments: comments.clone(),
            tags: vec![],
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.sites.lock().unwrap().insert(id, site.clone());
        Ok(site)
}

pub async fn update_site(client: &MockNetBoxClient, id: u64, name: Option<&str>, slug: Option<&str>, description: Option<String>, physical_address: Option<String>, shipping_address: Option<String>, latitude: Option<f64>, longitude: Option<f64>, tenant_id: Option<u64>, region_id: Option<u64>, site_group_id: Option<u64>, status: Option<&str>, facility: Option<String>, time_zone: Option<String>, comments: Option<String>) -> Result<Site, NetBoxError> {
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
        if let Some(desc) = description {
            site.description = Some(desc);
        }
        if let Some(addr) = physical_address {
            site.physical_address = Some(addr);
        }
        if let Some(addr) = shipping_address {
            site.shipping_address = Some(addr);
        }
        if let Some(lat) = latitude {
            site.latitude = Some(lat);
        }
        if let Some(lon) = longitude {
            site.longitude = Some(lon);
        }
        if let Some(tenant) = tenant_id {
            site.tenant = Some(client.helpers().create_nested_tenant(tenant, None));
        }
        if let Some(region) = region_id {
            site.region = Some(client.helpers().create_nested_region(region, None));
        }
        if let Some(site_group) = site_group_id {
            site.site_group = Some(client.helpers().create_nested_site_group(site_group, None));
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
        if let Some(fac) = facility {
            site.facility = Some(fac);
        }
        if let Some(tz) = time_zone {
            site.time_zone = Some(tz);
        }
        if let Some(comm) = comments {
            site.comments = Some(comm);
        }

        Ok(site.clone())
}

pub async fn query_regions(client: &MockNetBoxClient, filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Region>, NetBoxError> {
        let regions = client.regions.lock().unwrap();
        let mut results: Vec<Region> = regions.values().cloned().collect();
        
        // Apply filters
        for (key, value) in filters {
            match *key {
                "slug" => {
                    results.retain(|r| r.slug == *value);
                }
                "name" => {
                    results.retain(|r| r.name == *value);
                }
                _ => {}
            }
        }
        
        Ok(results)
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

pub async fn create_region(client: &MockNetBoxClient, name: &str, slug: Option<&str>, parent_id: Option<u64>, description: Option<String>, comments: Option<String>) -> Result<Region, NetBoxError> {
        let id = client.next_id();
        let slug_value = slug.map(|s| s.to_string()).unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));
        let region = Region {
            id,
            url: format!("{}/api/dcim/regions/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug_value,
            parent: parent_id.map(|id| client.helpers().create_nested_region(id, None)),
            description,
            comments,
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

pub async fn create_site_group(client: &MockNetBoxClient, name: &str, slug: Option<&str>, parent_id: Option<u64>, description: Option<String>, comments: Option<String>) -> Result<SiteGroup, NetBoxError> {
        let id = client.next_id();
        let slug_value = slug.map(|s| s.to_string()).unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));
        let site_group = SiteGroup {
            id,
            url: format!("{}/api/dcim/site-groups/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug_value,
            parent: parent_id.map(|id| client.helpers().create_nested_site_group(id, None)),
            description,
            comments,
            site_count: 0,
            prefix_count: 0,
            _depth: None,
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.site_groups.lock().unwrap().insert(id, site_group.clone());
        Ok(site_group)
    }

pub async fn query_locations(client: &MockNetBoxClient, filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Location>, NetBoxError> {
        let locations = client.locations.lock().unwrap();
        let mut results: Vec<Location> = locations.values().cloned().collect();
        
        // Apply filters
        for (key, value) in filters {
            match *key {
                "site_id" => {
                    let site_id: u64 = value.parse().unwrap_or(0);
                    results.retain(|l| l.site.id == site_id);
                }
                "name" => {
                    results.retain(|l| l.name == *value);
                }
                "slug" => {
                    results.retain(|l| l.slug == *value);
                }
                _ => {}
            }
        }
        
        Ok(results)
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

pub async fn create_location(client: &MockNetBoxClient, site_id: u64, name: &str, slug: Option<&str>, parent_id: Option<u64>, _tenant_id: Option<u64>, _facility: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Location, NetBoxError> {
        let id = client.next_id();
        let slug_value = slug.map(|s| s.to_string()).unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));
        let location = Location {
            id,
            url: format!("{}/api/dcim/locations/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug_value,
            site: client.helpers().create_nested_site(site_id, None),
            parent: parent_id.map(|id| client.helpers().create_nested_location(id, None)),
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

pub async fn create_device_role(client: &MockNetBoxClient, name: &str, slug: Option<&str>, color: Option<&str>, vm_role: Option<bool>, description: Option<String>, comments: Option<String>) -> Result<DeviceRole, NetBoxError> {
        let id = client.next_id();
        let slug_value = slug.map(|s| s.to_string()).unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));
        let device_role = DeviceRole {
            id,
            url: format!("{}/api/dcim/device-roles/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug_value,
            color: color.map(|s| s.to_string()),
            vm_role: vm_role.unwrap_or(false),
            description,
            comments,
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

pub async fn create_manufacturer(client: &MockNetBoxClient, name: &str, slug: Option<&str>, description: Option<String>) -> Result<Manufacturer, NetBoxError> {
        let id = client.next_id();
        let slug_value = slug.map(|s| s.to_string()).unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));
        let manufacturer = Manufacturer {
            id,
            url: format!("{}/api/dcim/manufacturers/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug_value,
            description,
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

pub async fn create_platform(client: &MockNetBoxClient, name: &str, slug: Option<&str>, manufacturer_id: Option<u64>, napalm_driver: Option<&str>, napalm_args: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Platform, NetBoxError> {
        let id = client.next_id();
        let slug_value = slug.map(|s| s.to_string()).unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));
        let platform = Platform {
            id,
            url: format!("{}/api/dcim/platforms/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug_value,
            manufacturer: manufacturer_id.map(|id| client.helpers().create_nested_manufacturer(id, None)),
            napalm_driver: napalm_driver.map(|s| s.to_string()),
            napalm_args: napalm_args.map(|s| s.to_string()),
            description,
            comments,
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

pub async fn create_device_type(client: &MockNetBoxClient, manufacturer_id: u64, model: &str, slug: Option<&str>, part_number: Option<&str>, u_height: Option<f64>, is_full_depth: Option<bool>, description: Option<String>, comments: Option<String>) -> Result<DeviceType, NetBoxError> {
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
            part_number: part_number.map(|s| s.to_string()),
            u_height: u_height.unwrap_or(0.0),
            is_full_depth: is_full_depth.unwrap_or(false),
            description,
            comments,
            device_count: 0,
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.device_types.lock().unwrap().insert((manufacturer_id, model.to_string()), device_type.clone());
        Ok(device_type)
    }

    // Tenancy Operations
