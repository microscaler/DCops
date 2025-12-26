//! Mock NetBoxClient for unit testing
//!
//! This module provides a mock implementation of NetBoxClientTrait that can be used
//! in unit tests without requiring a running NetBox instance.

use crate::error::NetBoxError;
use crate::models::*;
use crate::netbox_trait::NetBoxClientTrait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Mock NetBoxClient for testing
///
/// This mock stores resources in memory and can be configured to return
/// specific responses for testing different scenarios.
#[derive(Clone)]
pub struct MockNetBoxClient {
    base_url: String,
    // In-memory storage for resources
    prefixes: Arc<Mutex<HashMap<u64, Prefix>>>,
    ip_addresses: Arc<Mutex<HashMap<u64, IPAddress>>>,
    available_ips: Arc<Mutex<HashMap<u64, Vec<AvailableIP>>>>,
    aggregates: Arc<Mutex<HashMap<u64, Aggregate>>>,
    rirs: Arc<Mutex<HashMap<String, Rir>>>,
    vlans: Arc<Mutex<HashMap<u64, Vlan>>>,
    sites: Arc<Mutex<HashMap<u64, Site>>>,
    regions: Arc<Mutex<HashMap<u64, Region>>>,
    site_groups: Arc<Mutex<HashMap<u64, SiteGroup>>>,
    locations: Arc<Mutex<HashMap<u64, Location>>>,
    devices: Arc<Mutex<HashMap<u64, Device>>>,
    interfaces: Arc<Mutex<HashMap<u64, Interface>>>,
    mac_addresses: Arc<Mutex<HashMap<String, MACAddress>>>,
    device_roles: Arc<Mutex<HashMap<String, DeviceRole>>>,
    manufacturers: Arc<Mutex<HashMap<String, Manufacturer>>>,
    platforms: Arc<Mutex<HashMap<String, Platform>>>,
    device_types: Arc<Mutex<HashMap<(u64, String), DeviceType>>>,
    tenants: Arc<Mutex<HashMap<u64, Tenant>>>,
    tenant_groups: Arc<Mutex<HashMap<String, TenantGroup>>>,
    roles: Arc<Mutex<HashMap<u64, Role>>>,
    tags: Arc<Mutex<HashMap<u64, Tag>>>,
    // Counter for generating IDs
    next_id: Arc<Mutex<u64>>,
}

impl MockNetBoxClient {
    /// Create a new mock client
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            prefixes: Arc::new(Mutex::new(HashMap::new())),
            ip_addresses: Arc::new(Mutex::new(HashMap::new())),
            available_ips: Arc::new(Mutex::new(HashMap::new())),
            aggregates: Arc::new(Mutex::new(HashMap::new())),
            rirs: Arc::new(Mutex::new(HashMap::new())),
            vlans: Arc::new(Mutex::new(HashMap::new())),
            sites: Arc::new(Mutex::new(HashMap::new())),
            regions: Arc::new(Mutex::new(HashMap::new())),
            site_groups: Arc::new(Mutex::new(HashMap::new())),
            locations: Arc::new(Mutex::new(HashMap::new())),
            devices: Arc::new(Mutex::new(HashMap::new())),
            interfaces: Arc::new(Mutex::new(HashMap::new())),
            mac_addresses: Arc::new(Mutex::new(HashMap::new())),
            device_roles: Arc::new(Mutex::new(HashMap::new())),
            manufacturers: Arc::new(Mutex::new(HashMap::new())),
            platforms: Arc::new(Mutex::new(HashMap::new())),
            device_types: Arc::new(Mutex::new(HashMap::new())),
            tenants: Arc::new(Mutex::new(HashMap::new())),
            tenant_groups: Arc::new(Mutex::new(HashMap::new())),
            roles: Arc::new(Mutex::new(HashMap::new())),
            tags: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Add a prefix to the mock store (for test setup)
    pub fn add_prefix(&self, prefix: Prefix) {
        self.prefixes.lock().unwrap().insert(prefix.id, prefix);
    }

    /// Add an IP address to the mock store (for test setup)
    pub fn add_ip_address(&self, ip: IPAddress) {
        self.ip_addresses.lock().unwrap().insert(ip.id, ip);
    }

    /// Add available IPs for a prefix (for test setup)
    pub fn set_available_ips(&self, prefix_id: u64, ips: Vec<AvailableIP>) {
        self.available_ips.lock().unwrap().insert(prefix_id, ips);
    }

    /// Add a site to the mock store (for test setup)
    pub fn add_site(&self, site: Site) {
        self.sites.lock().unwrap().insert(site.id, site);
    }

    /// Add a tenant to the mock store (for test setup)
    pub fn add_tenant(&self, tenant: Tenant) {
        self.tenants.lock().unwrap().insert(tenant.id, tenant);
    }

