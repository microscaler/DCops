//! Mock TokenResolver for unit testing
//!
//! This module provides a mock implementation of TokenResolver that doesn't require
//! a real kube::Client. It stores secrets in memory and returns them when requested.

#[cfg(test)]
use crate::token_resolver::{TokenResolverTrait, TokenResolutionError};
#[cfg(test)]
use crds::NetBoxResourceReference;
#[cfg(test)]
use kube::Client;
#[cfg(test)]
use netbox_client::NetBoxClientTrait;
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::{Arc, Mutex};
#[cfg(test)]
use k8s_openapi::api::core::v1::Secret;
#[cfg(test)]
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
#[cfg(test)]
use std::collections::BTreeMap;
#[cfg(test)]
use kube::Error as KubeError;
#[cfg(test)]
use kube::api::Api;

/// Mock Api<Secret> for testing
///
/// This implements the Secret API operations needed by the tenant reconciler.
#[cfg(test)]
pub struct MockSecretApi {
    namespace: String,
    secrets: Arc<Mutex<HashMap<String, String>>>, // namespace/secret_name -> token
}

#[cfg(test)]
impl MockSecretApi {
    /// Get a secret by name
    pub async fn get(&self, name: &str) -> Result<Secret, KubeError> {
        let key = format!("{}/{}", self.namespace, name);
        let secrets = self.secrets.lock().unwrap();
        secrets.get(&key).map(|token| {
            // Create a Secret object from the stored token
            let mut data = BTreeMap::new();
            let token_bytes = token.as_bytes().to_vec();
            data.insert("token".to_string(), k8s_openapi::ByteString(token_bytes));
            
            Secret {
                metadata: ObjectMeta {
                    name: Some(name.to_string()),
                    namespace: Some(self.namespace.clone()),
                    ..Default::default()
                },
                data: Some(data),
                ..Default::default()
            }
        }).ok_or_else(|| {
            KubeError::Api(kube::error::ErrorResponse {
                code: 404,
                message: format!("Secret {} not found in namespace {}", name, self.namespace),
                reason: "NotFound".to_string(),
                status: "Failure".to_string(),
            })
        })
    }
}

/// Wrapper to make Arc<MockNetBoxClient> work as Box<dyn NetBoxClientTrait>
#[cfg(test)]
struct MockNetBoxClientWrapper {
    client: Arc<netbox_client::MockNetBoxClient>,
}

#[async_trait::async_trait]
#[cfg(test)]
impl NetBoxClientTrait for MockNetBoxClientWrapper {
    fn base_url(&self) -> &str {
        self.client.base_url()
    }

    async fn validate_token(&self) -> Result<(), netbox_client::NetBoxError> {
        self.client.validate_token().await
    }

    async fn get_prefix(&self, id: netbox_client::PrefixId) -> Result<netbox_client::Prefix, netbox_client::NetBoxError> {
        self.client.get_prefix(id).await
    }

    async fn get_available_ips(&self, prefix_id: netbox_client::PrefixId, limit: Option<u32>) -> Result<Vec<netbox_client::AvailableIP>, netbox_client::NetBoxError> {
        self.client.get_available_ips(prefix_id, limit).await
    }

    async fn allocate_ip(&self, prefix_id: netbox_client::PrefixId, request: Option<netbox_client::AllocateIPRequest>) -> Result<netbox_client::IPAddress, netbox_client::NetBoxError> {
        self.client.allocate_ip(prefix_id, request).await
    }

    async fn get_ip_address(&self, id: netbox_client::IpAddressId) -> Result<netbox_client::IPAddress, netbox_client::NetBoxError> {
        self.client.get_ip_address(id).await
    }

