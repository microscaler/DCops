//! Prefix Resolver - Resolves prefixes for IP ranges and addresses

use crate::error::ControllerError;
use crds::NetBoxPrefix;
use crds::ipam::PrefixState;
use kube::Api;
use ipnet::IpNet;
use std::str::FromStr;
use std::net::IpAddr;

/// Resolves prefixes for IP ranges and addresses
pub struct PrefixResolver {
    prefix_api: Api<NetBoxPrefix>,
}

impl PrefixResolver {
    /// Create a new Prefix Resolver
    pub fn new(prefix_api: Api<NetBoxPrefix>) -> Self {
        Self { prefix_api }
    }

    /// Find the prefix that contains a given IP range
    ///
    /// Returns the most specific (longest prefix) match if multiple prefixes contain the range.
    pub async fn find_prefix_for_range(&self, start: &str, end: &str) -> Result<Option<String>, ControllerError> {
        // List all prefixes and find one that contains both start and end IPs
        let prefixes = self.prefix_api.list(&kube::api::ListParams::default()).await?;
        
        let start_ip = start.split('/').next().unwrap_or(start);
        let end_ip = end.split('/').next().unwrap_or(end);
        
        let start_addr: IpAddr = start_ip.parse()
            .map_err(|e| ControllerError::InvalidInput(format!("Invalid start IP '{}': {}", start, e)))?;
        let end_addr: IpAddr = end_ip.parse()
            .map_err(|e| ControllerError::InvalidInput(format!("Invalid end IP '{}': {}", end, e)))?;
        
        let mut best_match: Option<(String, u8)> = None; // (prefix_cidr, prefix_length)
        
        for prefix_crd in prefixes.items {
            if let Some(status) = &prefix_crd.status {
                if status.state == PrefixState::Created {
                    let prefix_cidr = &prefix_crd.spec.prefix;
                    if let Ok(prefix_net) = IpNet::from_str(prefix_cidr) {
                        if prefix_net.contains(&start_addr) && prefix_net.contains(&end_addr) {
                            let prefix_len = prefix_net.prefix_len();
                            // Keep the most specific (longest prefix) match
                            if best_match.is_none() || best_match.as_ref().unwrap().1 < prefix_len {
                                best_match = Some((prefix_cidr.clone(), prefix_len));
                            }
                        }
                    }
                }
            }
        }
        
        Ok(best_match.map(|(prefix, _)| prefix))
    }

    /// Find the prefix that contains a given IP address
    ///
    /// Returns the most specific (longest prefix) match if multiple prefixes contain the IP.
    pub async fn find_prefix_for_address(&self, address: &str) -> Result<Option<String>, ControllerError> {
        // List all prefixes and find one that contains the IP address
        let prefixes = self.prefix_api.list(&kube::api::ListParams::default()).await?;
        
        let ip_str = address.split('/').next().unwrap_or(address);
        let ip_addr: IpAddr = ip_str.parse()
            .map_err(|e| ControllerError::InvalidInput(format!("Invalid IP address '{}': {}", address, e)))?;
        
        let mut best_match: Option<(String, u8)> = None; // (prefix_cidr, prefix_length)
        
        for prefix_crd in prefixes.items {
            if let Some(status) = &prefix_crd.status {
                if status.state == PrefixState::Created {
                    let prefix_cidr = &prefix_crd.spec.prefix;
                    if let Ok(prefix_net) = IpNet::from_str(prefix_cidr) {
                        if prefix_net.contains(&ip_addr) {
                            let prefix_len = prefix_net.prefix_len();
                            // Keep the most specific (longest prefix) match
                            if best_match.is_none() || best_match.as_ref().unwrap().1 < prefix_len {
                                best_match = Some((prefix_cidr.clone(), prefix_len));
                            }
                        }
                    }
                }
            }
        }
        
        Ok(best_match.map(|(prefix, _)| prefix))
    }
}

// Note: Integration tests for PrefixResolver would require a mock Kubernetes API client
// These are better suited for integration test suites with actual or mocked K8s clusters
