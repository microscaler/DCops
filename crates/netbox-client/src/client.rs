//! NetBox API client
//!
//! Implements the NetBox REST API client for IPAM, DCIM, Tenancy, and Extras operations.
//! This is the main client that composes resource-specific modules.

use crate::core::NetBoxClientCore;
use crate::dcim;
use crate::error::NetBoxError;
use crate::extras;
use crate::ipam;
use crate::models::*;
use crate::netbox_trait::NetBoxClientTrait;
use crate::tenancy;
use crate::types::*;

/// NetBox API client
///
/// This client composes resource-specific modules (IPAM, DCIM, Tenancy, Extras)
/// and provides a unified interface for interacting with the NetBox API.
pub struct NetBoxClient {
    core: NetBoxClientCore,
}

impl NetBoxClient {
    /// Create a new NetBox client
    ///
    /// # Arguments
    /// * `base_url` - NetBox base URL (e.g., "http://netbox:80")
    /// * `token` - API token for authentication
    pub fn new(base_url: String, token: String) -> Result<Self, NetBoxError> {
        let core = NetBoxClientCore::new(base_url, token)?;
        Ok(Self { core })
    }
    
    /// Get the base URL
    pub fn base_url(&self) -> &str {
        self.core.base_url()
    }
    
    /// Validate the API token by making a simple authenticated request.
    pub async fn validate_token(&self) -> Result<(), NetBoxError> {
        self.core.validate_token().await
    }
}

// Implement NetBoxClientTrait for NetBoxClient
// Trait signatures match module functions exactly, so implementations are direct calls
#[async_trait::async_trait]
impl NetBoxClientTrait for NetBoxClient {
    fn base_url(&self) -> &str {
        self.core.base_url()
    }

    async fn validate_token(&self) -> Result<(), NetBoxError> {
        self.core.validate_token().await
    }

    // IPAM Operations - Direct delegations
    async fn get_prefix(&self, id: PrefixId) -> Result<Prefix, NetBoxError> {
        ipam::get_prefix(&self.core, id).await
    }

    async fn get_available_ips(&self, prefix_id: PrefixId, limit: Option<u32>) -> Result<Vec<AvailableIP>, NetBoxError> {
        ipam::get_available_ips(&self.core, prefix_id, limit).await
    }

