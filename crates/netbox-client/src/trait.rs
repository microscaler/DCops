//! NetBoxClient trait for mocking
//!
//! This trait abstracts the NetBoxClient to enable mocking in unit tests.
//! The concrete NetBoxClient implements this trait, and tests can use mock implementations.
//!
//! NOTE: Trait signatures match module function signatures exactly to eliminate parameter mapping.

use crate::error::NetBoxError;
use crate::models::*;

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
    async fn get_prefix(&self, id: u64) -> Result<Prefix, NetBoxError>;
    async fn get_available_ips(&self, prefix_id: u64, limit: Option<u32>) -> Result<Vec<AvailableIP>, NetBoxError>;
    async fn allocate_ip(&self, prefix_id: u64, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError>;
    async fn get_ip_address(&self, id: u64) -> Result<IPAddress, NetBoxError>;
    async fn query_ip_addresses(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<IPAddress>, NetBoxError>;
    async fn query_prefixes(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Prefix>, NetBoxError>;
    async fn create_ip_address(&self, address: &str, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError>;
    async fn update_ip_address(&self, id: u64, request: AllocateIPRequest) -> Result<IPAddress, NetBoxError>;
    async fn delete_ip_address(&self, id: u64) -> Result<(), NetBoxError>;
    
    // Prefix operations - signatures match ipam::prefix module exactly
    async fn create_prefix(&self, prefix: &str, description: Option<String>, site_id: Option<u64>, vlan_id: Option<u32>, status: Option<&str>, role_id: Option<u64>, tenant_id: Option<u64>, tags: Option<Vec<String>>) -> Result<Prefix, NetBoxError>;
    async fn update_prefix(&self, id: u64, prefix: Option<&str>, description: Option<String>, status: Option<&str>, role: Option<String>, tenant_id: Option<u64>, site_id: Option<u64>, vlan_id: Option<u32>, tags: Option<Vec<String>>) -> Result<Prefix, NetBoxError>;
    
    async fn query_aggregates(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Aggregate>, NetBoxError>;
    async fn get_aggregate(&self, id: u64) -> Result<Aggregate, NetBoxError>;
    async fn create_aggregate(&self, prefix: &str, rir_id: Option<u64>, date_allocated: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Aggregate, NetBoxError>;
    
    async fn query_rirs(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Rir>, NetBoxError>;
    async fn get_rir_by_name(&self, name: &str) -> Result<Option<Rir>, NetBoxError>;
    async fn create_rir(&self, name: &str, slug: Option<&str>, description: Option<String>, is_private: Option<bool>) -> Result<Rir, NetBoxError>;
    
    async fn create_vlan(&self, vid: u16, name: &str, site_id: Option<u64>, group_id: Option<u64>, tenant_id: Option<u64>, role_id: Option<u64>, status: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Vlan, NetBoxError>;
    async fn update_vlan(&self, id: u64, vid: Option<u16>, name: Option<&str>, site_id: Option<u64>, group_id: Option<u64>, tenant_id: Option<u64>, role_id: Option<u64>, status: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Vlan, NetBoxError>;
    async fn query_vlans(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Vlan>, NetBoxError>;
    async fn get_vlan(&self, id: u64) -> Result<Vlan, NetBoxError>;

    // DCIM Operations
    async fn query_devices(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Device>, NetBoxError>;
    async fn get_device(&self, id: u64) -> Result<Device, NetBoxError>;
    async fn get_device_by_mac(&self, mac: &str) -> Result<Option<Device>, NetBoxError>;
    
    // Device operations - signatures match dcim::device module exactly
    async fn create_device(&self, device_type_id: u64, device_role_id: u64, site_id: u64, name: Option<&str>, tenant_id: Option<u64>, platform_id: Option<u64>, location_id: Option<u64>, serial: Option<&str>, asset_tag: Option<&str>, status: Option<&str>, primary_ip4_id: Option<u64>, primary_ip6_id: Option<u64>, description: Option<String>, comments: Option<String>) -> Result<Device, NetBoxError>;
    async fn update_device(&self, id: u64, name: Option<&str>, tenant_id: Option<u64>, platform_id: Option<u64>, location_id: Option<u64>, serial: Option<&str>, asset_tag: Option<&str>, status: Option<&str>, primary_ip4_id: Option<u64>, primary_ip6_id: Option<u64>, description: Option<String>, comments: Option<String>) -> Result<Device, NetBoxError>;
    
    async fn query_interfaces(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Interface>, NetBoxError>;
    async fn get_interface(&self, id: u64) -> Result<Interface, NetBoxError>;
    async fn create_interface(&self, device_id: u64, name: &str, interface_type: &str, enabled: Option<bool>, mac_address: Option<&str>, mtu: Option<u16>, description: Option<String>) -> Result<Interface, NetBoxError>;
    async fn update_interface(&self, id: u64, name: Option<&str>, interface_type: Option<&str>, enabled: Option<bool>, mac_address: Option<&str>, mtu: Option<u16>, description: Option<String>) -> Result<Interface, NetBoxError>;
    
    async fn query_mac_addresses(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<MACAddress>, NetBoxError>;
    async fn get_mac_address_by_address(&self, mac: &str) -> Result<Option<MACAddress>, NetBoxError>;
    async fn create_mac_address(&self, mac_address: &str, assigned_object_type: &str, assigned_object_id: u64, description: Option<String>, comments: Option<String>) -> Result<MACAddress, NetBoxError>;
    
    async fn query_sites(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Site>, NetBoxError>;
    async fn get_site(&self, id: u64) -> Result<Site, NetBoxError>;
    async fn create_site(&self, name: &str, slug: Option<&str>, description: Option<String>, physical_address: Option<String>, shipping_address: Option<String>, latitude: Option<f64>, longitude: Option<f64>, tenant_id: Option<u64>, region_id: Option<u64>, site_group_id: Option<u64>, status: Option<&str>, facility: Option<String>, time_zone: Option<String>, comments: Option<String>) -> Result<Site, NetBoxError>;
    async fn update_site(&self, id: u64, name: Option<&str>, slug: Option<&str>, description: Option<String>, physical_address: Option<String>, shipping_address: Option<String>, latitude: Option<f64>, longitude: Option<f64>, tenant_id: Option<u64>, region_id: Option<u64>, site_group_id: Option<u64>, status: Option<&str>, facility: Option<String>, time_zone: Option<String>, comments: Option<String>) -> Result<Site, NetBoxError>;
    
    async fn query_regions(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Region>, NetBoxError>;
    async fn get_region(&self, id: u64) -> Result<Region, NetBoxError>;
    async fn get_region_by_name(&self, name: &str) -> Result<Option<Region>, NetBoxError>;
    async fn create_region(&self, name: &str, slug: Option<&str>, parent_id: Option<u64>, description: Option<String>, comments: Option<String>) -> Result<Region, NetBoxError>;
    
    async fn query_site_groups(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<SiteGroup>, NetBoxError>;
    async fn get_site_group(&self, id: u64) -> Result<SiteGroup, NetBoxError>;
    async fn get_site_group_by_name(&self, name: &str) -> Result<Option<SiteGroup>, NetBoxError>;
    async fn create_site_group(&self, name: &str, slug: Option<&str>, parent_id: Option<u64>, description: Option<String>, comments: Option<String>) -> Result<SiteGroup, NetBoxError>;
    
    async fn query_locations(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Location>, NetBoxError>;
    async fn get_location(&self, id: u64) -> Result<Location, NetBoxError>;
    async fn get_location_by_name(&self, site_id: u64, name: &str) -> Result<Option<Location>, NetBoxError>;
    async fn create_location(&self, site_id: u64, name: &str, slug: Option<&str>, parent_id: Option<u64>, tenant_id: Option<u64>, facility: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Location, NetBoxError>;
    
    async fn query_device_roles(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<DeviceRole>, NetBoxError>;
    async fn get_device_role_by_name(&self, name: &str) -> Result<Option<DeviceRole>, NetBoxError>;
    async fn create_device_role(&self, name: &str, slug: Option<&str>, color: Option<&str>, vm_role: Option<bool>, description: Option<String>, comments: Option<String>) -> Result<DeviceRole, NetBoxError>;
    
    async fn query_manufacturers(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Manufacturer>, NetBoxError>;
    async fn get_manufacturer_by_name(&self, name: &str) -> Result<Option<Manufacturer>, NetBoxError>;
    async fn create_manufacturer(&self, name: &str, slug: Option<&str>, description: Option<String>) -> Result<Manufacturer, NetBoxError>;
    
    async fn query_platforms(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Platform>, NetBoxError>;
    async fn get_platform_by_name(&self, name: &str) -> Result<Option<Platform>, NetBoxError>;
    async fn create_platform(&self, name: &str, slug: Option<&str>, manufacturer_id: Option<u64>, napalm_driver: Option<&str>, napalm_args: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Platform, NetBoxError>;
    
    async fn query_device_types(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<DeviceType>, NetBoxError>;
    async fn get_device_type_by_model(&self, manufacturer_id: u64, model: &str) -> Result<Option<DeviceType>, NetBoxError>;
    async fn create_device_type(&self, manufacturer_id: u64, model: &str, slug: Option<&str>, part_number: Option<&str>, u_height: Option<f64>, is_full_depth: Option<bool>, description: Option<String>, comments: Option<String>) -> Result<DeviceType, NetBoxError>;

    // Tenancy Operations
    async fn query_tenants(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Tenant>, NetBoxError>;
    async fn get_tenant(&self, id: u64) -> Result<Tenant, NetBoxError>;
    async fn create_tenant(&self, name: &str, slug: Option<&str>, description: Option<String>, comments: Option<String>, group: Option<u64>) -> Result<Tenant, NetBoxError>;
    
    async fn query_tenant_groups(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<TenantGroup>, NetBoxError>;
    async fn get_tenant_group_by_name(&self, name: &str) -> Result<Option<TenantGroup>, NetBoxError>;
    async fn create_tenant_group(&self, name: &str, slug: Option<&str>, description: Option<String>, comments: Option<String>, parent_id: Option<u64>) -> Result<TenantGroup, NetBoxError>;

    // Extras Operations
    async fn query_roles(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Role>, NetBoxError>;
    async fn get_role(&self, id: u64) -> Result<Role, NetBoxError>;
    async fn create_role(&self, name: &str, slug: Option<&str>, description: Option<String>, weight: Option<u16>, comments: Option<String>) -> Result<Role, NetBoxError>;
    
    async fn query_tags(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Tag>, NetBoxError>;
    async fn get_tag(&self, id: u64) -> Result<Tag, NetBoxError>;
    async fn create_tag(&self, name: &str, slug: Option<&str>, color: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Tag, NetBoxError>;
}
