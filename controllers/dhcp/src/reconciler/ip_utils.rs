//! IP Utilities - IP address and CIDR manipulation functions

use ipnet::IpNet;
use std::str::FromStr;

/// IP address and CIDR utility functions
pub struct IpUtils;

impl IpUtils {
    /// Create a new IpUtils instance
    pub fn new() -> Self {
        Self
    }

    /// Extract IP address from CIDR notation (e.g., "192.168.1.100/24" -> "192.168.1.100")
    pub fn extract_ip_from_cidr(&self, cidr: &str) -> String {
        cidr.split('/').next().unwrap_or(cidr).to_string()
    }

    /// Extract network prefix from a CIDR address
    ///
    /// Given an IP address with CIDR notation (e.g., "192.168.1.100/24"),
    /// returns the network prefix (e.g., "192.168.1.0/24").
    ///
    /// # Examples
    /// ```
    /// let utils = IpUtils::new();
    /// assert_eq!(utils.extract_network_prefix("192.168.1.100/24").unwrap(), "192.168.1.0/24");
    /// assert_eq!(utils.extract_network_prefix("10.0.0.5/16").unwrap(), "10.0.0.0/16");
    /// ```
    pub fn extract_network_prefix(&self, cidr: &str) -> Result<String, String> {
        let ip_net = IpNet::from_str(cidr)
            .map_err(|e| format!("Invalid CIDR notation '{}': {}", cidr, e))?;
        
        let network = ip_net.network();
        let prefix_len = ip_net.prefix_len();
        Ok(format!("{}/{}", network, prefix_len))
    }

    /// Check if an IP address is within a CIDR prefix
    ///
    /// # Examples
    /// ```
    /// let utils = IpUtils::new();
    /// assert!(utils.is_ip_in_prefix("192.168.1.100", "192.168.1.0/24").unwrap());
    /// assert!(!utils.is_ip_in_prefix("10.0.0.5", "192.168.1.0/24").unwrap());
    /// ```
    pub fn is_ip_in_prefix(&self, ip: &str, prefix: &str) -> Result<bool, String> {
        use std::net::IpAddr;
        
        let ip_addr: IpAddr = ip.parse()
            .map_err(|e| format!("Invalid IP address '{}': {}", ip, e))?;
        
        let prefix_net = IpNet::from_str(prefix)
            .map_err(|e| format!("Invalid prefix '{}': {}", prefix, e))?;
        
        Ok(prefix_net.contains(&ip_addr))
    }
}

impl Default for IpUtils {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_ip_from_cidr() {
        let utils = IpUtils::new();
        
        assert_eq!(utils.extract_ip_from_cidr("192.168.1.100/24"), "192.168.1.100");
        assert_eq!(utils.extract_ip_from_cidr("10.0.0.5/16"), "10.0.0.5");
        assert_eq!(utils.extract_ip_from_cidr("192.168.1.1"), "192.168.1.1");
        assert_eq!(utils.extract_ip_from_cidr("2001:db8::1/64"), "2001:db8::1");
    }

    #[test]
    fn test_extract_network_prefix() {
        let utils = IpUtils::new();
        
        // IPv4 tests
        assert_eq!(utils.extract_network_prefix("192.168.1.100/24").unwrap(), "192.168.1.0/24");
        assert_eq!(utils.extract_network_prefix("10.0.0.5/16").unwrap(), "10.0.0.0/16");
        assert_eq!(utils.extract_network_prefix("172.16.0.1/12").unwrap(), "172.16.0.0/12");
        assert_eq!(utils.extract_network_prefix("192.168.1.1/32").unwrap(), "192.168.1.1/32");
        assert_eq!(utils.extract_network_prefix("10.0.0.0/8").unwrap(), "10.0.0.0/8");
        
        // IPv6 tests
        assert_eq!(utils.extract_network_prefix("2001:db8::1/64").unwrap(), "2001:db8::/64");
        assert_eq!(utils.extract_network_prefix("2001:db8:1::1/48").unwrap(), "2001:db8:1::/48");
        
        // Edge cases
        assert_eq!(utils.extract_network_prefix("0.0.0.0/0").unwrap(), "0.0.0.0/0");
        
        // Invalid inputs
        assert!(utils.extract_network_prefix("invalid").is_err());
        assert!(utils.extract_network_prefix("192.168.1.1").is_err());
        assert!(utils.extract_network_prefix("192.168.1.1/33").is_err()); // Invalid prefix length
    }

    #[test]
    fn test_is_ip_in_prefix() {
        let utils = IpUtils::new();
        
        // IPv4 tests
        assert!(utils.is_ip_in_prefix("192.168.1.100", "192.168.1.0/24").unwrap());
        assert!(utils.is_ip_in_prefix("192.168.1.255", "192.168.1.0/24").unwrap());
        assert!(!utils.is_ip_in_prefix("192.168.2.1", "192.168.1.0/24").unwrap());
        assert!(!utils.is_ip_in_prefix("10.0.0.5", "192.168.1.0/24").unwrap());
        
        // Different prefix lengths
        assert!(utils.is_ip_in_prefix("10.0.0.5", "10.0.0.0/16").unwrap());
        assert!(utils.is_ip_in_prefix("10.0.255.255", "10.0.0.0/16").unwrap());
        assert!(!utils.is_ip_in_prefix("10.1.0.1", "10.0.0.0/16").unwrap());
        
        // /32 prefix (single host)
        assert!(utils.is_ip_in_prefix("192.168.1.1", "192.168.1.1/32").unwrap());
        assert!(!utils.is_ip_in_prefix("192.168.1.2", "192.168.1.1/32").unwrap());
        
        // /8 prefix (large network)
        assert!(utils.is_ip_in_prefix("10.1.2.3", "10.0.0.0/8").unwrap());
        assert!(utils.is_ip_in_prefix("10.255.255.255", "10.0.0.0/8").unwrap());
        
        // IPv6 tests
        assert!(utils.is_ip_in_prefix("2001:db8::1", "2001:db8::/64").unwrap());
        assert!(utils.is_ip_in_prefix("2001:db8::ffff", "2001:db8::/64").unwrap());
        assert!(!utils.is_ip_in_prefix("2001:db9::1", "2001:db8::/64").unwrap());
        
        // Invalid inputs
        assert!(utils.is_ip_in_prefix("invalid", "192.168.1.0/24").is_err());
        assert!(utils.is_ip_in_prefix("192.168.1.1", "invalid").is_err());
    }
}
