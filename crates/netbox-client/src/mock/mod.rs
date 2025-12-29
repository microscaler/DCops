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
use crate::types::*;
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

    /// Add a role to the mock store (for test setup)
    pub fn add_role(&self, role: Role) {
        self.roles.lock().unwrap().insert(role.id, role);
    }

    /// Add a device to the mock store (for test setup)
    pub fn add_device(&self, device: Device) {
        self.devices.lock().unwrap().insert(device.id, device);
    }

    /// Add a device type to the mock store (for test setup)
    pub fn add_device_type(&self, device_type: DeviceType) {
        let manufacturer_id = device_type.manufacturer.id;
        self.device_types.lock().unwrap().insert((manufacturer_id, device_type.model.clone()), device_type);
    }

    /// Add a device role to the mock store (for test setup)
    pub fn add_device_role(&self, device_role: DeviceRole) {
        self.device_roles.lock().unwrap().insert(device_role.name.clone(), device_role);
    }

    /// Add a manufacturer to the mock store (for test setup)
    pub fn add_manufacturer(&self, manufacturer: Manufacturer) {
        self.manufacturers.lock().unwrap().insert(manufacturer.name.clone(), manufacturer);
    }

    /// Add a platform to the mock store (for test setup)
    pub fn add_platform(&self, platform: Platform) {
        self.platforms.lock().unwrap().insert(platform.name.clone(), platform);
    }

    /// Add an aggregate to the mock store (for test setup)
    pub fn add_aggregate(&self, aggregate: Aggregate) {
        self.aggregates.lock().unwrap().insert(aggregate.id, aggregate);
    }

    /// Add an RIR to the mock store (for test setup)
    pub fn add_rir(&self, rir: Rir) {
        self.rirs.lock().unwrap().insert(rir.name.clone(), rir);
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
    async fn get_prefix(&self, id: PrefixId) -> Result<Prefix, NetBoxError> {
        ipam::get_prefix(self, id).await
    }

    async fn get_available_ips(&self, prefix_id: PrefixId, limit: Option<u32>) -> Result<Vec<AvailableIP>, NetBoxError> {
        ipam::get_available_ips(self, prefix_id, limit).await
    }

    async fn allocate_ip(&self, prefix_id: PrefixId, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError> {
        ipam::allocate_ip(self, prefix_id, request).await
    }

    async fn get_ip_address(&self, id: IpAddressId) -> Result<IPAddress, NetBoxError> {
        ipam::get_ip_address(self, id).await
    }

    async fn query_ip_addresses(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<IPAddress>, NetBoxError> {
        ipam::query_ip_addresses(self, filters, fetch_all).await
    }

    async fn query_prefixes(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Prefix>, NetBoxError> {
        ipam::query_prefixes(self, filters, fetch_all).await
    }

    async fn create_ip_address(&self, address: &ipnet::IpNet, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError> {
        ipam::create_ip_address(self, address.to_string().as_str(), request).await
    }

    async fn update_ip_address(&self, id: IpAddressId, request: AllocateIPRequest) -> Result<IPAddress, NetBoxError> {
        ipam::update_ip_address(self, id, request).await
    }

    async fn delete_ip_address(&self, id: IpAddressId) -> Result<(), NetBoxError> {
        ipam::delete_ip_address(self, id.into()).await
    }

    async fn create_prefix(&self, prefix: &ipnet::IpNet, description: Option<String>, site_id: Option<SiteId>, vlan_id: Option<VlanId>, status: Option<&str>, role_id: Option<RoleId>, tenant_id: Option<TenantId>, tags: Option<Vec<String>>) -> Result<Prefix, NetBoxError> {
        let prefix_str = prefix.to_string();
        ipam::create_prefix(self, prefix_str.as_str(), description, site_id, vlan_id, status, role_id, tenant_id, tags).await
    }

    async fn update_prefix(&self, id: PrefixId, prefix: Option<&ipnet::IpNet>, description: Option<String>, status: Option<&str>, role: Option<String>, tenant_id: Option<TenantId>, site_id: Option<SiteId>, vlan_id: Option<VlanId>, tags: Option<Vec<String>>) -> Result<Prefix, NetBoxError> {
        let prefix_str_opt = prefix.map(|p| p.to_string());
        ipam::update_prefix(self, id, prefix_str_opt.as_deref(), description, status, role, tenant_id, site_id, vlan_id, tags).await
    }

    async fn query_aggregates(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Aggregate>, NetBoxError> {
        ipam::query_aggregates(self, filters, fetch_all).await
    }

    async fn get_aggregate(&self, id: AggregateId) -> Result<Aggregate, NetBoxError> {
        ipam::get_aggregate(self, id).await
    }

    async fn create_aggregate(&self, prefix: &ipnet::IpNet, rir_id: Option<RirId>, date_allocated: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Aggregate, NetBoxError> {
        let prefix_str = prefix.to_string();
        ipam::create_aggregate(self, prefix_str.as_str(), rir_id, date_allocated, description, comments).await
    }

    async fn query_rirs(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Rir>, NetBoxError> {
        ipam::query_rirs(self, filters, fetch_all).await
    }

    async fn get_rir_by_name(&self, name: &str) -> Result<Option<Rir>, NetBoxError> {
        ipam::get_rir_by_name(self, name).await
    }

    async fn create_rir(&self, name: &str, slug: Option<&str>, description: Option<String>, is_private: Option<bool>) -> Result<Rir, NetBoxError> {
        ipam::create_rir(self, name, slug, description, is_private).await
    }

    async fn create_vlan(&self, vid: u16, name: &str, site_id: Option<SiteId>, group_id: Option<VlanGroupId>, tenant_id: Option<TenantId>, role_id: Option<RoleId>, status: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Vlan, NetBoxError> {
        ipam::create_vlan(self, vid, name, site_id, group_id, tenant_id, role_id, status, description, comments).await
    }

    async fn update_vlan(&self, id: VlanId, vid: Option<u16>, name: Option<&str>, site_id: Option<SiteId>, group_id: Option<VlanGroupId>, tenant_id: Option<TenantId>, role_id: Option<RoleId>, status: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Vlan, NetBoxError> {
        ipam::update_vlan(self, id, vid, name, site_id, group_id, tenant_id, role_id, status, description, comments).await
    }

    async fn query_vlans(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Vlan>, NetBoxError> {
        ipam::query_vlans(self, filters, fetch_all).await
    }

    async fn get_vlan(&self, id: VlanId) -> Result<Vlan, NetBoxError> {
        ipam::get_vlan(self, id).await
    }

    // DCIM Operations - delegated to dcim module
    async fn query_devices(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Device>, NetBoxError> {
        dcim::query_devices(self, filters, fetch_all).await
    }

    async fn get_device(&self, id: DeviceId) -> Result<Device, NetBoxError> {
        dcim::get_device(self, id.into()).await
    }

    async fn get_device_by_mac(&self, mac: &str) -> Result<Option<Device>, NetBoxError> {
        dcim::get_device_by_mac(self, mac).await
    }

    async fn create_device(&self, device_type_id: DeviceTypeId, device_role_id: DeviceRoleId, site_id: SiteId, name: Option<&str>, tenant_id: Option<TenantId>, platform_id: Option<PlatformId>, location_id: Option<LocationId>, serial: Option<&str>, asset_tag: Option<&str>, status: Option<&str>, primary_ip4_id: Option<IpAddressId>, primary_ip6_id: Option<IpAddressId>, description: Option<String>, comments: Option<String>) -> Result<Device, NetBoxError> {
        dcim::create_device(self, device_type_id.into(), device_role_id.into(), site_id.into(), name, tenant_id.map(|id| id.into()), platform_id.map(|id| id.into()), location_id.map(|id| id.into()), serial, asset_tag, status, primary_ip4_id.map(|id| id.into()), primary_ip6_id.map(|id| id.into()), description, comments).await
    }

    async fn update_device(&self, id: DeviceId, name: Option<&str>, tenant_id: Option<TenantId>, platform_id: Option<PlatformId>, location_id: Option<LocationId>, serial: Option<&str>, asset_tag: Option<&str>, status: Option<&str>, primary_ip4_id: Option<IpAddressId>, primary_ip6_id: Option<IpAddressId>, description: Option<String>, comments: Option<String>) -> Result<Device, NetBoxError> {
        dcim::update_device(self, id.into(), name, tenant_id.map(|id| id.into()), platform_id.map(|id| id.into()), location_id.map(|id| id.into()), serial, asset_tag, status, primary_ip4_id.map(|id| id.into()), primary_ip6_id.map(|id| id.into()), description, comments).await
    }

    async fn query_interfaces(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Interface>, NetBoxError> {
        dcim::query_interfaces(self, filters, fetch_all).await
    }

    async fn get_interface(&self, id: InterfaceId) -> Result<Interface, NetBoxError> {
        dcim::get_interface(self, id.into()).await
    }

    async fn create_interface(&self, device_id: DeviceId, name: &str, interface_type: &str, enabled: Option<bool>, mac_address: Option<&str>, mtu: Option<u16>, description: Option<String>) -> Result<Interface, NetBoxError> {
        dcim::create_interface(self, device_id.into(), name, interface_type, enabled, mac_address, mtu, description).await
    }

    async fn update_interface(&self, id: InterfaceId, name: Option<&str>, interface_type: Option<&str>, enabled: Option<bool>, mac_address: Option<&str>, mtu: Option<u16>, description: Option<String>) -> Result<Interface, NetBoxError> {
        dcim::update_interface(self, id.into(), name, interface_type, enabled, mac_address, mtu, description).await
    }

    async fn query_mac_addresses(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<MACAddress>, NetBoxError> {
        dcim::query_mac_addresses(self, filters, fetch_all).await
    }

    async fn get_mac_address_by_address(&self, mac: &str) -> Result<Option<MACAddress>, NetBoxError> {
        dcim::get_mac_address_by_address(self, mac).await
    }

    async fn create_mac_address(&self, mac_address: &str, assigned_object_type: &str, assigned_object_id: u64, description: Option<String>, comments: Option<String>) -> Result<MACAddress, NetBoxError> {
        dcim::create_mac_address(self, mac_address, assigned_object_type, assigned_object_id, description, comments).await
    }

    async fn query_sites(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Site>, NetBoxError> {
        dcim::query_sites(self, filters, fetch_all).await
    }

    async fn get_site(&self, id: SiteId) -> Result<Site, NetBoxError> {
        dcim::get_site(self, id.into()).await
    }

    async fn create_site(&self, name: &str, slug: Option<&str>, description: Option<String>, physical_address: Option<String>, shipping_address: Option<String>, latitude: Option<f64>, longitude: Option<f64>, tenant_id: Option<TenantId>, region_id: Option<RegionId>, site_group_id: Option<SiteGroupId>, status: Option<&str>, facility: Option<String>, time_zone: Option<String>, comments: Option<String>) -> Result<Site, NetBoxError> {
        dcim::create_site(self, name, slug, description, physical_address, shipping_address, latitude, longitude, tenant_id.map(|id| id.into()), region_id.map(|id| id.into()), site_group_id.map(|id| id.into()), status, facility, time_zone, comments).await
    }

    async fn update_site(&self, id: SiteId, name: Option<&str>, slug: Option<&str>, description: Option<String>, physical_address: Option<String>, shipping_address: Option<String>, latitude: Option<f64>, longitude: Option<f64>, tenant_id: Option<TenantId>, region_id: Option<RegionId>, site_group_id: Option<SiteGroupId>, status: Option<&str>, facility: Option<String>, time_zone: Option<String>, comments: Option<String>) -> Result<Site, NetBoxError> {
        dcim::update_site(self, id.into(), name, slug, description, physical_address, shipping_address, latitude, longitude, tenant_id.map(|id| id.into()), region_id.map(|id| id.into()), site_group_id.map(|id| id.into()), status, facility, time_zone, comments).await
    }

    async fn query_regions(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Region>, NetBoxError> {
        dcim::query_regions(self, filters, fetch_all).await
    }

    async fn get_region(&self, id: RegionId) -> Result<Region, NetBoxError> {
        dcim::get_region(self, id.into()).await
    }

    async fn get_region_by_name(&self, name: &str) -> Result<Option<Region>, NetBoxError> {
        dcim::get_region_by_name(self, name).await
    }

    async fn create_region(&self, name: &str, slug: Option<&str>, parent_id: Option<RegionId>, description: Option<String>, comments: Option<String>) -> Result<Region, NetBoxError> {
        dcim::create_region(self, name, slug, parent_id.map(|id| id.into()), description, comments).await
    }

    async fn query_site_groups(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<SiteGroup>, NetBoxError> {
        dcim::query_site_groups(self, filters, fetch_all).await
    }

    async fn get_site_group(&self, id: SiteGroupId) -> Result<SiteGroup, NetBoxError> {
        dcim::get_site_group(self, id.into()).await
    }

    async fn get_site_group_by_name(&self, name: &str) -> Result<Option<SiteGroup>, NetBoxError> {
        dcim::get_site_group_by_name(self, name).await
    }

    async fn create_site_group(&self, name: &str, slug: Option<&str>, parent_id: Option<SiteGroupId>, description: Option<String>, comments: Option<String>) -> Result<SiteGroup, NetBoxError> {
        dcim::create_site_group(self, name, slug, parent_id.map(|id| id.into()), description, comments).await
    }

    async fn query_locations(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Location>, NetBoxError> {
        dcim::query_locations(self, filters, fetch_all).await
    }

    async fn get_location(&self, id: LocationId) -> Result<Location, NetBoxError> {
        dcim::get_location(self, id.into()).await
    }

    async fn get_location_by_name(&self, site_id: SiteId, name: &str) -> Result<Option<Location>, NetBoxError> {
        dcim::get_location_by_name(self, site_id.into(), name).await
    }

    async fn create_location(&self, site_id: SiteId, name: &str, slug: Option<&str>, parent_id: Option<LocationId>, tenant_id: Option<TenantId>, facility: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Location, NetBoxError> {
        dcim::create_location(self, site_id.into(), name, slug, parent_id.map(|id| id.into()), tenant_id.map(|id| id.into()), facility, description, comments).await
    }

    async fn query_device_roles(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<DeviceRole>, NetBoxError> {
        dcim::query_device_roles(self, filters, fetch_all).await
    }

    async fn get_device_role_by_name(&self, name: &str) -> Result<Option<DeviceRole>, NetBoxError> {
        dcim::get_device_role_by_name(self, name).await
    }

    async fn create_device_role(&self, name: &str, slug: Option<&str>, color: Option<&str>, vm_role: Option<bool>, description: Option<String>, comments: Option<String>) -> Result<DeviceRole, NetBoxError> {
        dcim::create_device_role(self, name, slug, color, vm_role, description, comments).await
    }

    async fn query_manufacturers(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Manufacturer>, NetBoxError> {
        dcim::query_manufacturers(self, filters, fetch_all).await
    }

    async fn get_manufacturer_by_name(&self, name: &str) -> Result<Option<Manufacturer>, NetBoxError> {
        dcim::get_manufacturer_by_name(self, name).await
    }

    async fn create_manufacturer(&self, name: &str, slug: Option<&str>, description: Option<String>) -> Result<Manufacturer, NetBoxError> {
        dcim::create_manufacturer(self, name, slug, description).await
    }

    async fn query_platforms(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Platform>, NetBoxError> {
        dcim::query_platforms(self, filters, fetch_all).await
    }

    async fn get_platform_by_name(&self, name: &str) -> Result<Option<Platform>, NetBoxError> {
        dcim::get_platform_by_name(self, name).await
    }

    async fn create_platform(&self, name: &str, slug: Option<&str>, manufacturer_id: Option<ManufacturerId>, napalm_driver: Option<&str>, napalm_args: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Platform, NetBoxError> {
        dcim::create_platform(self, name, slug, manufacturer_id.map(|id| id.into()), napalm_driver, napalm_args, description, comments).await
    }

    async fn query_device_types(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<DeviceType>, NetBoxError> {
        dcim::query_device_types(self, filters, fetch_all).await
    }

    async fn get_device_type_by_model(&self, manufacturer_id: ManufacturerId, model: &str) -> Result<Option<DeviceType>, NetBoxError> {
        dcim::get_device_type_by_model(self, manufacturer_id.into(), model).await
    }

    async fn create_device_type(&self, manufacturer_id: ManufacturerId, model: &str, slug: Option<&str>, part_number: Option<&str>, u_height: Option<f64>, is_full_depth: Option<bool>, description: Option<String>, comments: Option<String>) -> Result<DeviceType, NetBoxError> {
        dcim::create_device_type(self, manufacturer_id.into(), model, slug, part_number, u_height, is_full_depth, description, comments).await
    }

    // Tenancy Operations - delegated to tenancy module
    async fn query_tenants(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Tenant>, NetBoxError> {
        tenancy::query_tenants(self, filters, fetch_all).await
    }

    async fn get_tenant(&self, id: TenantId) -> Result<Tenant, NetBoxError> {
        tenancy::get_tenant(self, id.into()).await
    }

    async fn create_tenant(&self, name: &str, slug: Option<&str>, description: Option<String>, comments: Option<String>, group: Option<TenantGroupId>) -> Result<Tenant, NetBoxError> {
        tenancy::create_tenant(self, name, slug, description, comments, group.map(|id| id.into())).await
    }

    async fn update_tenant(&self, id: TenantId, name: Option<&str>, slug: Option<&str>, description: Option<String>, comments: Option<String>, group: Option<TenantGroupId>) -> Result<Tenant, NetBoxError> {
        tenancy::update_tenant(self, id.into(), name, slug, description, comments, group.map(|id| id.into())).await
    }

    async fn query_tenant_groups(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<TenantGroup>, NetBoxError> {
        tenancy::query_tenant_groups(self, filters, fetch_all).await
    }

    async fn get_tenant_group_by_name(&self, name: &str) -> Result<Option<TenantGroup>, NetBoxError> {
        tenancy::get_tenant_group_by_name(self, name).await
    }

    async fn create_tenant_group(&self, name: &str, slug: Option<&str>, description: Option<String>, comments: Option<String>, parent_id: Option<TenantGroupId>) -> Result<TenantGroup, NetBoxError> {
        tenancy::create_tenant_group(self, name, slug, description, comments, parent_id.map(|id| id.into())).await
    }

    // Extras Operations - delegated to extras module
    async fn query_roles(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Role>, NetBoxError> {
        extras::query_roles(self, filters, fetch_all).await
    }

    async fn get_role(&self, id: RoleId) -> Result<Role, NetBoxError> {
        extras::get_role(self, id.into()).await
    }

    async fn create_role(&self, name: &str, slug: Option<&str>, description: Option<String>, weight: Option<u16>, comments: Option<String>) -> Result<Role, NetBoxError> {
        extras::create_role(self, name, slug, description, weight, comments).await
    }

    async fn query_tags(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Tag>, NetBoxError> {
        extras::query_tags(self, filters, fetch_all).await
    }

    async fn get_tag(&self, id: u64) -> Result<Tag, NetBoxError> {
        extras::get_tag(self, id).await
    }

    async fn create_tag(&self, name: &str, slug: Option<&str>, color: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Tag, NetBoxError> {
        extras::create_tag(self, name, slug, color, description, comments).await
    }
}

