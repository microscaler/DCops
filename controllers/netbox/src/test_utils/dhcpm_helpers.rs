//! Helper functions for dhcpm CLI tool in test containers
//!
//! This module provides utilities for:
//! - Starting dhcpm test containers
//! - Executing DHCP DISCOVER/REQUEST (DORA sequence)
//! - Parsing IP addresses from dhcpm JSON output

use super::docker_test_container::DockerTestContainer;
use super::docker_helpers::{create_container_with_ports, PortMapping, exec_in_container};
use bollard::Docker;
use serde_json::Value;
use std::net::IpAddr;
use tracing::{debug, info, warn};

/// Result of a DHCP allocation request
#[derive(Debug, Clone)]
pub struct DhcpAllocationResult {
    pub ip_address: IpAddr,
    pub subnet_mask: Option<IpAddr>,
    pub gateway: Option<IpAddr>,
    pub dns_servers: Vec<IpAddr>,
    pub lease_time: Option<u32>,
}

/// Start a dhcpm test container
///
/// # Arguments
///
/// * `docker` - Docker client instance
/// * `image` - Docker image with dhcpm (default: build from Dockerfile.dhcpm-test)
///
/// # Returns
///
/// Returns a `DockerTestContainer` wrapper
///
/// # Note
///
/// For DHCP testing, containers should be on the same Docker network or use host networking.
/// This function creates a container that can be connected to a network later.
pub async fn start_dhcpm_container(
    docker: &Docker,
    image: Option<&str>,
) -> Result<DockerTestContainer, Box<dyn std::error::Error>> {
    let image = image.unwrap_or("dhcpm-test:latest");

    info!("Starting dhcpm test container: {}", image);

    // Create container (no port mappings needed - uses host network or shared network)
    let ports = vec![]; // dhcpm doesn't need exposed ports

    let container = create_container_with_ports(docker, image, ports, None).await?;
    container.start().await?;

    info!("dhcpm test container started");

    Ok(container)
}

/// Execute dhcpm DISCOVER/REQUEST (DORA sequence) and return JSON output
///
/// # Arguments
///
/// * `container` - dhcpm test container
/// * `interface` - Network interface to use (default: "eth0")
/// * `mac_address` - Optional MAC address to use (default: random)
/// * `server_ip` - Optional DHCP server IP (default: auto-discover)
///
/// # Returns
///
/// Returns the JSON output from dhcpm as a Value
pub async fn run_dhcpm_discover(
    container: &DockerTestContainer,
    interface: Option<&str>,
    mac_address: Option<&str>,
    server_ip: Option<&str>,
) -> Result<Value, Box<dyn std::error::Error>> {
    let interface = interface.unwrap_or("eth0");

    // Build dhcpm command
    let mut cmd = vec!["dhcpm", "--interface", interface, "--output", "json"];

    if let Some(mac) = mac_address {
        cmd.extend(&["--mac", mac]);
    }

    if let Some(server) = server_ip {
        cmd.extend(&["--server", server]);
    }

    info!("Executing dhcpm: {:?}", cmd);

    // Execute command in container
    let output = exec_in_container(container, &cmd).await?;

    // Parse JSON output
    let json: Value = serde_json::from_str(&output)
        .map_err(|e| format!("Failed to parse dhcpm JSON output: {} - Output: {}", e, output))?;

    debug!("dhcpm output: {}", serde_json::to_string_pretty(&json)?);

    Ok(json)
}

/// Parse IP address from dhcpm JSON output
///
/// # Arguments
///
/// * `json` - JSON output from dhcpm
///
/// # Returns
///
/// Returns a `DhcpAllocationResult` with parsed information
pub fn parse_dhcpm_output(json: &Value) -> Result<DhcpAllocationResult, Box<dyn std::error::Error>> {
    // dhcpm JSON structure (example):
    // {
    //   "yiaddr": "192.168.1.100",
    //   "subnet_mask": "255.255.255.0",
    //   "router": ["192.168.1.1"],
    //   "dns": ["8.8.8.8", "8.8.4.4"],
    //   "lease_time": 3600
    // }

    let ip_str = json
        .get("yiaddr")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'yiaddr' field in dhcpm output")?;

    let ip_address: IpAddr = ip_str
        .parse()
        .map_err(|e| format!("Invalid IP address '{}': {}", ip_str, e))?;

    let subnet_mask = json
        .get("subnet_mask")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<IpAddr>().ok());

    let gateway = json
        .get("router")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<IpAddr>().ok());

    let dns_servers: Vec<IpAddr> = json
        .get("dns")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().and_then(|s| s.parse::<IpAddr>().ok()))
                .collect()
        })
        .unwrap_or_default();

    let lease_time = json
        .get("lease_time")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    Ok(DhcpAllocationResult {
        ip_address,
        subnet_mask,
        gateway,
        dns_servers,
        lease_time,
    })
}

