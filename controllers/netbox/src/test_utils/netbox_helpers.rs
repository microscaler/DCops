//! Helper functions for NetBox container and API testing
//!
//! This module provides utilities for:
//! - Starting NetBox containers for testing
//! - Setting up test data (tenant, prefix, IP range)
//! - Creating IP addresses via NetBox API
//! - Verifying IP addresses in NetBox

use super::docker_test_container::DockerTestContainer;
use super::docker_helpers::{create_container_with_ports, PortMapping, wait_for_health_check};
use bollard::Docker;
use netbox_client::{AllocateIPRequest, IPAddressStatus, NetBoxClientTrait, NetBoxClient};
use netbox_client::types::TenantId;
use ipnet::IpNet;
use tracing::{debug, info};

/// NetBox test configuration
#[derive(Debug, Clone)]
pub struct NetBoxTestConfig {
    pub base_url: String,
    pub api_token: String,
    pub tenant_id: Option<u64>,
    pub prefix_id: Option<u64>,
    pub ip_range_id: Option<u64>,
}

impl NetBoxTestConfig {
    /// Create a new NetBox test configuration
    pub fn new(base_url: String, api_token: String) -> Self {
        Self {
            base_url,
            api_token,
            tenant_id: None,
            prefix_id: None,
            ip_range_id: None,
        }
    }

    /// Create a NetBox client from this configuration
    pub fn client(&self) -> Result<NetBoxClient, Box<dyn std::error::Error>> {
        Ok(NetBoxClient::new(self.base_url.clone(), self.api_token.clone())?)
    }
}

/// Start a NetBox container for testing
///
/// # Arguments
///
/// * `docker` - Docker client instance
/// * `image` - NetBox Docker image (default: "netboxcommunity/netbox:latest")
/// * `api_port` - Port for NetBox API (default: 8001, mapped to container port 8001)
///
/// # Returns
///
/// Returns a `DockerTestContainer` wrapper and a `NetBoxTestConfig` with connection details
///
/// # Note
///
/// NetBox requires PostgreSQL and Redis. For full integration tests, use docker-compose
/// or a pre-configured NetBox stack. This function starts a basic NetBox container.
pub async fn start_netbox_container(
    docker: &Docker,
    image: Option<&str>,
    api_port: Option<u16>,
) -> Result<(DockerTestContainer, NetBoxTestConfig), Box<dyn std::error::Error>> {
    let image = image.unwrap_or("netboxcommunity/netbox:latest");
    let api_port = api_port.unwrap_or(8001);

    info!("Starting NetBox container: {}", image);

    // Create container with port mappings
    let ports = vec![
        PortMapping::new(8001, Some(api_port)), // NetBox API port
    ];

    let container = create_container_with_ports(docker, image, ports, None).await?;
    container.start().await?;

    // Wait for NetBox API to be ready
    let health_url = format!("http://localhost:{}/api/", api_port);
    wait_for_health_check(&container, &health_url, Some(60), Some(2000)).await?;

    info!("NetBox API is ready at {}", health_url);

    // Create test configuration
    // Note: In a real scenario, you'd need to:
    // 1. Create a superuser account
    // 2. Generate an API token
    // 3. Use that token for authentication
    // For now, we'll use a default token (this would need to be configured)
    let config = NetBoxTestConfig::new(
        format!("http://localhost:{}", api_port),
        "test-token".to_string(), // This would need to be generated/configured
    );

    Ok((container, config))
}