    /// Add a tag to the mock store (for test setup)
    pub fn add_tag(&self, tag: Tag) {
        self.tags.lock().unwrap().insert(tag.id, tag);
    }

    /// Generate next ID
    fn next_id(&self) -> u64 {
        let mut id = self.next_id.lock().unwrap();
        let current = *id;
        *id += 1;
        current
    }
    
    /// Helper to create NestedTenant
    fn create_nested_tenant(&self, id: u64, name: Option<String>) -> NestedTenant {
        let name_str = name.unwrap_or_else(|| format!("Tenant {}", id));
        NestedTenant {
            id,
            url: format!("{}/api/tenancy/tenants/{}/", self.base_url, id),
            display: name_str.clone(),
            name: name_str.clone(),
            slug: name_str.to_lowercase().replace(' ', "-"),
        }
    }
    
    /// Helper to create NestedSite
    fn create_nested_site(&self, id: u64, name: Option<String>) -> NestedSite {
        let name_str = name.unwrap_or_else(|| format!("Site {}", id));
        NestedSite {
            id,
            url: format!("{}/api/dcim/sites/{}/", self.base_url, id),
            display: name_str.clone(),
            name: name_str.clone(),
            slug: name_str.to_lowercase().replace(' ', "-"),
        }
    }
    
    /// Helper to create NestedRegion
    fn create_nested_region(&self, id: u64, name: Option<String>) -> NestedRegion {
        let name_str = name.unwrap_or_else(|| format!("Region {}", id));
        NestedRegion {
            id,
            url: format!("{}/api/dcim/regions/{}/", self.base_url, id),
            display: name_str.clone(),
            name: name_str.clone(),
            slug: name_str.to_lowercase().replace(' ', "-"),
        }
    }
    
    /// Helper to create NestedSiteGroup
    fn create_nested_site_group(&self, id: u64, name: Option<String>) -> NestedSiteGroup {
        let name_str = name.unwrap_or_else(|| format!("Site Group {}", id));
        NestedSiteGroup {
            id,
            url: format!("{}/api/dcim/site-groups/{}/", self.base_url, id),
            display: name_str.clone(),
            name: name_str.clone(),
            slug: name_str.to_lowercase().replace(' ', "-"),
        }
    }
    
    /// Helper to create NestedVlan
    fn create_nested_vlan(&self, id: u64, vid: u16, name: Option<String>) -> NestedVlan {
        let name_str = name.unwrap_or_else(|| format!("VLAN {}", vid));
        NestedVlan {
            id,
            url: format!("{}/api/ipam/vlans/{}/", self.base_url, id),
            display: name_str.clone(),
            vid,
            name: name_str,
        }
    }
    
    /// Helper to create NestedRole
    fn create_nested_role(&self, id: u64, name: Option<String>) -> NestedRole {
        let name_str = name.unwrap_or_else(|| format!("Role {}", id));
        NestedRole {
            id,
            url: format!("{}/api/ipam/roles/{}/", self.base_url, id),
            display: name_str.clone(),
            name: name_str.clone(),
            slug: name_str.to_lowercase().replace(' ', "-"),
        }
    }
}

#[async_trait::async_trait]
impl NetBoxClientTrait for MockNetBoxClient {
    fn base_url(&self) -> &str {
        &self.base_url
    }

    async fn validate_token(&self) -> Result<(), NetBoxError> {
        Ok(())
    }

    // IPAM Operations
    async fn get_prefix(&self, id: u64) -> Result<Prefix, NetBoxError> {
        self.prefixes
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Prefix {} not found", id)))
    }