/// Verify IP address is within a CIDR range
///
/// # Arguments
///
/// * `ip` - IP address to check
/// * `cidr` - CIDR notation (e.g., "192.168.1.0/24")
///
/// # Returns
///
/// Returns `true` if IP is within the CIDR range
pub fn ip_in_cidr(ip: &IpAddr, cidr: &str) -> Result<bool, Box<dyn std::error::Error>> {
    use ipnet::IpNet;

    let net: IpNet = cidr.parse()?;
    Ok(net.contains(ip))
}

/// Verify IP address is within a pool range
///
/// # Arguments
///
/// * `ip` - IP address to check
/// * `pool_range` - Pool range notation (e.g., "192.168.1.100-192.168.1.200")
///
/// # Returns
///
/// Returns `true` if IP is within the pool range
pub fn ip_in_pool_range(ip: &IpAddr, pool_range: &str) -> Result<bool, Box<dyn std::error::Error>> {
    // Parse pool range (e.g., "192.168.1.100-192.168.1.200")
    let parts: Vec<&str> = pool_range.split('-').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid pool range format: {}", pool_range).into());
    }

    let start_ip: IpAddr = parts[0].trim().parse()?;
    let end_ip: IpAddr = parts[1].trim().parse()?;

    // Simple comparison (works for IPv4)
    match (ip, start_ip, end_ip) {
        (IpAddr::V4(ip), IpAddr::V4(start), IpAddr::V4(end)) => {
            let ip_u32 = u32::from(*ip);
            let start_u32 = u32::from(*start);
            let end_u32 = u32::from(*end);
            Ok(ip_u32 >= start_u32 && ip_u32 <= end_u32)
        }
        _ => Err("Pool range verification only supports IPv4".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dhcpm_output() {
        let json = serde_json::json!({
            "yiaddr": "192.168.1.100",
            "subnet_mask": "255.255.255.0",
            "router": ["192.168.1.1"],
            "dns": ["8.8.8.8", "8.8.4.4"],
            "lease_time": 3600
        });

        let result = parse_dhcpm_output(&json).unwrap();
        assert_eq!(result.ip_address.to_string(), "192.168.1.100");
        assert_eq!(result.gateway.unwrap().to_string(), "192.168.1.1");
        assert_eq!(result.dns_servers.len(), 2);
        assert_eq!(result.lease_time, Some(3600));
    }

    #[test]
    fn test_ip_in_pool_range() {
        let ip: IpAddr = "192.168.1.150".parse().unwrap();
        assert!(ip_in_pool_range(&ip, "192.168.1.100-192.168.1.200").unwrap());
        assert!(!ip_in_pool_range(&ip, "192.168.1.10-192.168.1.50").unwrap());
    }

    #[test]
    fn test_ip_in_cidr() {
        let ip: IpAddr = "192.168.1.100".parse().unwrap();
        assert!(ip_in_cidr(&ip, "192.168.1.0/24").unwrap());
        assert!(!ip_in_cidr(&ip, "10.0.0.0/8").unwrap());
    }

    #[test]
    fn test_ip_in_cidr_invalid_format() {
        let ip: IpAddr = "192.168.1.100".parse().unwrap();
        let result = ip_in_cidr(&ip, "invalid-cidr");
        assert!(result.is_err());
    }

    #[test]
    fn test_ip_in_cidr_ipv6() {
        let ip: IpAddr = "2001:db8::1".parse().unwrap();
        assert!(ip_in_cidr(&ip, "2001:db8::/32").unwrap());
        assert!(!ip_in_cidr(&ip, "2001:db9::/32").unwrap());
    }

    #[test]
    fn test_parse_dhcpm_output_missing_yiaddr() {
        let json = serde_json::json!({
            "subnet_mask": "255.255.255.0",
            "router": ["192.168.1.1"],
        });

        let result = parse_dhcpm_output(&json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'yiaddr'"));
    }

    #[test]
    fn test_parse_dhcpm_output_invalid_ip() {
        let json = serde_json::json!({
            "yiaddr": "invalid-ip",
        });

        let result = parse_dhcpm_output(&json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid IP address"));
    }

    #[test]
    fn test_parse_dhcpm_output_minimal() {
        // Test with only required field (yiaddr)
        let json = serde_json::json!({
            "yiaddr": "192.168.1.100",
        });

        let result = parse_dhcpm_output(&json).unwrap();
        assert_eq!(result.ip_address.to_string(), "192.168.1.100");
        assert!(result.subnet_mask.is_none());
        assert!(result.gateway.is_none());
        assert!(result.dns_servers.is_empty());
        assert!(result.lease_time.is_none());
    }

    #[test]
    fn test_parse_dhcpm_output_invalid_subnet_mask() {
        let json = serde_json::json!({
            "yiaddr": "192.168.1.100",
            "subnet_mask": "invalid",
        });

        let result = parse_dhcpm_output(&json).unwrap();
        // Invalid subnet_mask should be None, not an error
        assert!(result.subnet_mask.is_none());
    }

    #[test]
    fn test_parse_dhcpm_output_invalid_gateway() {
        let json = serde_json::json!({
            "yiaddr": "192.168.1.100",
            "router": ["invalid"],
        });

        let result = parse_dhcpm_output(&json).unwrap();
        // Invalid gateway should be None, not an error
        assert!(result.gateway.is_none());
    }

    #[test]
    fn test_parse_dhcpm_output_invalid_dns() {
        let json = serde_json::json!({
            "yiaddr": "192.168.1.100",
            "dns": ["invalid", "8.8.8.8"],
        });

        let result = parse_dhcpm_output(&json).unwrap();
        // Should filter out invalid DNS servers
        assert_eq!(result.dns_servers.len(), 1);
        assert_eq!(result.dns_servers[0].to_string(), "8.8.8.8");
    }

    #[test]
    fn test_ip_in_pool_range_invalid_format() {
        let ip: IpAddr = "192.168.1.100".parse().unwrap();
        
        // Missing dash
        let result = ip_in_pool_range(&ip, "192.168.1.100");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid pool range format"));
        
        // Too many dashes
        let result = ip_in_pool_range(&ip, "192.168.1.100-192.168.1.200-300");
        assert!(result.is_err());
    }

    #[test]
    fn test_ip_in_pool_range_invalid_ip() {
        let ip: IpAddr = "192.168.1.100".parse().unwrap();
        
        let result = ip_in_pool_range(&ip, "invalid-192.168.1.200");
        assert!(result.is_err());
        
        let result = ip_in_pool_range(&ip, "192.168.1.100-invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_ip_in_pool_range_boundary() {
        let start_ip: IpAddr = "192.168.1.100".parse().unwrap();
        let middle_ip: IpAddr = "192.168.1.150".parse().unwrap();
        let end_ip: IpAddr = "192.168.1.200".parse().unwrap();
        let outside_ip: IpAddr = "192.168.1.99".parse().unwrap();
        
        // Test at boundaries
        assert!(ip_in_pool_range(&start_ip, "192.168.1.100-192.168.1.200").unwrap());
        assert!(ip_in_pool_range(&end_ip, "192.168.1.100-192.168.1.200").unwrap());
        assert!(ip_in_pool_range(&middle_ip, "192.168.1.100-192.168.1.200").unwrap());
        assert!(!ip_in_pool_range(&outside_ip, "192.168.1.100-192.168.1.200").unwrap());
    }

    #[test]
    fn test_ip_in_pool_range_ipv6_not_supported() {
        let ip: IpAddr = "2001:db8::1".parse().unwrap();
        let result = ip_in_pool_range(&ip, "2001:db8::1-2001:db8::100");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("only supports IPv4"));
    }
}