/// Set up test data in NetBox (tenant, prefix, IP range)
///
/// # Arguments
///
/// * `client` - NetBox client instance
/// * `api_token` - API token (needed for config)
/// * `tenant_name` - Name for the test tenant
/// * `prefix_cidr` - CIDR notation for the test prefix (e.g., "192.168.1.0/24")
/// * `ip_range_start` - Start IP for the IP range (e.g., "192.168.1.100")
/// * `ip_range_end` - End IP for the IP range (e.g., "192.168.1.200")
/// * `ip_range_status` - IP range status: Active, Reserved, or Deprecated (default: Active)
/// * `vrf_id` - Optional VRF ID to associate with the IP range
/// * `role_id` - Optional Role ID to associate with the IP range
///
/// # Returns
///
/// Returns a `NetBoxTestConfig` with the created resource IDs
///
/// # IP Range Status Options
/// - `Active`: Range is active and available for use
/// - `Reserved`: Range is reserved for future use
/// - `Deprecated`: Range is deprecated and should not be used
pub async fn setup_netbox_test_data(
    client: &dyn NetBoxClientTrait,
    api_token: &str,
    tenant_name: &str,
    prefix_cidr: &str,
    ip_range_start: &str,
    ip_range_end: &str,
    ip_range_status: Option<netbox_client::models::IPRangeStatus>,
    vrf_id: Option<u64>,
    role_id: Option<netbox_client::types::RoleId>,
) -> Result<NetBoxTestConfig, Box<dyn std::error::Error>> {
    info!("Setting up NetBox test data: tenant={}, prefix={}", tenant_name, prefix_cidr);

    // Create tenant
    let tenant = client.create_tenant(
        tenant_name,
        None, // slug
        Some(format!("Test tenant for DHCP testing")),
        None, // comments
        None, // group
        None, // tags
    ).await?;

    info!("Created tenant: {} (ID: {})", tenant.name, tenant.id);

    // Create prefix
    let prefix_net: IpNet = prefix_cidr.parse()?;
    let prefix = client.create_prefix(
        &prefix_net,
        Some(format!("Test prefix for DHCP testing")), // description
        None, // site_id
        None, // vlan_id
        Some("active"), // status
        None, // role_id
        Some(TenantId::from(tenant.id)), // tenant_id
        None, // tags
    ).await?;

    info!("Created prefix: {} (ID: {})", prefix_cidr, prefix.id);

    // Create IP range
    // IP ranges need full CIDR notation, so we'll extract just the IP part
    // and use the prefix's network for the CIDR
    let start_ip = ip_range_start.parse::<std::net::IpAddr>()?;
    let end_ip = ip_range_end.parse::<std::net::IpAddr>()?;
    
    // Create IPNet from IP addresses (we'll use the prefix's network)
    // For IP ranges, NetBox expects the start and end addresses as separate IPs
    // We'll create them with the same prefix length as the parent prefix
    let prefix_len = prefix_net.prefix_len();
    let start_net: IpNet = format!("{}/{}", start_ip, prefix_len).parse()?;
    let end_net: IpNet = format!("{}/{}", end_ip, prefix_len).parse()?;
    
    let ip_range = client.create_ip_range(
        &start_net,
        &end_net,
        vrf_id, // vrf_id (optional)
        Some(TenantId::from(tenant.id)),
        role_id, // role_id (optional)
        ip_range_status.or(Some(netbox_client::models::IPRangeStatus::Active)), // status: Active, Reserved, or Deprecated
        Some(format!("DHCP pool for testing")), // description
        None, // mark_utilized
        None, // mark_populated
        None, // tags
    ).await?;

    info!("Created IP range: {}-{} (ID: {})", ip_range_start, ip_range_end, ip_range.id);

    // Create config with resource IDs
    let mut config = NetBoxTestConfig::new(
        client.base_url().to_string(),
        api_token.to_string(),
    );
    config.tenant_id = Some(tenant.id);
    config.prefix_id = Some(prefix.id);
    config.ip_range_id = Some(ip_range.id);

    Ok(config)
}

/// Create an IP address in NetBox after DHCP allocation
///
/// # Arguments
///
/// * `client` - NetBox client instance (trait object for mocking support)
/// * `ip_address` - IP address in CIDR notation (e.g., "192.168.1.100/24")
/// * `tenant_id` - Tenant ID to associate with the IP
/// * `status` - IP address status (default: "dhcp")
/// * `mac_address` - Optional MAC address for static reservation
/// * `description` - Optional description
///
/// # Returns
///
/// Returns the created `IPAddress` from NetBox
pub async fn create_netbox_ip_address(
    client: &dyn NetBoxClientTrait,
    ip_address: &str,
    tenant_id: Option<TenantId>,
    status: Option<IPAddressStatus>,
    _mac_address: Option<&str>,
    description: Option<&str>,
) -> Result<netbox_client::models::IPAddress, Box<dyn std::error::Error>> {
    info!("Creating NetBox IP address: {} (tenant: {:?}, status: {:?})", 
          ip_address, tenant_id, status);

    let ip_net: IpNet = ip_address.parse()?;
    let status = status.unwrap_or(IPAddressStatus::Dhcp);

    let request = AllocateIPRequest {
        address: Some(ip_net),
        description: description.map(|s| s.to_string()),
        status: Some(status),
        role: None,
        dns_name: None,
        tags: None,
        tenant: tenant_id.map(|id| id.0),
        assigned_object_type: None,
        assigned_object_id: None,
    };

    // If MAC address is provided, we'd need to find/create an interface
    // For now, we'll just create the IP address
    // TODO: Add interface assignment if MAC address is provided

    let ip = client.create_ip_address(&ip_net, Some(request)).await?;

    info!("Created NetBox IP address: {} (ID: {})", ip_address, ip.id);

    Ok(ip)
}

