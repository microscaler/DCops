//! Mock NetBoxClient for unit testing
//!
//! This module provides a mock implementation of NetBoxClientTrait that can be used
//! in unit tests without requiring a running NetBox instance.
//!
//! The mock is organized into domain-specific modules:
//! - `ipam.rs` - IPAM operations (prefixes, IP addresses, aggregates, RIRs, VLANs)
//! - `dcim.rs` - DCIM operations (sites, regions, devices, interfaces, etc.)
//! - `tenancy.rs` - Tenancy operations (tenants, tenant groups)
//! - `extras.rs` - Extras operations (roles, tags)
//! - `helpers.rs` - Helper functions for creating nested types

mod helpers;
mod ipam;
mod dcim;
mod tenancy;
mod extras;

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
    pub(crate) base_url: String,
    // In-memory storage for resources
    pub(crate) prefixes: Arc<Mutex<HashMap<u64, Prefix>>>,
    pub(crate) ip_addresses: Arc<Mutex<HashMap<u64, IPAddress>>>,
    pub(crate) available_ips: Arc<Mutex<HashMap<u64, Vec<AvailableIP>>>>,
    pub(crate) aggregates: Arc<Mutex<HashMap<u64, Aggregate>>>,
    pub(crate) rirs: Arc<Mutex<HashMap<String, Rir>>>,
    pub(crate) vlans: Arc<Mutex<HashMap<u64, Vlan>>>,
    pub(crate) sites: Arc<Mutex<HashMap<u64, Site>>>,
    pub(crate) regions: Arc<Mutex<HashMap<u64, Region>>>,
    pub(crate) site_groups: Arc<Mutex<HashMap<u64, SiteGroup>>>,
    pub(crate) locations: Arc<Mutex<HashMap<u64, Location>>>,
    pub(crate) devices: Arc<Mutex<HashMap<u64, Device>>>,
    pub(crate) interfaces: Arc<Mutex<HashMap<u64, Interface>>>,
    pub(crate) mac_addresses: Arc<Mutex<HashMap<String, MACAddress>>>,
    pub(crate) device_roles: Arc<Mutex<HashMap<String, DeviceRole>>>,
    pub(crate) manufacturers: Arc<Mutex<HashMap<String, Manufacturer>>>,
    pub(crate) platforms: Arc<Mutex<HashMap<String, Platform>>>,
    pub(crate) device_types: Arc<Mutex<HashMap<(u64, String), DeviceType>>>,
    pub(crate) tenants: Arc<Mutex<HashMap<u64, Tenant>>>,
    pub(crate) tenant_groups: Arc<Mutex<HashMap<String, TenantGroup>>>,
    pub(crate) roles: Arc<Mutex<HashMap<u64, Role>>>,
    pub(crate) tags: Arc<Mutex<HashMap<u64, Tag>>>,
    // Counter for generating IDs
    pub(crate) next_id: Arc<Mutex<u64>>,
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
    pub(crate) fn next_id(&self) -> u64 {
        let mut id = self.next_id.lock().unwrap();
        let current = *id;
        *id += 1;
        current
    }

    /// Get helpers instance
    pub(crate) fn helpers(&self) -> helpers::Helpers {
        helpers::Helpers::new(self.base_url.clone())
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

    // IPAM Operations - delegated to ipam module
    async fn get_prefix(&self, id: u64) -> Result<Prefix, NetBoxError> {
        ipam::get_prefix(self, id).await
    }

    async fn get_available_ips(&self, prefix_id: u64, limit: Option<u32>) -> Result<Vec<AvailableIP>, NetBoxError> {
        ipam::get_available_ips(self, prefix_id, limit).await
    }

    async fn allocate_ip(&self, prefix_id: u64, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError> {
        ipam::allocate_ip(self, prefix_id, request).await
    }

    async fn get_ip_address(&self, id: u64) -> Result<IPAddress, NetBoxError> {
        ipam::get_ip_address(self, id).await
    }

    async fn query_ip_addresses(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<IPAddress>, NetBoxError> {
        ipam::query_ip_addresses(self, filters, fetch_all).await
    }

    async fn query_prefixes(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Prefix>, NetBoxError> {
        ipam::query_prefixes(self, filters, fetch_all).await
    }

    async fn create_ip_address(&self, address: &str, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError> {
        ipam::create_ip_address(self, address, request).await
    }

    async fn update_ip_address(&self, id: u64, request: AllocateIPRequest) -> Result<IPAddress, NetBoxError> {
        ipam::update_ip_address(self, id, request).await
    }

    async fn delete_ip_address(&self, id: u64) -> Result<(), NetBoxError> {
        ipam::delete_ip_address(self, id).await
    }

    async fn create_prefix(&self, prefix: &str, site_id: Option<u64>, tenant_id: Option<u64>, vlan_id: Option<u32>, role_id: Option<u64>, status: Option<&str>, description: Option<&str>, tags: Option<Vec<serde_json::Value>>) -> Result<Prefix, NetBoxError> {
        ipam::create_prefix(self, prefix, site_id, tenant_id, vlan_id, role_id, status, description, tags).await
    }

    async fn update_prefix(&self, id: u64, site_id: Option<u64>, tenant_id: Option<u64>, vlan_id: Option<u32>, role_id: Option<u64>, status: Option<&str>, description: Option<&str>, tags: Option<Vec<serde_json::Value>>) -> Result<Prefix, NetBoxError> {
        ipam::update_prefix(self, id, site_id, tenant_id, vlan_id, role_id, status, description, tags).await
    }

    async fn query_aggregates(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Aggregate>, NetBoxError> {
        ipam::query_aggregates(self, filters, fetch_all).await
    }

    async fn get_aggregate(&self, id: u64) -> Result<Aggregate, NetBoxError> {
        ipam::get_aggregate(self, id).await
    }

    async fn create_aggregate(&self, prefix: &str, rir_id: u64, description: Option<&str>) -> Result<Aggregate, NetBoxError> {
        ipam::create_aggregate(self, prefix, rir_id, description).await
    }

    async fn query_rirs(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Rir>, NetBoxError> {
        ipam::query_rirs(self, filters, fetch_all).await
    }

    async fn get_rir_by_name(&self, name: &str) -> Result<Option<Rir>, NetBoxError> {
        ipam::get_rir_by_name(self, name).await
    }

    async fn create_rir(&self, name: &str, slug: &str, description: Option<&str>) -> Result<Rir, NetBoxError> {
        ipam::create_rir(self, name, slug, description).await
    }

    async fn create_vlan(&self, site_id: u64, vid: u32, name: &str, status: Option<&str>, description: Option<&str>) -> Result<Vlan, NetBoxError> {
        ipam::create_vlan(self, site_id, vid, name, status, description).await
    }

    async fn update_vlan(&self, id: u64, site_id: Option<u64>, vid: Option<u32>, name: Option<&str>, status: Option<&str>, description: Option<&str>) -> Result<Vlan, NetBoxError> {
        ipam::update_vlan(self, id, site_id, vid, name, status, description).await
    }

    async fn query_vlans(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Vlan>, NetBoxError> {
        ipam::query_vlans(self, filters, fetch_all).await
    }

    async fn get_vlan(&self, id: u64) -> Result<Vlan, NetBoxError> {
        ipam::get_vlan(self, id).await
    }

    // DCIM Operations - delegated to dcim module
    async fn query_devices(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Device>, NetBoxError> {
        dcim::query_devices(self, filters, fetch_all).await
    }

    async fn get_device(&self, id: u64) -> Result<Device, NetBoxError> {
        dcim::get_device(self, id).await
    }

    async fn get_device_by_mac(&self, mac: &str) -> Result<Option<Device>, NetBoxError> {
        dcim::get_device_by_mac(self, mac).await
    }

    async fn create_device(&self, name: &str, device_type_id: u64, device_role_id: u64, site_id: u64, location_id: Option<u64>, tenant_id: Option<u64>, platform_id: Option<u64>, serial: Option<&str>, asset_tag: Option<&str>, status: &str, primary_ip4_id: Option<u64>, primary_ip6_id: Option<u64>, description: Option<&str>, comments: Option<&str>) -> Result<Device, NetBoxError> {
        dcim::create_device(self, name, device_type_id, device_role_id, site_id, location_id, tenant_id, platform_id, serial, asset_tag, status, primary_ip4_id, primary_ip6_id, description, comments).await
    }

    async fn update_device(&self, id: u64, name: Option<&str>, device_type_id: Option<u64>, device_role_id: Option<u64>, site_id: Option<u64>, location_id: Option<u64>, tenant_id: Option<u64>, platform_id: Option<u64>, serial: Option<&str>, asset_tag: Option<&str>, status: Option<&str>, primary_ip4_id: Option<u64>, primary_ip6_id: Option<u64>, description: Option<&str>, comments: Option<&str>) -> Result<Device, NetBoxError> {
        dcim::update_device(self, id, name, device_type_id, device_role_id, site_id, location_id, tenant_id, platform_id, serial, asset_tag, status, primary_ip4_id, primary_ip6_id, description, comments).await
    }

    async fn query_interfaces(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Interface>, NetBoxError> {
        dcim::query_interfaces(self, filters, fetch_all).await
    }

    async fn get_interface(&self, id: u64) -> Result<Interface, NetBoxError> {
        dcim::get_interface(self, id).await
    }

    async fn create_interface(&self, device_id: u64, name: &str, interface_type: &str, enabled: bool, description: Option<&str>) -> Result<Interface, NetBoxError> {
        dcim::create_interface(self, device_id, name, interface_type, enabled, description).await
    }

    async fn update_interface(&self, id: u64, name: Option<&str>, interface_type: Option<&str>, enabled: Option<bool>, mac_address: Option<&str>, description: Option<&str>) -> Result<Interface, NetBoxError> {
        dcim::update_interface(self, id, name, interface_type, enabled, mac_address, description).await
    }

    async fn query_mac_addresses(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<MACAddress>, NetBoxError> {
        dcim::query_mac_addresses(self, filters, fetch_all).await
    }

    async fn get_mac_address_by_address(&self, mac: &str) -> Result<Option<MACAddress>, NetBoxError> {
        dcim::get_mac_address_by_address(self, mac).await
    }

    async fn create_mac_address(&self, interface_id: u64, address: &str, description: Option<&str>) -> Result<MACAddress, NetBoxError> {
        dcim::create_mac_address(self, interface_id, address, description).await
    }

    async fn query_sites(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Site>, NetBoxError> {
        dcim::query_sites(self, filters, fetch_all).await
    }

    async fn get_site(&self, id: u64) -> Result<Site, NetBoxError> {
        dcim::get_site(self, id).await
    }

    async fn create_site(&self, name: &str, slug: Option<&str>, status: &str, region_id: Option<u64>, site_group_id: Option<u64>, tenant_id: Option<u64>, facility: Option<&str>, time_zone: Option<&str>, description: Option<&str>, comments: Option<&str>) -> Result<Site, NetBoxError> {
        dcim::create_site(self, name, slug, status, region_id, site_group_id, tenant_id, facility, time_zone, description, comments).await
    }

    async fn update_site(&self, id: u64, name: Option<&str>, slug: Option<&str>, status: Option<&str>, region_id: Option<u64>, site_group_id: Option<u64>, tenant_id: Option<u64>, facility: Option<&str>, time_zone: Option<&str>, description: Option<&str>, comments: Option<&str>) -> Result<Site, NetBoxError> {
        dcim::update_site(self, id, name, slug, status, region_id, site_group_id, tenant_id, facility, time_zone, description, comments).await
    }

    async fn query_regions(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Region>, NetBoxError> {
        dcim::query_regions(self, filters, fetch_all).await
    }

    async fn get_region(&self, id: u64) -> Result<Region, NetBoxError> {
        dcim::get_region(self, id).await
    }

    async fn get_region_by_name(&self, name: &str) -> Result<Option<Region>, NetBoxError> {
        dcim::get_region_by_name(self, name).await
    }

    async fn create_region(&self, name: &str, slug: &str, description: Option<&str>) -> Result<Region, NetBoxError> {
        dcim::create_region(self, name, slug, description).await
    }

    async fn query_site_groups(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<SiteGroup>, NetBoxError> {
        dcim::query_site_groups(self, filters, fetch_all).await
    }

    async fn get_site_group(&self, id: u64) -> Result<SiteGroup, NetBoxError> {
        dcim::get_site_group(self, id).await
    }

    async fn get_site_group_by_name(&self, name: &str) -> Result<Option<SiteGroup>, NetBoxError> {
        dcim::get_site_group_by_name(self, name).await
    }

    async fn create_site_group(&self, name: &str, slug: &str, description: Option<&str>) -> Result<SiteGroup, NetBoxError> {
        dcim::create_site_group(self, name, slug, description).await
    }

    async fn query_locations(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Location>, NetBoxError> {
        dcim::query_locations(self, filters, fetch_all).await
    }

    async fn get_location(&self, id: u64) -> Result<Location, NetBoxError> {
        dcim::get_location(self, id).await
    }

    async fn get_location_by_name(&self, site_id: u64, name: &str) -> Result<Option<Location>, NetBoxError> {
        dcim::get_location_by_name(self, site_id, name).await
    }

    async fn create_location(&self, site_id: u64, name: &str, slug: Option<&str>, parent_id: Option<u64>, description: Option<String>, comments: Option<String>) -> Result<Location, NetBoxError> {
        dcim::create_location(self, site_id, name, slug, parent_id, description, comments).await
    }

    async fn query_device_roles(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<DeviceRole>, NetBoxError> {
        dcim::query_device_roles(self, filters, fetch_all).await
    }

    async fn get_device_role_by_name(&self, name: &str) -> Result<Option<DeviceRole>, NetBoxError> {
        dcim::get_device_role_by_name(self, name).await
    }

    async fn create_device_role(&self, name: &str, slug: &str, description: Option<&str>) -> Result<DeviceRole, NetBoxError> {
        dcim::create_device_role(self, name, slug, description).await
    }

    async fn query_manufacturers(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Manufacturer>, NetBoxError> {
        dcim::query_manufacturers(self, filters, fetch_all).await
    }

    async fn get_manufacturer_by_name(&self, name: &str) -> Result<Option<Manufacturer>, NetBoxError> {
        dcim::get_manufacturer_by_name(self, name).await
    }

    async fn create_manufacturer(&self, name: &str, slug: &str, description: Option<&str>) -> Result<Manufacturer, NetBoxError> {
        dcim::create_manufacturer(self, name, slug, description).await
    }

    async fn query_platforms(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Platform>, NetBoxError> {
        dcim::query_platforms(self, filters, fetch_all).await
    }

    async fn get_platform_by_name(&self, name: &str) -> Result<Option<Platform>, NetBoxError> {
        dcim::get_platform_by_name(self, name).await
    }

    async fn create_platform(&self, name: &str, slug: &str, description: Option<&str>) -> Result<Platform, NetBoxError> {
        dcim::create_platform(self, name, slug, description).await
    }

    async fn query_device_types(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<DeviceType>, NetBoxError> {
        dcim::query_device_types(self, filters, fetch_all).await
    }

    async fn get_device_type_by_model(&self, manufacturer_id: u64, model: &str) -> Result<Option<DeviceType>, NetBoxError> {
        dcim::get_device_type_by_model(self, manufacturer_id, model).await
    }

    async fn create_device_type(&self, manufacturer_id: u64, model: &str, slug: Option<&str>, description: Option<&str>) -> Result<DeviceType, NetBoxError> {
        dcim::create_device_type(self, manufacturer_id, model, slug, description).await
    }

    // Tenancy Operations - delegated to tenancy module
    async fn query_tenants(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Tenant>, NetBoxError> {
        tenancy::query_tenants(self, filters, fetch_all).await
    }

    async fn get_tenant(&self, id: u64) -> Result<Tenant, NetBoxError> {
        tenancy::get_tenant(self, id).await
    }

    async fn create_tenant(&self, name: &str, slug: &str, tenant_group_id: Option<u64>, description: Option<&str>, comments: Option<&str>) -> Result<Tenant, NetBoxError> {
        tenancy::create_tenant(self, name, slug, tenant_group_id, description, comments).await
    }

    async fn query_tenant_groups(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<TenantGroup>, NetBoxError> {
        tenancy::query_tenant_groups(self, filters, fetch_all).await
    }

    async fn get_tenant_group_by_name(&self, name: &str) -> Result<Option<TenantGroup>, NetBoxError> {
        tenancy::get_tenant_group_by_name(self, name).await
    }

    async fn create_tenant_group(&self, name: &str, slug: &str, description: Option<&str>) -> Result<TenantGroup, NetBoxError> {
        tenancy::create_tenant_group(self, name, slug, description).await
    }

    // Extras Operations - delegated to extras module
    async fn query_roles(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Role>, NetBoxError> {
        extras::query_roles(self, filters, fetch_all).await
    }

    async fn get_role(&self, id: u64) -> Result<Role, NetBoxError> {
        extras::get_role(self, id).await
    }

    async fn create_role(&self, name: &str, slug: &str, description: Option<&str>) -> Result<Role, NetBoxError> {
        extras::create_role(self, name, slug, description).await
    }

    async fn query_tags(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Tag>, NetBoxError> {
        extras::query_tags(self, filters, fetch_all).await
    }

    async fn get_tag(&self, id: u64) -> Result<Tag, NetBoxError> {
        extras::get_tag(self, id).await
    }

    async fn create_tag(&self, name: &str, slug: &str, description: Option<&str>) -> Result<Tag, NetBoxError> {
        extras::create_tag(self, name, slug, description).await
    }
}