    async fn query_ip_addresses(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::IPAddress>, netbox_client::NetBoxError> {
        self.client.query_ip_addresses(filters, fetch_all).await
    }

    async fn query_prefixes(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::Prefix>, netbox_client::NetBoxError> {
        self.client.query_prefixes(filters, fetch_all).await
    }

    async fn create_ip_address(&self, address: &ipnet::IpNet, request: Option<netbox_client::AllocateIPRequest>) -> Result<netbox_client::IPAddress, netbox_client::NetBoxError> {
        self.client.create_ip_address(address, request).await
    }

    async fn update_ip_address(&self, id: netbox_client::IpAddressId, request: netbox_client::AllocateIPRequest) -> Result<netbox_client::IPAddress, netbox_client::NetBoxError> {
        self.client.update_ip_address(id, request).await
    }

    async fn delete_ip_address(&self, id: netbox_client::IpAddressId) -> Result<(), netbox_client::NetBoxError> {
        self.client.delete_ip_address(id).await
    }

    async fn create_prefix(&self, prefix: &ipnet::IpNet, description: Option<String>, site_id: Option<netbox_client::SiteId>, vlan_id: Option<netbox_client::VlanId>, status: Option<&str>, role_id: Option<netbox_client::RoleId>, tenant_id: Option<netbox_client::TenantId>, tags: Option<Vec<String>>) -> Result<netbox_client::Prefix, netbox_client::NetBoxError> {
        self.client.create_prefix(prefix, description, site_id, vlan_id, status, role_id, tenant_id, tags).await
    }

    async fn update_prefix(&self, id: netbox_client::PrefixId, prefix: Option<&ipnet::IpNet>, description: Option<String>, status: Option<&str>, role: Option<String>, tenant_id: Option<netbox_client::TenantId>, site_id: Option<netbox_client::SiteId>, vlan_id: Option<netbox_client::VlanId>, tags: Option<Vec<String>>) -> Result<netbox_client::Prefix, netbox_client::NetBoxError> {
        self.client.update_prefix(id, prefix, description, status, role, tenant_id, site_id, vlan_id, tags).await
    }

    async fn query_aggregates(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::Aggregate>, netbox_client::NetBoxError> {
        self.client.query_aggregates(filters, fetch_all).await
    }

    async fn get_aggregate(&self, id: netbox_client::AggregateId) -> Result<netbox_client::Aggregate, netbox_client::NetBoxError> {
        self.client.get_aggregate(id).await
    }

    async fn create_aggregate(&self, prefix: &ipnet::IpNet, rir_id: Option<netbox_client::RirId>, date_allocated: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<netbox_client::Aggregate, netbox_client::NetBoxError> {
        self.client.create_aggregate(prefix, rir_id, date_allocated, description, comments).await
    }

    async fn query_rirs(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::Rir>, netbox_client::NetBoxError> {
        self.client.query_rirs(filters, fetch_all).await
    }

    async fn get_rir_by_name(&self, name: &str) -> Result<Option<netbox_client::Rir>, netbox_client::NetBoxError> {
        self.client.get_rir_by_name(name).await
    }

    async fn create_rir(&self, name: &str, slug: Option<&str>, description: Option<String>, is_private: Option<bool>) -> Result<netbox_client::Rir, netbox_client::NetBoxError> {
        self.client.create_rir(name, slug, description, is_private).await
    }

    async fn create_vlan(&self, vid: u16, name: &str, site_id: Option<netbox_client::SiteId>, group_id: Option<netbox_client::VlanGroupId>, tenant_id: Option<netbox_client::TenantId>, role_id: Option<netbox_client::RoleId>, status: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<netbox_client::Vlan, netbox_client::NetBoxError> {
        self.client.create_vlan(vid, name, site_id, group_id, tenant_id, role_id, status, description, comments).await
    }

    async fn get_vlan(&self, id: netbox_client::VlanId) -> Result<netbox_client::Vlan, netbox_client::NetBoxError> {
        self.client.get_vlan(id).await
    }

    async fn update_vlan(&self, id: netbox_client::VlanId, vid: Option<u16>, name: Option<&str>, site_id: Option<netbox_client::SiteId>, group_id: Option<netbox_client::VlanGroupId>, tenant_id: Option<netbox_client::TenantId>, role_id: Option<netbox_client::RoleId>, status: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<netbox_client::Vlan, netbox_client::NetBoxError> {
        self.client.update_vlan(id, vid, name, site_id, group_id, tenant_id, role_id, status, description, comments).await
    }

    async fn query_vlans(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::Vlan>, netbox_client::NetBoxError> {
        self.client.query_vlans(filters, fetch_all).await
    }

    async fn query_sites(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::Site>, netbox_client::NetBoxError> {
        self.client.query_sites(filters, fetch_all).await
    }

    async fn get_site(&self, id: netbox_client::SiteId) -> Result<netbox_client::Site, netbox_client::NetBoxError> {
        self.client.get_site(id).await
    }

    async fn create_site(&self, name: &str, slug: Option<&str>, description: Option<String>, physical_address: Option<String>, shipping_address: Option<String>, latitude: Option<f64>, longitude: Option<f64>, tenant_id: Option<netbox_client::TenantId>, region_id: Option<netbox_client::RegionId>, site_group_id: Option<netbox_client::SiteGroupId>, status: Option<&str>, facility: Option<String>, time_zone: Option<String>, comments: Option<String>) -> Result<netbox_client::Site, netbox_client::NetBoxError> {
        self.client.create_site(name, slug, description, physical_address, shipping_address, latitude, longitude, tenant_id, region_id, site_group_id, status, facility, time_zone, comments).await
    }

    async fn update_site(&self, id: netbox_client::SiteId, name: Option<&str>, slug: Option<&str>, description: Option<String>, physical_address: Option<String>, shipping_address: Option<String>, latitude: Option<f64>, longitude: Option<f64>, tenant_id: Option<netbox_client::TenantId>, region_id: Option<netbox_client::RegionId>, site_group_id: Option<netbox_client::SiteGroupId>, status: Option<&str>, facility: Option<String>, time_zone: Option<String>, comments: Option<String>) -> Result<netbox_client::Site, netbox_client::NetBoxError> {
        self.client.update_site(id, name, slug, description, physical_address, shipping_address, latitude, longitude, tenant_id, region_id, site_group_id, status, facility, time_zone, comments).await
    }

    async fn query_regions(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::Region>, netbox_client::NetBoxError> {
        self.client.query_regions(filters, fetch_all).await
    }

    async fn get_region(&self, id: netbox_client::RegionId) -> Result<netbox_client::Region, netbox_client::NetBoxError> {
        self.client.get_region(id).await
    }

    async fn get_region_by_name(&self, name: &str) -> Result<Option<netbox_client::Region>, netbox_client::NetBoxError> {
        self.client.get_region_by_name(name).await
    }

    async fn create_region(&self, name: &str, slug: Option<&str>, parent_id: Option<netbox_client::RegionId>, description: Option<String>, comments: Option<String>) -> Result<netbox_client::Region, netbox_client::NetBoxError> {
        self.client.create_region(name, slug, parent_id, description, comments).await
    }

    async fn query_site_groups(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::SiteGroup>, netbox_client::NetBoxError> {
        self.client.query_site_groups(filters, fetch_all).await
    }

    async fn get_site_group(&self, id: netbox_client::SiteGroupId) -> Result<netbox_client::SiteGroup, netbox_client::NetBoxError> {
        self.client.get_site_group(id).await
    }

    async fn get_site_group_by_name(&self, name: &str) -> Result<Option<netbox_client::SiteGroup>, netbox_client::NetBoxError> {
        self.client.get_site_group_by_name(name).await
    }

    async fn create_site_group(&self, name: &str, slug: Option<&str>, parent_id: Option<netbox_client::SiteGroupId>, description: Option<String>, comments: Option<String>) -> Result<netbox_client::SiteGroup, netbox_client::NetBoxError> {
        self.client.create_site_group(name, slug, parent_id, description, comments).await
    }

    async fn query_locations(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::Location>, netbox_client::NetBoxError> {
        self.client.query_locations(filters, fetch_all).await
    }

    async fn get_location(&self, id: netbox_client::LocationId) -> Result<netbox_client::Location, netbox_client::NetBoxError> {
        self.client.get_location(id).await
    }

    async fn get_location_by_name(&self, site_id: netbox_client::SiteId, name: &str) -> Result<Option<netbox_client::Location>, netbox_client::NetBoxError> {
        self.client.get_location_by_name(site_id, name).await
    }

    async fn create_location(&self, site_id: netbox_client::SiteId, name: &str, slug: Option<&str>, parent_id: Option<netbox_client::LocationId>, tenant_id: Option<netbox_client::TenantId>, facility: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<netbox_client::Location, netbox_client::NetBoxError> {
        self.client.create_location(site_id, name, slug, parent_id, tenant_id, facility, description, comments).await
    }

    async fn query_devices(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::Device>, netbox_client::NetBoxError> {
        self.client.query_devices(filters, fetch_all).await
    }

    async fn get_device(&self, id: netbox_client::DeviceId) -> Result<netbox_client::Device, netbox_client::NetBoxError> {
        self.client.get_device(id).await
    }

    async fn get_device_by_mac(&self, mac: &str) -> Result<Option<netbox_client::Device>, netbox_client::NetBoxError> {
        self.client.get_device_by_mac(mac).await
    }

    async fn create_device(&self, device_type_id: netbox_client::DeviceTypeId, device_role_id: netbox_client::DeviceRoleId, site_id: netbox_client::SiteId, name: Option<&str>, tenant_id: Option<netbox_client::TenantId>, platform_id: Option<netbox_client::PlatformId>, location_id: Option<netbox_client::LocationId>, serial: Option<&str>, asset_tag: Option<&str>, status: Option<&str>, primary_ip4_id: Option<netbox_client::IpAddressId>, primary_ip6_id: Option<netbox_client::IpAddressId>, description: Option<String>, comments: Option<String>) -> Result<netbox_client::Device, netbox_client::NetBoxError> {
        self.client.create_device(device_type_id, device_role_id, site_id, name, tenant_id, platform_id, location_id, serial, asset_tag, status, primary_ip4_id, primary_ip6_id, description, comments).await
    }

    async fn update_device(&self, id: netbox_client::DeviceId, name: Option<&str>, tenant_id: Option<netbox_client::TenantId>, platform_id: Option<netbox_client::PlatformId>, location_id: Option<netbox_client::LocationId>, serial: Option<&str>, asset_tag: Option<&str>, status: Option<&str>, primary_ip4_id: Option<netbox_client::IpAddressId>, primary_ip6_id: Option<netbox_client::IpAddressId>, description: Option<String>, comments: Option<String>) -> Result<netbox_client::Device, netbox_client::NetBoxError> {
        self.client.update_device(id, name, tenant_id, platform_id, location_id, serial, asset_tag, status, primary_ip4_id, primary_ip6_id, description, comments).await
    }

    async fn query_interfaces(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::Interface>, netbox_client::NetBoxError> {
        self.client.query_interfaces(filters, fetch_all).await
    }

    async fn get_interface(&self, id: netbox_client::InterfaceId) -> Result<netbox_client::Interface, netbox_client::NetBoxError> {
        self.client.get_interface(id).await
    }

    async fn create_interface(&self, device_id: netbox_client::DeviceId, name: &str, interface_type: &str, enabled: Option<bool>, mac_address: Option<&str>, mtu: Option<u16>, description: Option<String>) -> Result<netbox_client::Interface, netbox_client::NetBoxError> {
        self.client.create_interface(device_id, name, interface_type, enabled, mac_address, mtu, description).await
    }

    async fn update_interface(&self, id: netbox_client::InterfaceId, name: Option<&str>, interface_type: Option<&str>, enabled: Option<bool>, mac_address: Option<&str>, mtu: Option<u16>, description: Option<String>) -> Result<netbox_client::Interface, netbox_client::NetBoxError> {
        self.client.update_interface(id, name, interface_type, enabled, mac_address, mtu, description).await
    }

    async fn query_mac_addresses(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::MACAddress>, netbox_client::NetBoxError> {
        self.client.query_mac_addresses(filters, fetch_all).await
    }

    async fn get_mac_address_by_address(&self, mac: &str) -> Result<Option<netbox_client::MACAddress>, netbox_client::NetBoxError> {
        self.client.get_mac_address_by_address(mac).await
    }

    async fn create_mac_address(&self, mac_address: &str, assigned_object_type: &str, assigned_object_id: u64, description: Option<String>, comments: Option<String>) -> Result<netbox_client::MACAddress, netbox_client::NetBoxError> {
        self.client.create_mac_address(mac_address, assigned_object_type, assigned_object_id, description, comments).await
    }

    async fn query_device_roles(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::DeviceRole>, netbox_client::NetBoxError> {
        self.client.query_device_roles(filters, fetch_all).await
    }

    async fn get_device_role_by_name(&self, name: &str) -> Result<Option<netbox_client::DeviceRole>, netbox_client::NetBoxError> {
        self.client.get_device_role_by_name(name).await
    }

    async fn create_device_role(&self, name: &str, slug: Option<&str>, color: Option<&str>, vm_role: Option<bool>, description: Option<String>, comments: Option<String>) -> Result<netbox_client::DeviceRole, netbox_client::NetBoxError> {
        self.client.create_device_role(name, slug, color, vm_role, description, comments).await
    }

    async fn query_manufacturers(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::Manufacturer>, netbox_client::NetBoxError> {
        self.client.query_manufacturers(filters, fetch_all).await
    }

    async fn get_manufacturer_by_name(&self, name: &str) -> Result<Option<netbox_client::Manufacturer>, netbox_client::NetBoxError> {
        self.client.get_manufacturer_by_name(name).await
    }

    async fn create_manufacturer(&self, name: &str, slug: Option<&str>, description: Option<String>) -> Result<netbox_client::Manufacturer, netbox_client::NetBoxError> {
        self.client.create_manufacturer(name, slug, description).await
    }

    async fn query_platforms(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::Platform>, netbox_client::NetBoxError> {
        self.client.query_platforms(filters, fetch_all).await
    }

    async fn get_platform_by_name(&self, name: &str) -> Result<Option<netbox_client::Platform>, netbox_client::NetBoxError> {
        self.client.get_platform_by_name(name).await
    }

    async fn create_platform(&self, name: &str, slug: Option<&str>, manufacturer_id: Option<netbox_client::ManufacturerId>, napalm_driver: Option<&str>, napalm_args: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<netbox_client::Platform, netbox_client::NetBoxError> {
        self.client.create_platform(name, slug, manufacturer_id, napalm_driver, napalm_args, description, comments).await
    }

    async fn query_device_types(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::DeviceType>, netbox_client::NetBoxError> {
        self.client.query_device_types(filters, fetch_all).await
    }

    async fn get_device_type_by_model(&self, manufacturer_id: netbox_client::ManufacturerId, model: &str) -> Result<Option<netbox_client::DeviceType>, netbox_client::NetBoxError> {
        self.client.get_device_type_by_model(manufacturer_id, model).await
    }

    async fn create_device_type(&self, manufacturer_id: netbox_client::ManufacturerId, model: &str, slug: Option<&str>, part_number: Option<&str>, u_height: Option<f64>, is_full_depth: Option<bool>, description: Option<String>, comments: Option<String>) -> Result<netbox_client::DeviceType, netbox_client::NetBoxError> {
        self.client.create_device_type(manufacturer_id, model, slug, part_number, u_height, is_full_depth, description, comments).await
    }

    async fn query_tenants(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::Tenant>, netbox_client::NetBoxError> {
        self.client.query_tenants(filters, fetch_all).await
    }

    async fn get_tenant(&self, id: netbox_client::TenantId) -> Result<netbox_client::Tenant, netbox_client::NetBoxError> {
        self.client.get_tenant(id).await
    }

    async fn create_tenant(&self, name: &str, slug: Option<&str>, description: Option<String>, comments: Option<String>, group: Option<netbox_client::TenantGroupId>) -> Result<netbox_client::Tenant, netbox_client::NetBoxError> {
        self.client.create_tenant(name, slug, description, comments, group).await
    }

    async fn update_tenant(&self, id: netbox_client::TenantId, name: Option<&str>, slug: Option<&str>, description: Option<String>, comments: Option<String>, group: Option<netbox_client::TenantGroupId>) -> Result<netbox_client::Tenant, netbox_client::NetBoxError> {
        self.client.update_tenant(id, name, slug, description, comments, group).await
    }

    async fn query_tenant_groups(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::TenantGroup>, netbox_client::NetBoxError> {
        self.client.query_tenant_groups(filters, fetch_all).await
    }

    async fn get_tenant_group_by_name(&self, name: &str) -> Result<Option<netbox_client::TenantGroup>, netbox_client::NetBoxError> {
        self.client.get_tenant_group_by_name(name).await
    }

    async fn create_tenant_group(&self, name: &str, slug: Option<&str>, description: Option<String>, comments: Option<String>, parent_id: Option<netbox_client::TenantGroupId>) -> Result<netbox_client::TenantGroup, netbox_client::NetBoxError> {
        self.client.create_tenant_group(name, slug, description, comments, parent_id).await
    }

    async fn query_roles(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::Role>, netbox_client::NetBoxError> {
        self.client.query_roles(filters, fetch_all).await
    }

    async fn get_role(&self, id: netbox_client::RoleId) -> Result<netbox_client::Role, netbox_client::NetBoxError> {
        self.client.get_role(id).await
    }

    async fn create_role(&self, name: &str, slug: Option<&str>, description: Option<String>, weight: Option<u16>, comments: Option<String>) -> Result<netbox_client::Role, netbox_client::NetBoxError> {
        self.client.create_role(name, slug, description, weight, comments).await
    }

    async fn query_tags(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<netbox_client::Tag>, netbox_client::NetBoxError> {
        self.client.query_tags(filters, fetch_all).await
    }

    async fn get_tag(&self, id: u64) -> Result<netbox_client::Tag, netbox_client::NetBoxError> {
        self.client.get_tag(id).await
    }

    async fn create_tag(&self, name: &str, slug: Option<&str>, color: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<netbox_client::Tag, netbox_client::NetBoxError> {
        self.client.create_tag(name, slug, color, description, comments).await
    }
}

/// Mock kube::Client for Secret API calls
///
/// This is a minimal mock that only supports Secret API operations.
/// It uses the in-memory secret storage from MockTokenResolver.
#[cfg(test)]
struct MockKubeClient {
    secrets: Arc<Mutex<HashMap<String, String>>>, // namespace/secret_name -> token
}

/// Mock TokenResolver for testing
///
/// This mock stores secrets in memory and returns them when requested.
/// It doesn't require a real kube::Client, making it suitable for unit tests.
#[cfg(test)]
pub struct MockTokenResolver {
    netbox_url: String,
    pub(crate) secrets: Arc<Mutex<HashMap<String, String>>>, // namespace/secret_name -> token
    mock_netbox_client: Arc<netbox_client::MockNetBoxClient>, // Shared MockNetBoxClient
    mock_kube_client: Arc<MockKubeClient>, // Mock kube::Client for Secret API
}

#[cfg(test)]
impl MockTokenResolver {
    /// Create a new mock TokenResolver
    pub fn new(netbox_url: String) -> Self {
        let secrets = Arc::new(Mutex::new(HashMap::new()));
        Self {
            netbox_url: netbox_url.clone(),
            secrets: secrets.clone(),
            mock_netbox_client: Arc::new(netbox_client::MockNetBoxClient::new(netbox_url)),
            mock_kube_client: Arc::new(MockKubeClient { secrets }),
        }
    }
    
    /// Get a secret from the mock storage
    ///
    /// This is used by the mock kube::Client to fetch secrets.
    pub fn get_secret(&self, namespace: &str, secret_name: &str) -> Option<Secret> {
        let key = format!("{}/{}", namespace, secret_name);
        let secrets = self.secrets.lock().unwrap();
        secrets.get(&key).map(|token| {
            // Create a Secret object from the stored token
            let mut data = BTreeMap::new();
            let token_bytes = token.as_bytes().to_vec();
            data.insert("token".to_string(), k8s_openapi::ByteString(token_bytes));
            
            Secret {
                metadata: ObjectMeta {
                    name: Some(secret_name.to_string()),
                    namespace: Some(namespace.to_string()),
                    ..Default::default()
                },
                data: Some(data),
                ..Default::default()
            }
        })
    }
    
    /// Get a reference to the underlying MockNetBoxClient
    /// 
    /// This allows tests to set up mock data (e.g., add_prefix, set_available_ips)
    pub fn mock_client(&self) -> Arc<netbox_client::MockNetBoxClient> {
        self.mock_netbox_client.clone()
    }
    
    /// Add a secret token to the mock
    ///
    /// # Arguments
    /// * `namespace` - Namespace where the secret exists
    /// * `secret_name` - Name of the secret
    /// * `token` - The NetBox API token
    pub fn add_secret(&self, namespace: &str, secret_name: &str, token: String) {
        let key = format!("{}/{}", namespace, secret_name);
        let mut secrets = self.secrets.lock().unwrap();
        secrets.insert(key, token);
    }
    
    /// Resolve token for a tenant reference (mock implementation)
    ///
    /// This simulates the token resolution process by looking up the secret
    /// in the in-memory store. The tenant_ref.name is used to construct the
    /// secret name (following the pattern: netbox-token-{tenant-name}).
    pub async fn resolve_token(
        &self,
        namespace: &str,
        tenant_ref: &NetBoxResourceReference,
    ) -> Result<String, TokenResolutionError> {
        // Simulate secret name pattern: netbox-token-{tenant-name}
        let secret_name = format!("netbox-token-{}", tenant_ref.name);
        let key = format!("{}/{}", namespace, secret_name);
        
        let secrets = self.secrets.lock().unwrap();
        secrets.get(&key)
            .cloned()
            .ok_or_else(|| {
                TokenResolutionError::SecretNotFound(format!(
                    "Secret {} not found in namespace {} (mock)",
                    secret_name, namespace
                ))
            })
    }
    
    /// Get the main tenant reference (datacenter-tenant)
    pub fn get_main_tenant_reference(&self) -> NetBoxResourceReference {
        NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTenant".to_string(),
            name: "datacenter-tenant".to_string(),
            namespace: None,
        }
    }
}

#[async_trait::async_trait]
#[cfg(test)]
impl TokenResolverTrait for MockTokenResolver {
    async fn create_client_for_tenant(
        &self,
        namespace: &str,
        tenant_ref: &NetBoxResourceReference,
    ) -> Result<Box<dyn netbox_client::NetBoxClientTrait>, TokenResolutionError> {
        // Verify token exists (for test validation)
        let _token = self.resolve_token(namespace, tenant_ref).await?;
        
        // Return the shared MockNetBoxClient wrapped in a Box
        // We clone the Arc to get a new reference to the same MockNetBoxClient
        Ok(Box::new(MockNetBoxClientWrapper {
            client: self.mock_netbox_client.clone(),
        }))
    }

    async fn create_client_for_shared_resource(
        &self,
        namespace: &str,
        _resource_kind: &str,
        _resource_name: &str,
    ) -> Result<Box<dyn netbox_client::NetBoxClientTrait>, TokenResolutionError> {
        // For shared resources, use main tenant
        let tenant_ref = self.get_main_tenant_reference();
        self.create_client_for_tenant(namespace, &tenant_ref).await
    }

    fn kube_client(&self) -> &Client {
        // Mock doesn't have a real kube client - this should not be called in tests
        // that use MockTokenResolver. The tenant reconciler should use SecretFetcher instead.
        panic!("MockTokenResolver::kube_client() called - not supported. Use SecretFetcher for tests that need secret fetching.");
    }

    fn netbox_url(&self) -> &str {
        &self.netbox_url
    }
    
    fn create_client_with_token(&self, _token: String) -> Result<Box<dyn netbox_client::NetBoxClientTrait>, TokenResolutionError> {
        // Return the shared MockNetBoxClient (token is already validated via SecretFetcher)
        Ok(Box::new(MockNetBoxClientWrapper {
            client: self.mock_netbox_client.clone(),
        }))
    }
}

/// Test APIs structure to hold unboxed MockKubeApi instances
/// 
/// This allows tests to store data in the APIs before they're boxed and passed to the reconciler.
#[cfg(test)]
pub struct TestReconcilerApis {
    pub prefix_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::NetBoxPrefix>>,
    pub tenant_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::NetBoxTenant>>,
    pub role_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::NetBoxRole>>,
    pub tag_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::NetBoxTag>>,
    pub aggregate_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::NetBoxAggregate>>,
    pub vlan_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::NetBoxVLAN>>,
    pub rir_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::NetBoxRIR>>,
    pub site_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::NetBoxSite>>,
    pub device_role_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::NetBoxDeviceRole>>,
    pub manufacturer_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::NetBoxManufacturer>>,
    pub platform_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::NetBoxPlatform>>,
    pub device_type_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::NetBoxDeviceType>>,
    pub device_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::NetBoxDevice>>,
    pub interface_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::NetBoxInterface>>,
    pub mac_address_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::NetBoxMACAddress>>,
    pub region_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::NetBoxRegion>>,
    pub site_group_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::NetBoxSiteGroup>>,
    pub location_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::NetBoxLocation>>,
    pub ip_pool_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::IPPool>>,
    pub ip_claim_api: std::sync::Arc<crate::kube_api_trait::mock::MockKubeApi<crds::IPClaim>>,
}

/// Helper to create a test reconciler with MockTokenResolver
///
/// This creates a Reconciler with a MockTokenResolver instead of a real TokenResolver,
/// allowing tests to run without a real kube::Client.
///
/// Returns both the Reconciler and the unboxed MockKubeApi instances so tests can
/// store data in them before reconciliation.
#[cfg(test)]
pub fn create_test_reconciler_with_mock_token_resolver(
    mock_token_resolver: Arc<MockTokenResolver>,
) -> (crate::reconciler::Reconciler, TestReconcilerApis) {
    use crate::kube_api_trait::mock::MockKubeApi;
    use crate::reconciler::Reconciler;
    use crds::*;
    use std::sync::Arc;
    
    // Create all the mock APIs as Arc so we can share them
    let prefix_api = Arc::new(MockKubeApi::<NetBoxPrefix>::new());
    let role_api = Arc::new(MockKubeApi::<NetBoxRole>::new());
    let tag_api = Arc::new(MockKubeApi::<NetBoxTag>::new());
    let aggregate_api = Arc::new(MockKubeApi::<NetBoxAggregate>::new());
    let vlan_api = Arc::new(MockKubeApi::<NetBoxVLAN>::new());
    let rir_api = Arc::new(MockKubeApi::<NetBoxRIR>::new());
    let tenant_api = Arc::new(MockKubeApi::<NetBoxTenant>::new());
    let site_api = Arc::new(MockKubeApi::<NetBoxSite>::new());
    let device_role_api = Arc::new(MockKubeApi::<NetBoxDeviceRole>::new());
    let manufacturer_api = Arc::new(MockKubeApi::<NetBoxManufacturer>::new());
    let platform_api = Arc::new(MockKubeApi::<NetBoxPlatform>::new());
    let device_type_api = Arc::new(MockKubeApi::<NetBoxDeviceType>::new());
    let device_api = Arc::new(MockKubeApi::<NetBoxDevice>::new());
    let interface_api = Arc::new(MockKubeApi::<NetBoxInterface>::new());
    let mac_address_api = Arc::new(MockKubeApi::<NetBoxMACAddress>::new());
    let region_api = Arc::new(MockKubeApi::<NetBoxRegion>::new());
    let site_group_api = Arc::new(MockKubeApi::<NetBoxSiteGroup>::new());
    let location_api = Arc::new(MockKubeApi::<NetBoxLocation>::new());
    let ip_pool_api = Arc::new(MockKubeApi::<IPPool>::new());
    let ip_claim_api = Arc::new(MockKubeApi::<IPClaim>::new());
    
    // Create MockSecretFetcher using the same secret storage
    use crate::secret_fetcher::mock::MockSecretFetcher;
    let secret_fetcher = Arc::new(MockSecretFetcher::new(mock_token_resolver.secrets.clone()));
    
    let reconciler = Reconciler::new(
        // Now that Reconciler uses TokenResolverTrait, we can use MockTokenResolver!
        mock_token_resolver as Arc<dyn TokenResolverTrait>,
        Some(secret_fetcher), // Use MockSecretFetcher for testing
        // IPAM APIs - clone Arc and pass directly (Reconciler::new boxes them internally)
        prefix_api.clone(),
        role_api.clone(),
        tag_api.clone(),
        aggregate_api.clone(),
        vlan_api.clone(),
        rir_api.clone(),
        // Tenancy APIs
        tenant_api.clone(),
        // DCIM APIs
        site_api.clone(),
        device_role_api.clone(),
        manufacturer_api.clone(),
        platform_api.clone(),
        device_type_api.clone(),
        device_api.clone(),
        interface_api.clone(),
        mac_address_api.clone(),
        region_api.clone(),
        site_group_api.clone(),
        location_api.clone(),
        // Custom CRDs
        ip_pool_api.clone(),
        ip_claim_api.clone(),
    );
    
    let apis = TestReconcilerApis {
        prefix_api,
        tenant_api,
        role_api,
        tag_api,
        aggregate_api,
        vlan_api,
        rir_api,
        site_api,
        device_role_api,
        manufacturer_api,
        platform_api,
        device_type_api,
        device_api,
        interface_api,
        mac_address_api,
        region_api,
        site_group_api,
        location_api,
        ip_pool_api,
        ip_claim_api,
    };
    
    (reconciler, apis)
}
