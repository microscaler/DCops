//! NetBoxClient trait for mocking
//!
//! This trait abstracts the NetBoxClient to enable mocking in unit tests.
//! The concrete NetBoxClient implements this trait, and tests can use mock implementations.
//!
//! NOTE: Trait signatures match module function signatures exactly to eliminate parameter mapping.

use crate::error::NetBoxError;
use crate::models::*;
use crate::types::*;
use ipnet::IpNet;

/// Trait for NetBox API client operations
///
/// This trait enables mocking of NetBox API calls for unit testing.
/// All async methods must be `Send` to work with Tokio's work-stealing runtime.
/// 
/// Trait signatures match module function signatures exactly to maximize DRY.
#[async_trait::async_trait]
pub trait NetBoxClientTrait: Send + Sync {
    /// Get the base URL
    fn base_url(&self) -> &str;

    /// Validate the API token
    async fn validate_token(&self) -> Result<(), NetBoxError>;

    // IPAM Operations
    async fn get_prefix(&self, id: PrefixId) -> Result<Prefix, NetBoxError>;
    async fn get_available_ips(&self, prefix_id: PrefixId, limit: Option<u32>) -> Result<Vec<AvailableIP>, NetBoxError>;
    async fn allocate_ip(&self, prefix_id: PrefixId, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError>;
    async fn get_ip_address(&self, id: IpAddressId) -> Result<IPAddress, NetBoxError>;
    async fn query_ip_addresses(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<IPAddress>, NetBoxError>;
    async fn query_prefixes(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Prefix>, NetBoxError>;
    async fn create_ip_address(&self, address: &IpNet, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError>;
    async fn update_ip_address(&self, id: IpAddressId, request: AllocateIPRequest) -> Result<IPAddress, NetBoxError>;
    async fn delete_ip_address(&self, id: IpAddressId) -> Result<(), NetBoxError>;
    
    // IP Range operations - signatures match ipam::ip_range module exactly
    async fn get_ip_range(&self, id: IPRangeId) -> Result<IPRange, NetBoxError>;
    async fn query_ip_ranges(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<IPRange>, NetBoxError>;
    async fn create_ip_range(&self, start_address: &IpNet, end_address: &IpNet, vrf_id: Option<u64>, tenant_id: Option<TenantId>, role_id: Option<RoleId>, status: Option<IPRangeStatus>, description: Option<String>, comments: Option<String>, mark_utilized: Option<bool>, mark_populated: Option<bool>, tags: Option<Vec<String>>) -> Result<IPRange, NetBoxError>;
    async fn update_ip_range(&self, id: IPRangeId, start_address: Option<&IpNet>, end_address: Option<&IpNet>, vrf_id: Option<u64>, tenant_id: Option<TenantId>, role_id: Option<RoleId>, status: Option<IPRangeStatus>, description: Option<String>, comments: Option<String>, mark_utilized: Option<bool>, mark_populated: Option<bool>, tags: Option<Vec<String>>) -> Result<IPRange, NetBoxError>;
    async fn delete_ip_range(&self, id: IPRangeId) -> Result<(), NetBoxError>;
    
    // Prefix operations - signatures match ipam::prefix module exactly
    async fn create_prefix(&self, prefix: &IpNet, description: Option<String>, site_id: Option<SiteId>, vlan_id: Option<VlanId>, status: Option<&str>, role_id: Option<RoleId>, tenant_id: Option<TenantId>, tags: Option<Vec<String>>) -> Result<Prefix, NetBoxError>;
    async fn update_prefix(&self, id: PrefixId, prefix: Option<&IpNet>, description: Option<String>, status: Option<&str>, role: Option<String>, tenant_id: Option<TenantId>, site_id: Option<SiteId>, vlan_id: Option<VlanId>, tags: Option<Vec<String>>) -> Result<Prefix, NetBoxError>;
    
    async fn query_aggregates(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Aggregate>, NetBoxError>;
    async fn get_aggregate(&self, id: AggregateId) -> Result<Aggregate, NetBoxError>;
    async fn create_aggregate(&self, prefix: &IpNet, rir_id: Option<RirId>, date_allocated: Option<&str>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Aggregate, NetBoxError>;
    async fn update_aggregate(&self, id: AggregateId, rir_id: Option<RirId>, date_allocated: Option<&str>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Aggregate, NetBoxError>;
    
    async fn query_rirs(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Rir>, NetBoxError>;
    async fn get_rir_by_name(&self, name: &str) -> Result<Option<Rir>, NetBoxError>;
    async fn create_rir(&self, name: &str, slug: Option<&str>, description: Option<String>, comments: Option<String>, is_private: Option<bool>, tags: Option<Vec<String>>) -> Result<Rir, NetBoxError>;
    async fn update_rir(&self, id: RirId, name: Option<&str>, slug: Option<&str>, description: Option<String>, comments: Option<String>, is_private: Option<bool>, tags: Option<Vec<String>>) -> Result<Rir, NetBoxError>;
    
    async fn create_vlan(&self, vid: u16, name: &str, site_id: Option<SiteId>, group_id: Option<VlanGroupId>, tenant_id: Option<TenantId>, role_id: Option<RoleId>, status: Option<&str>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Vlan, NetBoxError>;
    async fn update_vlan(&self, id: VlanId, vid: Option<u16>, name: Option<&str>, site_id: Option<SiteId>, group_id: Option<VlanGroupId>, tenant_id: Option<TenantId>, role_id: Option<RoleId>, status: Option<&str>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Vlan, NetBoxError>;
    async fn query_vlans(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Vlan>, NetBoxError>;
    async fn get_vlan(&self, id: VlanId) -> Result<Vlan, NetBoxError>;
    
    // VRF operations
    async fn query_vrfs(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Vrf>, NetBoxError>;
    async fn get_vrf(&self, id: VrfId) -> Result<Vrf, NetBoxError>;
    async fn get_vrf_by_name(&self, name: &str) -> Result<Option<Vrf>, NetBoxError>;
    async fn create_vrf(&self, name: &str, rd: Option<&str>, enforce_unique: Option<bool>, tenant_id: Option<TenantId>, description: Option<String>, comments: Option<String>, import_targets: Option<Vec<RouteTargetId>>, export_targets: Option<Vec<RouteTargetId>>, tags: Option<Vec<String>>) -> Result<Vrf, NetBoxError>;
    async fn update_vrf(&self, id: VrfId, name: Option<&str>, rd: Option<&str>, enforce_unique: Option<bool>, tenant_id: Option<TenantId>, description: Option<String>, comments: Option<String>, import_targets: Option<Vec<RouteTargetId>>, export_targets: Option<Vec<RouteTargetId>>, tags: Option<Vec<String>>) -> Result<Vrf, NetBoxError>;
    async fn delete_vrf(&self, id: VrfId) -> Result<(), NetBoxError>;
    
    // Route Target operations
    async fn query_route_targets(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<RouteTarget>, NetBoxError>;
    async fn get_route_target(&self, id: RouteTargetId) -> Result<RouteTarget, NetBoxError>;
    async fn get_route_target_by_name(&self, name: &str) -> Result<Option<RouteTarget>, NetBoxError>;
    async fn create_route_target(&self, name: &str, tenant_id: Option<TenantId>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<RouteTarget, NetBoxError>;
    async fn update_route_target(&self, id: RouteTargetId, name: Option<&str>, tenant_id: Option<TenantId>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<RouteTarget, NetBoxError>;
    async fn delete_route_target(&self, id: RouteTargetId) -> Result<(), NetBoxError>;

    // DCIM Operations
    async fn query_devices(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Device>, NetBoxError>;
    async fn get_device(&self, id: DeviceId) -> Result<Device, NetBoxError>;
    async fn get_device_by_mac(&self, mac: &str) -> Result<Option<Device>, NetBoxError>;
    
    // Device operations - signatures match dcim::device module exactly
    async fn create_device(&self, device_type_id: DeviceTypeId, device_role_id: DeviceRoleId, site_id: SiteId, name: Option<&str>, tenant_id: Option<TenantId>, platform_id: Option<PlatformId>, location_id: Option<LocationId>, serial: Option<&str>, asset_tag: Option<&str>, status: Option<&str>, primary_ip4_id: Option<IpAddressId>, primary_ip6_id: Option<IpAddressId>, description: Option<String>, comments: Option<String>) -> Result<Device, NetBoxError>;
    async fn update_device(&self, id: DeviceId, name: Option<&str>, tenant_id: Option<TenantId>, platform_id: Option<PlatformId>, location_id: Option<LocationId>, serial: Option<&str>, asset_tag: Option<&str>, status: Option<&str>, primary_ip4_id: Option<IpAddressId>, primary_ip6_id: Option<IpAddressId>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Device, NetBoxError>;
    
    async fn query_interfaces(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Interface>, NetBoxError>;
    async fn get_interface(&self, id: InterfaceId) -> Result<Interface, NetBoxError>;
    async fn create_interface(&self, device_id: DeviceId, name: &str, interface_type: &str, enabled: Option<bool>, mac_address: Option<&str>, mtu: Option<u16>, description: Option<String>, comments: Option<String>) -> Result<Interface, NetBoxError>;
    async fn update_interface(&self, id: InterfaceId, name: Option<&str>, interface_type: Option<&str>, enabled: Option<bool>, mac_address: Option<&str>, mtu: Option<u16>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Interface, NetBoxError>;
    
    async fn query_mac_addresses(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<MACAddress>, NetBoxError>;
    async fn get_mac_address_by_address(&self, mac: &str) -> Result<Option<MACAddress>, NetBoxError>;
    async fn create_mac_address(&self, mac_address: &str, assigned_object_type: &str, assigned_object_id: u64, description: Option<String>, comments: Option<String>) -> Result<MACAddress, NetBoxError>;
    async fn update_mac_address(&self, id: u64, assigned_object_type: Option<&str>, assigned_object_id: Option<u64>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<MACAddress, NetBoxError>;
    
    async fn query_sites(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Site>, NetBoxError>;
    async fn get_site(&self, id: SiteId) -> Result<Site, NetBoxError>;
    async fn create_site(&self, name: &str, slug: Option<&str>, description: Option<String>, physical_address: Option<String>, shipping_address: Option<String>, latitude: Option<f64>, longitude: Option<f64>, tenant_id: Option<TenantId>, region_id: Option<RegionId>, site_group_id: Option<SiteGroupId>, status: Option<&str>, facility: Option<String>, time_zone: Option<String>, comments: Option<String>) -> Result<Site, NetBoxError>;
    async fn update_site(&self, id: SiteId, name: Option<&str>, slug: Option<&str>, description: Option<String>, physical_address: Option<String>, shipping_address: Option<String>, latitude: Option<f64>, longitude: Option<f64>, tenant_id: Option<TenantId>, region_id: Option<RegionId>, site_group_id: Option<SiteGroupId>, status: Option<&str>, facility: Option<String>, time_zone: Option<String>, comments: Option<String>) -> Result<Site, NetBoxError>;
    
    async fn query_regions(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Region>, NetBoxError>;
    async fn get_region(&self, id: RegionId) -> Result<Region, NetBoxError>;
    async fn get_region_by_name(&self, name: &str) -> Result<Option<Region>, NetBoxError>;
    async fn create_region(&self, name: &str, slug: Option<&str>, parent_id: Option<RegionId>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Region, NetBoxError>;
    async fn update_region(&self, id: RegionId, name: Option<&str>, slug: Option<&str>, parent_id: Option<RegionId>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Region, NetBoxError>;
    
    async fn query_site_groups(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<SiteGroup>, NetBoxError>;
    async fn get_site_group(&self, id: SiteGroupId) -> Result<SiteGroup, NetBoxError>;
    async fn get_site_group_by_name(&self, name: &str) -> Result<Option<SiteGroup>, NetBoxError>;
    async fn create_site_group(&self, name: &str, slug: Option<&str>, parent_id: Option<SiteGroupId>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<SiteGroup, NetBoxError>;
    async fn update_site_group(&self, id: SiteGroupId, name: Option<&str>, slug: Option<&str>, parent_id: Option<SiteGroupId>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<SiteGroup, NetBoxError>;
    
    async fn query_locations(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Location>, NetBoxError>;
    async fn get_location(&self, id: LocationId) -> Result<Location, NetBoxError>;
    async fn get_location_by_name(&self, site_id: SiteId, name: &str) -> Result<Option<Location>, NetBoxError>;
    async fn create_location(&self, site_id: SiteId, name: &str, slug: Option<&str>, parent_id: Option<LocationId>, tenant_id: Option<TenantId>, facility: Option<&str>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Location, NetBoxError>;
    async fn update_location(&self, id: LocationId, name: Option<&str>, slug: Option<&str>, parent_id: Option<LocationId>, tenant_id: Option<TenantId>, facility: Option<&str>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Location, NetBoxError>;
    
    async fn query_device_roles(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<DeviceRole>, NetBoxError>;
    async fn get_device_role_by_name(&self, name: &str) -> Result<Option<DeviceRole>, NetBoxError>;
    async fn create_device_role(&self, name: &str, slug: Option<&str>, color: Option<&str>, vm_role: Option<bool>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<DeviceRole, NetBoxError>;
    async fn update_device_role(&self, id: DeviceRoleId, name: Option<&str>, slug: Option<&str>, color: Option<&str>, vm_role: Option<bool>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<DeviceRole, NetBoxError>;
    
    async fn query_manufacturers(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Manufacturer>, NetBoxError>;
    async fn get_manufacturer_by_name(&self, name: &str) -> Result<Option<Manufacturer>, NetBoxError>;
    async fn create_manufacturer(&self, name: &str, slug: Option<&str>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Manufacturer, NetBoxError>;
    async fn update_manufacturer(&self, id: ManufacturerId, name: Option<&str>, slug: Option<&str>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Manufacturer, NetBoxError>;
    
    async fn query_platforms(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Platform>, NetBoxError>;
    async fn get_platform_by_name(&self, name: &str) -> Result<Option<Platform>, NetBoxError>;
    async fn create_platform(&self, name: &str, slug: Option<&str>, manufacturer_id: Option<ManufacturerId>, napalm_driver: Option<&str>, napalm_args: Option<&str>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Platform, NetBoxError>;
    async fn update_platform(&self, id: PlatformId, name: Option<&str>, slug: Option<&str>, manufacturer_id: Option<ManufacturerId>, napalm_driver: Option<&str>, napalm_args: Option<&str>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Platform, NetBoxError>;
    
    async fn query_device_types(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<DeviceType>, NetBoxError>;
    async fn get_device_type_by_model(&self, manufacturer_id: ManufacturerId, model: &str) -> Result<Option<DeviceType>, NetBoxError>;
    async fn create_device_type(&self, manufacturer_id: ManufacturerId, model: &str, slug: Option<&str>, part_number: Option<&str>, u_height: Option<f64>, is_full_depth: Option<bool>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<DeviceType, NetBoxError>;
    async fn update_device_type(&self, id: DeviceTypeId, manufacturer_id: Option<ManufacturerId>, model: Option<&str>, slug: Option<&str>, part_number: Option<&str>, u_height: Option<f64>, is_full_depth: Option<bool>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<DeviceType, NetBoxError>;

    // Tenancy Operations
    async fn query_tenants(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Tenant>, NetBoxError>;
    async fn get_tenant(&self, id: TenantId) -> Result<Tenant, NetBoxError>;
    async fn create_tenant(&self, name: &str, slug: Option<&str>, description: Option<String>, comments: Option<String>, group: Option<TenantGroupId>, tags: Option<Vec<String>>) -> Result<Tenant, NetBoxError>;
    async fn update_tenant(&self, id: TenantId, name: Option<&str>, slug: Option<&str>, description: Option<String>, comments: Option<String>, group: Option<TenantGroupId>, tags: Option<Vec<String>>) -> Result<Tenant, NetBoxError>;
    
    async fn query_tenant_groups(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<TenantGroup>, NetBoxError>;
    async fn get_tenant_group(&self, id: TenantGroupId) -> Result<TenantGroup, NetBoxError>;
    async fn get_tenant_group_by_name(&self, name: &str) -> Result<Option<TenantGroup>, NetBoxError>;
    async fn create_tenant_group(&self, name: &str, slug: Option<&str>, description: Option<String>, comments: Option<String>, parent_id: Option<TenantGroupId>, tags: Option<Vec<String>>) -> Result<TenantGroup, NetBoxError>;
    async fn update_tenant_group(&self, id: TenantGroupId, name: Option<&str>, slug: Option<&str>, description: Option<String>, comments: Option<String>, parent_id: Option<TenantGroupId>, tags: Option<Vec<String>>) -> Result<TenantGroup, NetBoxError>;

    // Extras Operations
    async fn query_roles(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Role>, NetBoxError>;
    async fn get_role(&self, id: RoleId) -> Result<Role, NetBoxError>;
    async fn create_role(&self, name: &str, slug: Option<&str>, description: Option<String>, weight: Option<u16>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Role, NetBoxError>;
    async fn update_role(&self, id: RoleId, name: Option<&str>, slug: Option<&str>, description: Option<String>, weight: Option<u16>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Role, NetBoxError>;
    
    async fn query_tags(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Tag>, NetBoxError>;
    async fn get_tag(&self, id: u64) -> Result<Tag, NetBoxError>;
    async fn create_tag(&self, name: &str, slug: Option<&str>, color: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Tag, NetBoxError>;
    async fn update_tag(&self, id: u64, name: Option<&str>, slug: Option<&str>, color: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Tag, NetBoxError>;
}