/// Verify an IP address exists in NetBox
///
/// # Arguments
///
/// * `client` - NetBox client instance (trait object for mocking support)
/// * `ip_address` - IP address to verify (can be partial, e.g., "192.168.1.100")
///
/// # Returns
///
/// Returns `Some(IPAddress)` if found, or `None` if not found
pub async fn verify_ip_in_netbox(
    client: &dyn NetBoxClientTrait,
    ip_address: &str,
) -> Result<Option<netbox_client::models::IPAddress>, Box<dyn std::error::Error>> {
    debug!("Verifying IP address in NetBox: {}", ip_address);

    // Query IP addresses by address
    let ips: Vec<netbox_client::models::IPAddress> = client.query_ip_addresses(&[("address", ip_address)], false).await?;

    if ips.is_empty() {
        debug!("IP address {} not found in NetBox", ip_address);
        Ok(None)
    } else {
        debug!("Found IP address {} in NetBox (ID: {})", ip_address, ips[0].id);
        Ok(Some(ips[0].clone()))
    }
}

/// Verify IP address has correct status and associations
///
/// # Arguments
///
/// * `ip` - IP address from NetBox
/// * `expected_status` - Expected status value
/// * `expected_tenant_id` - Optional expected tenant ID
///
/// # Returns
///
/// Returns `Ok(())` if verification passes, or an error with details
pub fn verify_ip_status(
    ip: &netbox_client::models::IPAddress,
    expected_status: &str,
    expected_tenant_id: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check status - IPAddress.status is an enum, convert to string for comparison
    let status_str = match &ip.status {
        netbox_client::IPAddressStatus::Active => "active",
        netbox_client::IPAddressStatus::Reserved => "reserved",
        netbox_client::IPAddressStatus::Deprecated => "deprecated",
        netbox_client::IPAddressStatus::Dhcp => "dhcp",
        netbox_client::IPAddressStatus::Slaac => "slaac",
    };
    
    if status_str != expected_status {
        return Err(format!(
            "IP address {} has status '{}', expected '{}'",
            ip.address, status_str, expected_status
        ).into());
    }

    // Check tenant
    if let Some(expected_id) = expected_tenant_id {
        if let Some(tenant) = &ip.tenant {
            if tenant.id != expected_id {
                return Err(format!(
                    "IP address {} has tenant ID {}, expected {}",
                    ip.address, tenant.id, expected_id
                ).into());
            }
        } else {
            return Err(format!("IP address {} has no tenant", ip.address).into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use netbox_client::models::{IPAddress, IPAddressStatus};

    #[test]
    fn test_netbox_test_config() {
        let config = NetBoxTestConfig::new(
            "http://localhost:8001".to_string(),
            "test-token".to_string(),
        );
        assert_eq!(config.base_url, "http://localhost:8001");
        assert_eq!(config.api_token, "test-token");
    }

    #[test]
    fn test_netbox_test_config_client() {
        let config = NetBoxTestConfig::new(
            "http://localhost:8001".to_string(),
            "test-token".to_string(),
        );
        let client = config.client().unwrap();
        assert_eq!(client.base_url(), "http://localhost:8001");
    }

    fn create_test_ip_address(id: u64, address: &str, status: IPAddressStatus, tenant_id: Option<u64>) -> IPAddress {
        use ipnet::IpNet;
        use std::str::FromStr;
        
        IPAddress {
            id,
            url: format!("http://netbox/api/ipam/ip-addresses/{}/", id),
            display: address.to_string(),
            family: 4,
            address: IpNet::from_str(address).unwrap(),
            vrf: None,
            tenant: tenant_id.map(|tid| netbox_client::NestedTenant {
                id: tid,
                url: format!("http://netbox/api/tenancy/tenants/{}/", tid),
                display: format!("tenant-{}", tid),
                name: format!("tenant-{}", tid),
                slug: format!("tenant-{}", tid),
            }),
            status,
            role: None,
            assigned_object_type: None,
            assigned_object_id: None,
            assigned_object: None,
            nat_inside: None,
            nat_outside: vec![],
            dns_name: String::new(),
            description: String::new(),
            comments: String::new(),
            tags: vec![],
            custom_fields: serde_json::json!({}),
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_verify_ip_status_success() {
        let ip = create_test_ip_address(1, "192.168.1.100/24", IPAddressStatus::Dhcp, Some(10));

        // Test with matching status and tenant
        assert!(verify_ip_status(&ip, "dhcp", Some(10)).is_ok());
        
        // Test with matching status but no tenant check
        assert!(verify_ip_status(&ip, "dhcp", None).is_ok());
    }

    #[test]
    fn test_verify_ip_status_status_mismatch() {
        let ip = create_test_ip_address(1, "192.168.1.100/24", IPAddressStatus::Active, Some(10));

        let result = verify_ip_status(&ip, "dhcp", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("status 'active', expected 'dhcp'"));
    }

    #[test]
    fn test_verify_ip_status_tenant_mismatch() {
        let ip = create_test_ip_address(1, "192.168.1.100/24", IPAddressStatus::Dhcp, Some(10));

        let result = verify_ip_status(&ip, "dhcp", Some(20));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("tenant ID 10, expected 20"));
    }

    #[test]
    fn test_verify_ip_status_missing_tenant() {
        let ip = create_test_ip_address(1, "192.168.1.100/24", IPAddressStatus::Dhcp, None);

        let result = verify_ip_status(&ip, "dhcp", Some(10));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("has no tenant"));
    }

    #[tokio::test]
    async fn test_verify_ip_in_netbox_with_mock() {
        use netbox_client::MockNetBoxClient;

        let mock_client = MockNetBoxClient::new("http://netbox".to_string());
        
        // Create a test IP address in the mock (not used, but kept for reference)
        let _test_ip = create_test_ip_address(1, "192.168.1.100/24", IPAddressStatus::Dhcp, Some(10));

        // Add IP to mock client
        let ip_net: IpNet = "192.168.1.100/24".parse().unwrap();
        mock_client.create_ip_address(
            &ip_net,
            Some(netbox_client::AllocateIPRequest {
                address: Some(ip_net),
                description: Some("Test IP".to_string()),
                status: Some(netbox_client::IPAddressStatus::Dhcp),
                role: None,
                dns_name: None,
                tags: None,
                tenant: Some(10),
                assigned_object_type: None,
                assigned_object_id: None,
            }),
        ).await.unwrap();

        // Verify IP exists
        let result = verify_ip_in_netbox(&mock_client, "192.168.1.100").await.unwrap();
        assert!(result.is_some());
        let found_ip = result.unwrap();
        assert_eq!(found_ip.address.to_string(), "192.168.1.100/24");
    }

    #[tokio::test]
    async fn test_verify_ip_in_netbox_not_found() {
        use netbox_client::MockNetBoxClient;

        let mock_client = MockNetBoxClient::new("http://netbox".to_string());

        // Verify IP doesn't exist
        let result = verify_ip_in_netbox(&mock_client, "192.168.1.999").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_create_netbox_ip_address_with_mock() {
        use netbox_client::MockNetBoxClient;
        use netbox_client::types::TenantId;

        let mock_client = MockNetBoxClient::new("http://netbox".to_string());

        // Create tenant first
        let tenant = mock_client.create_tenant(
            "test-tenant",
            None,
            None,
            None,
            None,
            None,
        ).await.unwrap();

        // Create IP address
        let ip = create_netbox_ip_address(
            &mock_client,
            "192.168.1.100/24",
            Some(TenantId::from(tenant.id)),
            Some(netbox_client::IPAddressStatus::Dhcp),
            None,
            Some("Test DHCP IP"),
        ).await.unwrap();

        assert_eq!(ip.address.to_string(), "192.168.1.100/24");
        // IPAddress.status is an enum, not Option
        match ip.status {
            netbox_client::IPAddressStatus::Dhcp => {}
            _ => panic!("Expected status to be Dhcp"),
        }
        assert!(ip.tenant.is_some());
        assert_eq!(ip.tenant.as_ref().unwrap().id, tenant.id);
    }
}