    async fn allocate_ip(&self, prefix_id: PrefixId, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError> {
        ipam::allocate_ip(&self.core, prefix_id, request).await
    }

    async fn get_ip_address(&self, id: IpAddressId) -> Result<IPAddress, NetBoxError> {
        ipam::get_ip_address(&self.core, id).await
    }

    async fn query_ip_addresses(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<IPAddress>, NetBoxError> {
        ipam::query_ip_addresses(&self.core, filters, fetch_all).await
    }

    async fn query_prefixes(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Prefix>, NetBoxError> {
        ipam::query_prefixes(&self.core, filters, fetch_all).await
    }

    async fn create_ip_address(&self, address: &str, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError> {
        ipam::create_ip_address(&self.core, address, request).await
    }

    async fn update_ip_address(&self, id: IpAddressId, request: AllocateIPRequest) -> Result<IPAddress, NetBoxError> {
        ipam::update_ip_address(&self.core, id.into(), request).await
    }

    async fn delete_ip_address(&self, id: IpAddressId) -> Result<(), NetBoxError> {
        ipam::delete_ip_address(&self.core, id.into()).await
    }

    async fn create_prefix(&self, prefix: &str, description: Option<String>, site_id: Option<SiteId>, vlan_id: Option<VlanId>, status: Option<&str>, role_id: Option<RoleId>, tenant_id: Option<TenantId>, tags: Option<Vec<String>>) -> Result<Prefix, NetBoxError> {
        ipam::create_prefix(&self.core, prefix, description, site_id.map(|id| id.into()), vlan_id.map(|id| id.into()), status, role_id.map(|id| id.into()), tenant_id.map(|id| id.into()), tags).await
    }

    async fn update_prefix(&self, id: PrefixId, prefix: Option<&str>, description: Option<String>, status: Option<&str>, role: Option<String>, tenant_id: Option<TenantId>, site_id: Option<SiteId>, vlan_id: Option<VlanId>, tags: Option<Vec<String>>) -> Result<Prefix, NetBoxError> {
        ipam::update_prefix(&self.core, id, prefix, description, status, role, tenant_id, site_id, vlan_id, tags).await
    }

    async fn query_aggregates(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Aggregate>, NetBoxError> {
        ipam::query_aggregates(&self.core, filters, fetch_all).await
    }

    async fn get_aggregate(&self, id: AggregateId) -> Result<Aggregate, NetBoxError> {
        ipam::get_aggregate(&self.core, id).await
    }

    async fn create_aggregate(&self, prefix: &str, rir_id: Option<RirId>, date_allocated: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Aggregate, NetBoxError> {
        ipam::create_aggregate(&self.core, prefix, rir_id, date_allocated, description, comments).await
    }

    async fn query_rirs(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Rir>, NetBoxError> {
        ipam::query_rirs(&self.core, filters, fetch_all).await
    }

    async fn get_rir_by_name(&self, name: &str) -> Result<Option<Rir>, NetBoxError> {
        ipam::get_rir_by_name(&self.core, name).await
    }

    async fn create_rir(&self, name: &str, slug: Option<&str>, description: Option<String>, is_private: Option<bool>) -> Result<Rir, NetBoxError> {
        ipam::create_rir(&self.core, name, slug, description, is_private).await
    }

    async fn create_vlan(&self, vid: u16, name: &str, site_id: Option<SiteId>, group_id: Option<VlanGroupId>, tenant_id: Option<TenantId>, role_id: Option<RoleId>, status: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Vlan, NetBoxError> {
        ipam::create_vlan(&self.core, vid, name, site_id, group_id, tenant_id, role_id, status, description, comments).await
    }

    async fn update_vlan(&self, id: VlanId, vid: Option<u16>, name: Option<&str>, site_id: Option<SiteId>, group_id: Option<VlanGroupId>, tenant_id: Option<TenantId>, role_id: Option<RoleId>, status: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Vlan, NetBoxError> {
        ipam::update_vlan(&self.core, <VlanId as Into<u32>>::into(id) as u64, vid, name, site_id.map(|id| id.into()), group_id.map(|id| id.into()), tenant_id.map(|id| id.into()), role_id.map(|id| id.into()), status, description, comments).await
    }

    async fn query_vlans(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Vlan>, NetBoxError> {
        ipam::query_vlans(&self.core, filters, fetch_all).await
    }

    async fn get_vlan(&self, id: VlanId) -> Result<Vlan, NetBoxError> {
        ipam::get_vlan(&self.core, id).await
    }

    // DCIM Operations - Direct delegations
    async fn query_devices(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Device>, NetBoxError> {
        dcim::query_devices(&self.core, filters, fetch_all).await
    }

    async fn get_device(&self, id: DeviceId) -> Result<Device, NetBoxError> {
        dcim::get_device(&self.core, id).await
    }

    async fn get_device_by_mac(&self, mac: &str) -> Result<Option<Device>, NetBoxError> {
        dcim::get_device_by_mac(&self.core, mac).await
    }

    async fn create_device(&self, device_type_id: DeviceTypeId, device_role_id: DeviceRoleId, site_id: SiteId, name: Option<&str>, tenant_id: Option<TenantId>, platform_id: Option<PlatformId>, location_id: Option<LocationId>, serial: Option<&str>, asset_tag: Option<&str>, status: Option<&str>, primary_ip4_id: Option<IpAddressId>, primary_ip6_id: Option<IpAddressId>, description: Option<String>, comments: Option<String>) -> Result<Device, NetBoxError> {
        dcim::create_device(&self.core, device_type_id, device_role_id, site_id, name, tenant_id, platform_id, location_id, serial, asset_tag, status, primary_ip4_id, primary_ip6_id, description, comments).await
    }

    async fn update_device(&self, id: DeviceId, name: Option<&str>, tenant_id: Option<TenantId>, platform_id: Option<PlatformId>, location_id: Option<LocationId>, serial: Option<&str>, asset_tag: Option<&str>, status: Option<&str>, primary_ip4_id: Option<IpAddressId>, primary_ip6_id: Option<IpAddressId>, description: Option<String>, comments: Option<String>) -> Result<Device, NetBoxError> {
        dcim::update_device(&self.core, id.into(), name, tenant_id.map(|id| id.into()), platform_id.map(|id| id.into()), location_id.map(|id| id.into()), serial, asset_tag, status, primary_ip4_id.map(|id| id.into()), primary_ip6_id.map(|id| id.into()), description, comments).await
    }

    async fn query_interfaces(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Interface>, NetBoxError> {
        dcim::query_interfaces(&self.core, filters, fetch_all).await
    }

    async fn get_interface(&self, id: InterfaceId) -> Result<Interface, NetBoxError> {
        dcim::get_interface(&self.core, id).await
    }

    async fn create_interface(&self, device_id: DeviceId, name: &str, interface_type: &str, enabled: Option<bool>, mac_address: Option<&str>, mtu: Option<u16>, description: Option<String>) -> Result<Interface, NetBoxError> {
        dcim::create_interface(&self.core, device_id, name, interface_type, enabled, mac_address, mtu, description).await
    }

    async fn update_interface(&self, id: InterfaceId, name: Option<&str>, interface_type: Option<&str>, enabled: Option<bool>, mac_address: Option<&str>, mtu: Option<u16>, description: Option<String>) -> Result<Interface, NetBoxError> {
        dcim::update_interface(&self.core, id, name, interface_type, enabled, mac_address, mtu, description).await
    }

    async fn query_mac_addresses(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<MACAddress>, NetBoxError> {
        dcim::query_mac_addresses(&self.core, filters, fetch_all).await
    }

    async fn get_mac_address_by_address(&self, mac: &str) -> Result<Option<MACAddress>, NetBoxError> {
        dcim::get_mac_address_by_address(&self.core, mac).await
    }

    async fn create_mac_address(&self, mac_address: &str, assigned_object_type: &str, assigned_object_id: u64, description: Option<String>, comments: Option<String>) -> Result<MACAddress, NetBoxError> {
        dcim::create_mac_address(&self.core, mac_address, assigned_object_type, assigned_object_id, description, comments).await
    }

    async fn query_sites(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Site>, NetBoxError> {
        dcim::query_sites(&self.core, filters, fetch_all).await
    }

    async fn get_site(&self, id: SiteId) -> Result<Site, NetBoxError> {
        dcim::get_site(&self.core, id).await
    }

    async fn create_site(&self, name: &str, slug: Option<&str>, description: Option<String>, physical_address: Option<String>, shipping_address: Option<String>, latitude: Option<f64>, longitude: Option<f64>, tenant_id: Option<TenantId>, region_id: Option<RegionId>, site_group_id: Option<SiteGroupId>, status: Option<&str>, facility: Option<String>, time_zone: Option<String>, comments: Option<String>) -> Result<Site, NetBoxError> {
        dcim::create_site(&self.core, name, slug, description, physical_address, shipping_address, latitude, longitude, tenant_id.map(|id| id.into()), region_id.map(|id| id.into()), site_group_id.map(|id| id.into()), status, facility, time_zone, comments).await
    }

    async fn update_site(&self, id: SiteId, name: Option<&str>, slug: Option<&str>, description: Option<String>, physical_address: Option<String>, shipping_address: Option<String>, latitude: Option<f64>, longitude: Option<f64>, tenant_id: Option<TenantId>, region_id: Option<RegionId>, site_group_id: Option<SiteGroupId>, status: Option<&str>, facility: Option<String>, time_zone: Option<String>, comments: Option<String>) -> Result<Site, NetBoxError> {
        dcim::update_site(&self.core, id, name, slug, description, physical_address, shipping_address, latitude, longitude, tenant_id, region_id, site_group_id, status, facility, time_zone, comments).await
    }

    async fn query_regions(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Region>, NetBoxError> {
        dcim::query_regions(&self.core, filters, fetch_all).await
    }

    async fn get_region(&self, id: RegionId) -> Result<Region, NetBoxError> {
        dcim::get_region(&self.core, id).await
    }

    async fn get_region_by_name(&self, name: &str) -> Result<Option<Region>, NetBoxError> {
        dcim::get_region_by_name(&self.core, name).await
    }

    async fn create_region(&self, name: &str, slug: Option<&str>, parent_id: Option<RegionId>, description: Option<String>, comments: Option<String>) -> Result<Region, NetBoxError> {
        dcim::create_region(&self.core, name, slug, parent_id, description, comments).await
    }

    async fn query_site_groups(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<SiteGroup>, NetBoxError> {
        dcim::query_site_groups(&self.core, filters, fetch_all).await
    }

    async fn get_site_group(&self, id: SiteGroupId) -> Result<SiteGroup, NetBoxError> {
        dcim::get_site_group(&self.core, id).await
    }

    async fn get_site_group_by_name(&self, name: &str) -> Result<Option<SiteGroup>, NetBoxError> {
        dcim::get_site_group_by_name(&self.core, name).await
    }

    async fn create_site_group(&self, name: &str, slug: Option<&str>, parent_id: Option<SiteGroupId>, description: Option<String>, comments: Option<String>) -> Result<SiteGroup, NetBoxError> {
        dcim::create_site_group(&self.core, name, slug, parent_id, description, comments).await
    }

    async fn query_locations(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Location>, NetBoxError> {
        dcim::query_locations(&self.core, filters, fetch_all).await
    }

    async fn get_location(&self, id: LocationId) -> Result<Location, NetBoxError> {
        dcim::get_location(&self.core, id).await
    }

    async fn get_location_by_name(&self, site_id: SiteId, name: &str) -> Result<Option<Location>, NetBoxError> {
        dcim::get_location_by_name(&self.core, site_id, name).await
    }

    async fn create_location(&self, site_id: SiteId, name: &str, slug: Option<&str>, parent_id: Option<LocationId>, tenant_id: Option<TenantId>, facility: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Location, NetBoxError> {
        dcim::create_location(&self.core, site_id, name, slug, parent_id, tenant_id, facility, description, comments).await
    }

    async fn query_device_roles(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<DeviceRole>, NetBoxError> {
        dcim::query_device_roles(&self.core, filters, fetch_all).await
    }

    async fn get_device_role_by_name(&self, name: &str) -> Result<Option<DeviceRole>, NetBoxError> {
        dcim::get_device_role_by_name(&self.core, name).await
    }

    async fn create_device_role(&self, name: &str, slug: Option<&str>, color: Option<&str>, vm_role: Option<bool>, description: Option<String>, comments: Option<String>) -> Result<DeviceRole, NetBoxError> {
        dcim::create_device_role(&self.core, name, slug, color, vm_role, description, comments).await
    }

    async fn query_manufacturers(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Manufacturer>, NetBoxError> {
        dcim::query_manufacturers(&self.core, filters, fetch_all).await
    }

    async fn get_manufacturer_by_name(&self, name: &str) -> Result<Option<Manufacturer>, NetBoxError> {
        dcim::get_manufacturer_by_name(&self.core, name).await
    }

    async fn create_manufacturer(&self, name: &str, slug: Option<&str>, description: Option<String>) -> Result<Manufacturer, NetBoxError> {
        dcim::create_manufacturer(&self.core, name, slug, description).await
    }

    async fn query_platforms(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Platform>, NetBoxError> {
        dcim::query_platforms(&self.core, filters, fetch_all).await
    }

    async fn get_platform_by_name(&self, name: &str) -> Result<Option<Platform>, NetBoxError> {
        dcim::get_platform_by_name(&self.core, name).await
    }

    async fn create_platform(&self, name: &str, slug: Option<&str>, manufacturer_id: Option<ManufacturerId>, napalm_driver: Option<&str>, napalm_args: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Platform, NetBoxError> {
        dcim::create_platform(&self.core, name, slug, manufacturer_id, napalm_driver, napalm_args, description, comments).await
    }

    async fn query_device_types(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<DeviceType>, NetBoxError> {
        dcim::query_device_types(&self.core, filters, fetch_all).await
    }

    async fn get_device_type_by_model(&self, manufacturer_id: ManufacturerId, model: &str) -> Result<Option<DeviceType>, NetBoxError> {
        dcim::get_device_type_by_model(&self.core, manufacturer_id, model).await
    }

    async fn create_device_type(&self, manufacturer_id: ManufacturerId, model: &str, slug: Option<&str>, part_number: Option<&str>, u_height: Option<f64>, is_full_depth: Option<bool>, description: Option<String>, comments: Option<String>) -> Result<DeviceType, NetBoxError> {
        dcim::create_device_type(&self.core, manufacturer_id, model, slug, part_number, u_height, is_full_depth, description, comments).await
    }

    // Tenancy Operations - Direct delegations
    async fn query_tenants(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Tenant>, NetBoxError> {
        tenancy::query_tenants(&self.core, filters, fetch_all).await
    }

    async fn get_tenant(&self, id: TenantId) -> Result<Tenant, NetBoxError> {
        tenancy::get_tenant(&self.core, id.into()).await
    }

    async fn create_tenant(&self, name: &str, slug: Option<&str>, description: Option<String>, comments: Option<String>, group: Option<TenantGroupId>) -> Result<Tenant, NetBoxError> {
        tenancy::create_tenant(&self.core, name, slug, description, comments, group.map(|id| id.into())).await
    }

    async fn query_tenant_groups(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<TenantGroup>, NetBoxError> {
        tenancy::query_tenant_groups(&self.core, filters, fetch_all).await
    }

    async fn get_tenant_group_by_name(&self, name: &str) -> Result<Option<TenantGroup>, NetBoxError> {
        tenancy::get_tenant_group_by_name(&self.core, name).await
    }

    async fn create_tenant_group(&self, name: &str, slug: Option<&str>, description: Option<String>, comments: Option<String>, parent_id: Option<TenantGroupId>) -> Result<TenantGroup, NetBoxError> {
        tenancy::create_tenant_group(&self.core, name, slug, description, comments, parent_id).await
    }

    // Extras Operations - Direct delegations
    async fn query_roles(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Role>, NetBoxError> {
        extras::query_roles(&self.core, filters, fetch_all).await
    }

    async fn get_role(&self, id: RoleId) -> Result<Role, NetBoxError> {
        extras::get_role(&self.core, id.into()).await
    }

    async fn create_role(&self, name: &str, slug: Option<&str>, description: Option<String>, weight: Option<u16>, comments: Option<String>) -> Result<Role, NetBoxError> {
        extras::create_role(&self.core, name, slug, description, weight, comments).await
    }

    async fn query_tags(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Tag>, NetBoxError> {
        extras::query_tags(&self.core, filters, fetch_all).await
    }

    async fn get_tag(&self, id: u64) -> Result<Tag, NetBoxError> {
        extras::get_tag(&self.core, id).await
    }

    async fn create_tag(&self, name: &str, slug: Option<&str>, color: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Tag, NetBoxError> {
        extras::create_tag(&self.core, name, slug, color, description, comments).await
    }
}