    async fn get_available_ips(&self, prefix_id: u64, _limit: Option<u32>) -> Result<Vec<AvailableIP>, NetBoxError> {
        Ok(self.available_ips
            .lock()
            .unwrap()
            .get(&prefix_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn allocate_ip(&self, prefix_id: u64, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError> {
        // Verify prefix exists
        self.get_prefix(prefix_id).await?;

        let id = self.next_id();
        let address = request
            .as_ref()
            .and_then(|r| r.address.clone())
            .unwrap_or_else(|| format!("192.168.1.{}", id));

        let ip = IPAddress {
            id,
            url: format!("{}/api/ipam/ip-addresses/{}/", self.base_url, id),
            display: address.clone(),
            family: 4, // Default to IPv4
            address: address.clone(),
            vrf: None,
            tenant: None, // AllocateIPRequest doesn't have tenant field
            status: request
                .as_ref()
                .and_then(|r| r.status.clone())
                .unwrap_or(IPAddressStatus::Active),
            role: request.as_ref().and_then(|r| r.role.clone()),
            assigned_object_type: None,
            assigned_object_id: None,
            assigned_object: None,
            nat_inside: None,
            nat_outside: vec![],
            dns_name: request.as_ref().and_then(|r| r.dns_name.clone()).unwrap_or_default(),
            description: request.as_ref().and_then(|r| r.description.clone()).unwrap_or_default(),
            comments: String::new(),
            tags: request.as_ref().and_then(|r| r.tags.clone())
                .map(|tags_vec| {
                    tags_vec.into_iter()
                        .filter_map(|v| v.as_str().map(|s| NestedTag {
                            id: 0,
                            url: format!("{}/api/extras/tags/0/", self.base_url),
                            display: s.to_string(),
                            name: s.to_string(),
                            slug: s.to_lowercase().replace(' ', "-"),
                        }))
                        .collect()
                })
                .unwrap_or_default(),
            custom_fields: serde_json::json!({}),
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        self.ip_addresses.lock().unwrap().insert(id, ip.clone());
        Ok(ip)
    }

    async fn get_ip_address(&self, id: u64) -> Result<IPAddress, NetBoxError> {
        self.ip_addresses
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("IP address {} not found", id)))
    }

    async fn query_ip_addresses(&self, filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<IPAddress>, NetBoxError> {
        let ips = self.ip_addresses.lock().unwrap();
        let mut results: Vec<IPAddress> = ips.values().cloned().collect();

        // Apply filters (simplified - only handles prefix filter)
        for (key, value) in filters {
            if *key == "prefix" {
                results.retain(|ip| ip.address.starts_with(value));
            }
        }

        Ok(results)
    }

    async fn query_prefixes(&self, filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Prefix>, NetBoxError> {
        let prefixes = self.prefixes.lock().unwrap();
        Ok(prefixes.values().cloned().collect())
    }

    async fn create_ip_address(&self, address: &str, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError> {
        let id = self.next_id();
        let ip = IPAddress {
            id,
            url: format!("{}/api/ipam/ip-addresses/{}/", self.base_url, id),
            display: address.to_string(),
            family: if address.contains(':') { 6 } else { 4 },
            address: address.to_string(),
            vrf: None,
            tenant: None, // AllocateIPRequest doesn't have tenant field
            status: request
                .as_ref()
                .and_then(|r| r.status.clone())
                .unwrap_or(IPAddressStatus::Active),
            role: request.as_ref().and_then(|r| r.role.clone()),
            assigned_object_type: None,
            assigned_object_id: None,
            assigned_object: None,
            nat_inside: None,
            nat_outside: vec![],
            dns_name: request.as_ref().and_then(|r| r.dns_name.clone()).unwrap_or_default(),
            description: request.as_ref().and_then(|r| r.description.clone()).unwrap_or_default(),
            comments: String::new(),
            tags: request.as_ref().and_then(|r| r.tags.clone())
                .map(|tags_vec| {
                    tags_vec.into_iter()
                        .filter_map(|v| v.as_str().map(|s| NestedTag {
                            id: 0,
                            url: format!("{}/api/extras/tags/0/", self.base_url),
                            display: s.to_string(),
                            name: s.to_string(),
                            slug: s.to_lowercase().replace(' ', "-"),
                        }))
                        .collect()
                })
                .unwrap_or_default(),
            custom_fields: serde_json::json!({}),
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        self.ip_addresses.lock().unwrap().insert(id, ip.clone());
        Ok(ip)
    }

    async fn update_ip_address(&self, id: u64, request: AllocateIPRequest) -> Result<IPAddress, NetBoxError> {
        let mut ips = self.ip_addresses.lock().unwrap();
        let ip = ips
            .get_mut(&id)
            .ok_or_else(|| NetBoxError::NotFound(format!("IP address {} not found", id)))?;

        if let Some(description) = request.description {
            ip.description = description;
        }
        if let Some(status) = request.status {
            ip.status = status;
        }
        if let Some(dns_name) = request.dns_name {
            ip.dns_name = dns_name;
        }
        if let Some(role) = request.role {
            ip.role = Some(role);
        }
        if let Some(tags) = request.tags {
            ip.tags = tags.into_iter()
                .filter_map(|v| v.as_str().map(|s| NestedTag {
                    id: 0,
                    url: format!("{}/api/extras/tags/0/", self.base_url),
                    display: s.to_string(),
                    name: s.to_string(),
                    slug: s.to_lowercase().replace(' ', "-"),
                }))
                .collect();
        }

        Ok(ip.clone())
    }

    async fn delete_ip_address(&self, id: u64) -> Result<(), NetBoxError> {
        self.ip_addresses
            .lock()
            .unwrap()
            .remove(&id)
            .ok_or_else(|| NetBoxError::NotFound(format!("IP address {} not found", id)))
            .map(|_| ())
    }

    async fn create_prefix(&self, prefix: &str, site_id: Option<u64>, tenant_id: Option<u64>, vlan_id: Option<u32>, role_id: Option<u64>, status: Option<&str>, description: Option<&str>, tags: Option<Vec<serde_json::Value>>) -> Result<Prefix, NetBoxError> {
        let id = self.next_id();
        let status_str = status.unwrap_or("active");
        let prefix_status = match status_str {
            "active" => PrefixStatus::Active,
            "reserved" => PrefixStatus::Reserved,
            "deprecated" => PrefixStatus::Deprecated,
            "container" => PrefixStatus::Container,
            _ => PrefixStatus::Active,
        };
        
        let tags_vec: Vec<NestedTag> = tags
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_str().map(|s| NestedTag {
                id: 0,
                url: format!("{}/api/extras/tags/{}/", self.base_url, 0),
                display: s.to_string(),
                name: s.to_string(),
                slug: s.to_lowercase().replace(' ', "-"),
            }))
            .collect();
        
        let prefix_obj = Prefix {
            id,
            url: format!("{}/api/ipam/prefixes/{}/", self.base_url, id),
            display: prefix.to_string(),
            family: if prefix.contains(':') { 6 } else { 4 },
            prefix: prefix.to_string(),
            vrf: None,
            tenant: tenant_id.map(|id| self.create_nested_tenant(id, None)),
            vlan: vlan_id.map(|id| self.create_nested_vlan(id as u64, id as u16, None)),
            status: prefix_status,
            role: role_id.map(|id| self.create_nested_role(id, None)),
            is_pool: false,
            mark_utilized: false,
            description: description.map(|s| s.to_string()).unwrap_or_default(),
            comments: String::new(),
            tags: tags_vec,
            custom_fields: serde_json::json!({}),
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
            children: 0,
            _depth: 0,
        };

        self.prefixes.lock().unwrap().insert(id, prefix_obj.clone());
        Ok(prefix_obj)
    }

    async fn update_prefix(&self, id: u64, site_id: Option<u64>, tenant_id: Option<u64>, vlan_id: Option<u32>, role_id: Option<u64>, status: Option<&str>, description: Option<&str>, tags: Option<Vec<serde_json::Value>>) -> Result<Prefix, NetBoxError> {
        let mut prefixes = self.prefixes.lock().unwrap();
        let prefix = prefixes
            .get_mut(&id)
            .ok_or_else(|| NetBoxError::NotFound(format!("Prefix {} not found", id)))?;

        // Prefix model doesn't have a site field - skip
        if let Some(tenant) = tenant_id {
            prefix.tenant = Some(self.create_nested_tenant(tenant, None));
        }
        if let Some(vlan) = vlan_id {
            prefix.vlan = Some(self.create_nested_vlan(vlan as u64, vlan as u16, None));
        }
        if let Some(role) = role_id {
            prefix.role = Some(self.create_nested_role(role, None));
        }
        if let Some(status_str) = status {
            prefix.status = match status_str {
                "active" => PrefixStatus::Active,
                "reserved" => PrefixStatus::Reserved,
                "deprecated" => PrefixStatus::Deprecated,
                "container" => PrefixStatus::Container,
                _ => PrefixStatus::Active,
            };
        }
        if let Some(desc) = description {
            prefix.description = desc.to_string();
        }
        if let Some(tags_val) = tags {
            prefix.tags = tags_val.into_iter()
                .filter_map(|v| v.as_str().map(|s| NestedTag {
                    id: 0,
                    url: format!("{}/api/extras/tags/0/", self.base_url),
                    display: s.to_string(),
                    name: s.to_string(),
                    slug: s.to_lowercase().replace(' ', "-"),
                }))
                .collect();
        }

        Ok(prefix.clone())
    }

    async fn query_aggregates(&self, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Aggregate>, NetBoxError> {
        let aggregates = self.aggregates.lock().unwrap();
        Ok(aggregates.values().cloned().collect())
    }

    async fn get_aggregate(&self, id: u64) -> Result<Aggregate, NetBoxError> {
        self.aggregates
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Aggregate {} not found", id)))
    }

    async fn create_aggregate(&self, prefix: &str, rir_id: u64, description: Option<&str>) -> Result<Aggregate, NetBoxError> {
        let id = self.next_id();
        let aggregate = Aggregate {
            id,
            url: format!("{}/api/ipam/aggregates/{}/", self.base_url, id),
            display: prefix.to_string(),
            prefix: prefix.to_string(),
            rir: Some(NestedRir {
                id: rir_id,
                url: format!("{}/api/ipam/rirs/{}/", self.base_url, rir_id),
                display: format!("RIR {}", rir_id),
                name: format!("RIR {}", rir_id),
                slug: format!("rir-{}", rir_id),
            }),
            date_allocated: None,
            description: description.map(|s| s.to_string()),
            comments: None,
            tags: vec![],
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        self.aggregates.lock().unwrap().insert(id, aggregate.clone());
        Ok(aggregate)
    }

    async fn query_rirs(&self, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Rir>, NetBoxError> {
        let rirs = self.rirs.lock().unwrap();
        Ok(rirs.values().cloned().collect())
    }

    async fn get_rir_by_name(&self, name: &str) -> Result<Option<Rir>, NetBoxError> {
        Ok(self.rirs.lock().unwrap().get(name).cloned())
    }

    async fn create_rir(&self, name: &str, slug: &str, description: Option<&str>) -> Result<Rir, NetBoxError> {
        let id = self.next_id();
        let rir = Rir {
            id,
            url: format!("{}/api/ipam/rirs/{}/", self.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug.to_string(),
            description: description.map(|s| s.to_string()),
            is_private: false,
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        self.rirs.lock().unwrap().insert(name.to_string(), rir.clone());
        Ok(rir)
    }

    async fn create_vlan(&self, site_id: u64, vid: u32, name: &str, status: Option<&str>, description: Option<&str>) -> Result<Vlan, NetBoxError> {
        let id = self.next_id();
        let status_str = status.unwrap_or("active");
        let vlan_status = match status_str {
            "active" => VlanStatus::Active,
            "reserved" => VlanStatus::Reserved,
            "deprecated" => VlanStatus::Deprecated,
            _ => VlanStatus::Active,
        };
        
        let vlan = Vlan {
            id,
            url: format!("{}/api/ipam/vlans/{}/", self.base_url, id),
            display: name.to_string(),
            site: Some(self.create_nested_site(site_id, None)),
            group: None,
            vid: vid as u16,
            name: name.to_string(),
            tenant: None,
            status: vlan_status,
            role: None,
            description: description.map(|s| s.to_string()).unwrap_or_default(),
            comments: String::new(),
            tags: vec![],
            custom_fields: serde_json::json!({}),
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        self.vlans.lock().unwrap().insert(id, vlan.clone());
        Ok(vlan)
    }

    async fn update_vlan(&self, id: u64, site_id: Option<u64>, vid: Option<u32>, name: Option<&str>, status: Option<&str>, description: Option<&str>) -> Result<Vlan, NetBoxError> {
        let mut vlans = self.vlans.lock().unwrap();
        let vlan = vlans
            .get_mut(&id)
            .ok_or_else(|| NetBoxError::NotFound(format!("VLAN {} not found", id)))?;

        if let Some(site_id_val) = site_id {
            vlan.site = Some(self.create_nested_site(site_id_val, None));
        }
        if let Some(vid_val) = vid {
            vlan.vid = vid_val as u16;
        }
        if let Some(name_str) = name {
            vlan.name = name_str.to_string();
        }
        if let Some(status_str) = status {
            vlan.status = match status_str {
                "active" => VlanStatus::Active,
                "reserved" => VlanStatus::Reserved,
                "deprecated" => VlanStatus::Deprecated,
                _ => VlanStatus::Active,
            };
        }
        if let Some(desc) = description {
            vlan.description = desc.to_string();
        }

        Ok(vlan.clone())
    }

    async fn query_vlans(&self, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Vlan>, NetBoxError> {
        let vlans = self.vlans.lock().unwrap();
        Ok(vlans.values().cloned().collect())
    }

    async fn get_vlan(&self, id: u64) -> Result<Vlan, NetBoxError> {
        self.vlans
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("VLAN {} not found", id)))
    }

    // DCIM Operations - Stub implementations for now
    async fn query_devices(&self, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Device>, NetBoxError> {
        let devices = self.devices.lock().unwrap();
        Ok(devices.values().cloned().collect())
    }

    async fn get_device(&self, id: u64) -> Result<Device, NetBoxError> {
        self.devices
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Device {} not found", id)))
    }

    async fn get_device_by_mac(&self, _mac: &str) -> Result<Option<Device>, NetBoxError> {
        Ok(None)
    }

    async fn create_device(&self, _name: &str, _device_type_id: u64, _device_role_id: u64, _site_id: u64, _location_id: Option<u64>, _tenant_id: Option<u64>, _platform_id: Option<u64>, _serial: Option<&str>, _asset_tag: Option<&str>, _status: &str, _primary_ip4_id: Option<u64>, _primary_ip6_id: Option<u64>, _description: Option<&str>, _comments: Option<&str>) -> Result<Device, NetBoxError> {
        Err(NetBoxError::Api("Not implemented in mock".to_string()))
    }

    async fn update_device(&self, _id: u64, _name: Option<&str>, _device_type_id: Option<u64>, _device_role_id: Option<u64>, _site_id: Option<u64>, _location_id: Option<u64>, _tenant_id: Option<u64>, _platform_id: Option<u64>, _serial: Option<&str>, _asset_tag: Option<&str>, _status: Option<&str>, _primary_ip4_id: Option<u64>, _primary_ip6_id: Option<u64>, _description: Option<&str>, _comments: Option<&str>) -> Result<Device, NetBoxError> {
        Err(NetBoxError::Api("Not implemented in mock".to_string()))
    }

    async fn query_interfaces(&self, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Interface>, NetBoxError> {
        Ok(vec![])
    }

    async fn get_interface(&self, id: u64) -> Result<Interface, NetBoxError> {
        self.interfaces
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Interface {} not found", id)))
    }

    async fn create_interface(&self, _device_id: u64, _name: &str, _interface_type: &str, _enabled: bool, _description: Option<&str>) -> Result<Interface, NetBoxError> {
        Err(NetBoxError::Api("Not implemented in mock".to_string()))
    }

    async fn update_interface(&self, _id: u64, _name: Option<&str>, _interface_type: Option<&str>, _enabled: Option<bool>, _mac_address: Option<&str>, _description: Option<&str>) -> Result<Interface, NetBoxError> {
        Err(NetBoxError::Api("Not implemented in mock".to_string()))
    }

    async fn query_mac_addresses(&self, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<MACAddress>, NetBoxError> {
        Ok(vec![])
    }

    async fn get_mac_address_by_address(&self, mac: &str) -> Result<Option<MACAddress>, NetBoxError> {
        Ok(self.mac_addresses.lock().unwrap().get(mac).cloned())
    }

    async fn create_mac_address(&self, _interface_id: u64, _address: &str, _description: Option<&str>) -> Result<MACAddress, NetBoxError> {
        Err(NetBoxError::Api("Not implemented in mock".to_string()))
    }

    async fn query_sites(&self, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Site>, NetBoxError> {
        let sites = self.sites.lock().unwrap();
        Ok(sites.values().cloned().collect())
    }

    async fn get_site(&self, id: u64) -> Result<Site, NetBoxError> {
        self.sites
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Site {} not found", id)))
    }

    async fn create_site(&self, name: &str, slug: Option<&str>, status: &str, region_id: Option<u64>, site_group_id: Option<u64>, tenant_id: Option<u64>, facility: Option<&str>, time_zone: Option<&str>, description: Option<&str>, comments: Option<&str>) -> Result<Site, NetBoxError> {
        let id = self.next_id();
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
            url: format!("{}/api/dcim/sites/{}/", self.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug_value,
            status: status_enum,
            region: region_id.map(|id| self.create_nested_region(id, None)),
            site_group: site_group_id.map(|id| self.create_nested_site_group(id, None)),
            tenant: tenant_id.map(|id| self.create_nested_tenant(id, None)),
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

        self.sites.lock().unwrap().insert(id, site.clone());
        Ok(site)
    }

    async fn update_site(&self, id: u64, name: Option<&str>, slug: Option<&str>, status: Option<&str>, region_id: Option<u64>, site_group_id: Option<u64>, tenant_id: Option<u64>, facility: Option<&str>, time_zone: Option<&str>, description: Option<&str>, comments: Option<&str>) -> Result<Site, NetBoxError> {
        let mut sites = self.sites.lock().unwrap();
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
            site.region = Some(self.create_nested_region(region, None));
        }
        if let Some(site_group) = site_group_id {
            site.site_group = Some(self.create_nested_site_group(site_group, None));
        }
        if let Some(tenant) = tenant_id {
            site.tenant = Some(self.create_nested_tenant(tenant, None));
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

    async fn query_regions(&self, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Region>, NetBoxError> {
        let regions = self.regions.lock().unwrap();
        Ok(regions.values().cloned().collect())
    }

    async fn get_region(&self, id: u64) -> Result<Region, NetBoxError> {
        self.regions
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Region {} not found", id)))
    }

    async fn get_region_by_name(&self, name: &str) -> Result<Option<Region>, NetBoxError> {
        let regions = self.regions.lock().unwrap();
        Ok(regions.values().find(|r| r.name == name).cloned())
    }

    async fn create_region(&self, name: &str, slug: &str, description: Option<&str>) -> Result<Region, NetBoxError> {
        let id = self.next_id();
        let region = Region {
            id,
            url: format!("{}/api/dcim/regions/{}/", self.base_url, id),
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

        self.regions.lock().unwrap().insert(id, region.clone());
        Ok(region)
    }

    async fn query_site_groups(&self, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<SiteGroup>, NetBoxError> {
        let site_groups = self.site_groups.lock().unwrap();
        Ok(site_groups.values().cloned().collect())
    }

    async fn get_site_group(&self, id: u64) -> Result<SiteGroup, NetBoxError> {
        self.site_groups
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Site group {} not found", id)))
    }

    async fn get_site_group_by_name(&self, name: &str) -> Result<Option<SiteGroup>, NetBoxError> {
        let site_groups = self.site_groups.lock().unwrap();
        Ok(site_groups.values().find(|sg| sg.name == name).cloned())
    }

    async fn create_site_group(&self, name: &str, slug: &str, description: Option<&str>) -> Result<SiteGroup, NetBoxError> {
        let id = self.next_id();
        let site_group = SiteGroup {
            id,
            url: format!("{}/api/dcim/site-groups/{}/", self.base_url, id),
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

        self.site_groups.lock().unwrap().insert(id, site_group.clone());
        Ok(site_group)
    }

    async fn query_locations(&self, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Location>, NetBoxError> {
        let locations = self.locations.lock().unwrap();
        Ok(locations.values().cloned().collect())
    }

    async fn get_location(&self, id: u64) -> Result<Location, NetBoxError> {
        self.locations
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Location {} not found", id)))
    }

    async fn get_location_by_name(&self, site_id: u64, name: &str) -> Result<Option<Location>, NetBoxError> {
        let locations = self.locations.lock().unwrap();
        Ok(locations.values().find(|l| l.site.id == site_id && l.name == name).cloned())
    }

    async fn create_location(&self, site_id: u64, name: &str, slug: Option<&str>, parent_id: Option<u64>, description: Option<String>, comments: Option<String>) -> Result<Location, NetBoxError> {
        let id = self.next_id();
        let slug_value = slug.map(|s| s.to_string()).unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));
        let location = Location {
            id,
            url: format!("{}/api/dcim/locations/{}/", self.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug_value,
            site: self.create_nested_site(site_id, None),
            parent: parent_id.map(|id| NestedLocation {
                id,
                url: format!("{}/api/dcim/locations/{}/", self.base_url, id),
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

        self.locations.lock().unwrap().insert(id, location.clone());
        Ok(location)
    }

    async fn query_device_roles(&self, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<DeviceRole>, NetBoxError> {
        let device_roles = self.device_roles.lock().unwrap();
        Ok(device_roles.values().cloned().collect())
    }

    async fn get_device_role_by_name(&self, name: &str) -> Result<Option<DeviceRole>, NetBoxError> {
        Ok(self.device_roles.lock().unwrap().get(name).cloned())
    }

    async fn create_device_role(&self, name: &str, slug: &str, description: Option<&str>) -> Result<DeviceRole, NetBoxError> {
        let id = self.next_id();
        let device_role = DeviceRole {
            id,
            url: format!("{}/api/dcim/device-roles/{}/", self.base_url, id),
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

        self.device_roles.lock().unwrap().insert(name.to_string(), device_role.clone());
        Ok(device_role)
    }

    async fn query_manufacturers(&self, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Manufacturer>, NetBoxError> {
        let manufacturers = self.manufacturers.lock().unwrap();
        Ok(manufacturers.values().cloned().collect())
    }

    async fn get_manufacturer_by_name(&self, name: &str) -> Result<Option<Manufacturer>, NetBoxError> {
        Ok(self.manufacturers.lock().unwrap().get(name).cloned())
    }

    async fn create_manufacturer(&self, name: &str, slug: &str, description: Option<&str>) -> Result<Manufacturer, NetBoxError> {
        let id = self.next_id();
        let manufacturer = Manufacturer {
            id,
            url: format!("{}/api/dcim/manufacturers/{}/", self.base_url, id),
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

        self.manufacturers.lock().unwrap().insert(name.to_string(), manufacturer.clone());
        Ok(manufacturer)
    }

    async fn query_platforms(&self, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Platform>, NetBoxError> {
        let platforms = self.platforms.lock().unwrap();
        Ok(platforms.values().cloned().collect())
    }

    async fn get_platform_by_name(&self, name: &str) -> Result<Option<Platform>, NetBoxError> {
        Ok(self.platforms.lock().unwrap().get(name).cloned())
    }

    async fn create_platform(&self, name: &str, slug: &str, description: Option<&str>) -> Result<Platform, NetBoxError> {
        let id = self.next_id();
        let platform = Platform {
            id,
            url: format!("{}/api/dcim/platforms/{}/", self.base_url, id),
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

        self.platforms.lock().unwrap().insert(name.to_string(), platform.clone());
        Ok(platform)
    }

    async fn query_device_types(&self, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<DeviceType>, NetBoxError> {
        let device_types = self.device_types.lock().unwrap();
        Ok(device_types.values().cloned().collect())
    }

    async fn get_device_type_by_model(&self, manufacturer_id: u64, model: &str) -> Result<Option<DeviceType>, NetBoxError> {
        Ok(self.device_types.lock().unwrap().get(&(manufacturer_id, model.to_string())).cloned())
    }

    async fn create_device_type(&self, manufacturer_id: u64, model: &str, slug: Option<&str>, description: Option<&str>) -> Result<DeviceType, NetBoxError> {
        let id = self.next_id();
        let slug_value = slug.map(|s| s.to_string()).unwrap_or_else(|| model.to_lowercase().replace(' ', "-"));
        let device_type = DeviceType {
            id,
            url: format!("{}/api/dcim/device-types/{}/", self.base_url, id),
            display: model.to_string(),
            manufacturer: NestedManufacturer {
                id: manufacturer_id,
                url: format!("{}/api/dcim/manufacturers/{}/", self.base_url, manufacturer_id),
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

        self.device_types.lock().unwrap().insert((manufacturer_id, model.to_string()), device_type.clone());
        Ok(device_type)
    }

    // Tenancy Operations
    async fn query_tenants(&self, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Tenant>, NetBoxError> {
        let tenants = self.tenants.lock().unwrap();
        Ok(tenants.values().cloned().collect())
    }

    async fn get_tenant(&self, id: u64) -> Result<Tenant, NetBoxError> {
        self.tenants
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Tenant {} not found", id)))
    }

    async fn create_tenant(&self, name: &str, slug: &str, tenant_group_id: Option<u64>, description: Option<&str>, comments: Option<&str>) -> Result<Tenant, NetBoxError> {
        let id = self.next_id();
        let tenant = Tenant {
            id,
            url: format!("{}/api/tenancy/tenants/{}/", self.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug.to_string(),
            description: description.map(|s| s.to_string()),
            comments: comments.map(|s| s.to_string()),
            group: tenant_group_id.map(|id| NestedTenantGroup {
                id,
                url: format!("{}/api/tenancy/tenant-groups/{}/", self.base_url, id),
                display: format!("Tenant Group {}", id),
                name: format!("Tenant Group {}", id),
                slug: format!("tenant-group-{}", id),
            }),
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        self.tenants.lock().unwrap().insert(id, tenant.clone());
        Ok(tenant)
    }

    async fn query_tenant_groups(&self, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<TenantGroup>, NetBoxError> {
        let tenant_groups = self.tenant_groups.lock().unwrap();
        Ok(tenant_groups.values().cloned().collect())
    }

    async fn get_tenant_group_by_name(&self, name: &str) -> Result<Option<TenantGroup>, NetBoxError> {
        Ok(self.tenant_groups.lock().unwrap().get(name).cloned())
    }

    async fn create_tenant_group(&self, name: &str, slug: &str, description: Option<&str>) -> Result<TenantGroup, NetBoxError> {
        let id = self.next_id();
        let tenant_group = TenantGroup {
            id,
            url: format!("{}/api/tenancy/tenant-groups/{}/", self.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug.to_string(),
            description: description.map(|s| s.to_string()),
            comments: None,
            parent: None,
            tenant_count: 0,
            _depth: None,
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        self.tenant_groups.lock().unwrap().insert(name.to_string(), tenant_group.clone());
        Ok(tenant_group)
    }

    // Extras Operations
    async fn query_roles(&self, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Role>, NetBoxError> {
        let roles = self.roles.lock().unwrap();
        Ok(roles.values().cloned().collect())
    }

    async fn get_role(&self, id: u64) -> Result<Role, NetBoxError> {
        self.roles
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Role {} not found", id)))
    }

    async fn create_role(&self, name: &str, slug: &str, description: Option<&str>) -> Result<Role, NetBoxError> {
        let id = self.next_id();
        let role = Role {
            id,
            url: format!("{}/api/extras/roles/{}/", self.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug.to_string(),
            description: description.map(|s| s.to_string()),
            weight: None,
            comments: None,
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        self.roles.lock().unwrap().insert(id, role.clone());
        Ok(role)
    }

    async fn query_tags(&self, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Tag>, NetBoxError> {
        let tags = self.tags.lock().unwrap();
        Ok(tags.values().cloned().collect())
    }

    async fn get_tag(&self, id: u64) -> Result<Tag, NetBoxError> {
        self.tags
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Tag {} not found", id)))
    }

    async fn create_tag(&self, name: &str, slug: &str, description: Option<&str>) -> Result<Tag, NetBoxError> {
        let id = self.next_id();
        let tag = Tag {
            id,
            url: format!("{}/api/extras/tags/{}/", self.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug.to_string(),
            color: None, // DeviceRole.color is Option<String>
            description: description.map(|s| s.to_string()),
            comments: None,
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        self.tags.lock().unwrap().insert(id, tag.clone());
        Ok(tag)
    }
}

