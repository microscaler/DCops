//! Config Builder - Builds Kea configuration from NetBox CRDs

use crate::error::ControllerError;
use crds::{NetBoxPrefix, NetBoxIPRange, NetBoxIPAddress};
use crds::ipam::PrefixState;
use kube::Api;
use serde_json::{json, Value};
use tracing::debug;
use super::prefix_resolver::PrefixResolver;
use super::ip_utils::IpUtils;

/// Builds Kea configuration from NetBox CRDs
pub struct ConfigBuilder {
    prefix_api: Api<NetBoxPrefix>,
    ip_range_api: Api<NetBoxIPRange>,
    ip_address_api: Api<NetBoxIPAddress>,
    prefix_resolver: PrefixResolver,
    ip_utils: IpUtils,
}

impl ConfigBuilder {
    /// Create a new Config Builder
    pub fn new(
        prefix_api: Api<NetBoxPrefix>,
        ip_range_api: Api<NetBoxIPRange>,
        ip_address_api: Api<NetBoxIPAddress>,
    ) -> Self {
        let prefix_resolver = PrefixResolver::new(prefix_api.clone());
        let ip_utils = IpUtils::new();
        
        Self {
            prefix_api,
            ip_range_api,
            ip_address_api,
            prefix_resolver,
            ip_utils,
        }
    }

    /// Build Kea configuration from all NetBox CRDs
    pub async fn build_kea_config_from_crds(&self) -> Result<Value, ControllerError> {
        // List all relevant CRDs
        let prefixes = self.prefix_api.list(&kube::api::ListParams::default()).await?;
        let ip_ranges = self.ip_range_api.list(&kube::api::ListParams::default()).await?;
        let ip_addresses = self.ip_address_api.list(&kube::api::ListParams::default()).await?;
        
        debug!("Found {} prefixes, {} IP ranges, {} IP addresses", 
            prefixes.items.len(), ip_ranges.items.len(), ip_addresses.items.len());
        
        // Build subnet map: prefix -> (pools, reservations)
        let mut subnet_map: std::collections::HashMap<String, (Vec<Value>, Vec<Value>)> = std::collections::HashMap::new();
        
        // Process prefixes (create subnet entries)
        self.process_prefixes(&prefixes.items, &mut subnet_map);
        
        // Process IP ranges (add pools to subnets)
        self.process_ip_ranges(&ip_ranges.items, &mut subnet_map).await?;
        
        // Process IP addresses (add reservations to subnets)
        self.process_ip_addresses(&ip_addresses.items, &mut subnet_map).await?;
        
        // Build Kea configuration
        self.build_kea_config(subnet_map)
    }

    /// Process prefixes and create subnet entries
    fn process_prefixes(
        &self,
        prefixes: &[NetBoxPrefix],
        subnet_map: &mut std::collections::HashMap<String, (Vec<Value>, Vec<Value>)>,
    ) {
        for prefix_crd in prefixes {
            // Only process prefixes that are ready (have netbox_id)
            if let Some(status) = &prefix_crd.status {
                if status.state == PrefixState::Created && status.netbox_id.is_some() {
                    let prefix_cidr = &prefix_crd.spec.prefix;
                    subnet_map.insert(prefix_cidr.clone(), (Vec::new(), Vec::new()));
                    debug!("Added subnet from prefix: {}", prefix_cidr);
                }
            }
        }
    }

    /// Process IP ranges and add pools to subnets
    async fn process_ip_ranges(
        &self,
        ip_ranges: &[NetBoxIPRange],
        subnet_map: &mut std::collections::HashMap<String, (Vec<Value>, Vec<Value>)>,
    ) -> Result<(), ControllerError> {
        use crds::ipam::IPRangeStatus;
        
        for ip_range_crd in ip_ranges {
            // Only process IP ranges with status Active and that are ready
            // TODO: Add proper DHCP filtering via annotations/tags
            if ip_range_crd.spec.status == IPRangeStatus::Active {
                if let Some(status) = &ip_range_crd.status {
                    if let Some(netbox_id) = status.netbox_id {
                        if netbox_id > 0 {
                            let range_start = &ip_range_crd.spec.start_address;
                            let range_end = &ip_range_crd.spec.end_address;
                            
                            // Find the subnet this range belongs to
                            if let Some(prefix) = self.prefix_resolver.find_prefix_for_range(range_start, range_end).await? {
                                if let Some((pools, _reservations)) = subnet_map.get_mut(&prefix) {
                                    let pool_range = format!("{}-{}", 
                                        self.ip_utils.extract_ip_from_cidr(range_start),
                                        self.ip_utils.extract_ip_from_cidr(range_end));
                                    pools.push(json!({
                                        "pool": pool_range
                                    }));
                                    debug!("Added pool {} to subnet {}", pool_range, prefix);
                                }
                            } else {
                                debug!("Could not find prefix for IP range {}-{}, skipping pool", range_start, range_end);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Process IP addresses and add reservations to subnets
    async fn process_ip_addresses(
        &self,
        ip_addresses: &[NetBoxIPAddress],
        subnet_map: &mut std::collections::HashMap<String, (Vec<Value>, Vec<Value>)>,
    ) -> Result<(), ControllerError> {
        for ip_address_crd in ip_addresses {
            // Only process IP addresses with status "dhcp" and that are ready
            if ip_address_crd.spec.status == crds::IPAddressStatus::Dhcp {
                if let Some(status) = &ip_address_crd.status {
                    if let Some(netbox_id) = status.netbox_id {
                        if netbox_id > 0 {
                            // Get IP address and MAC address
                            if let Some(address) = &ip_address_crd.spec.address {
                                // Find the actual prefix that contains this IP address
                                if let Some(prefix) = self.prefix_resolver.find_prefix_for_address(address).await? {
                                    if let Some((_pools, reservations)) = subnet_map.get_mut(&prefix) {
                                        // Get MAC address from spec
                                        if let Some(mac) = &ip_address_crd.spec.mac_address {
                                            let ip = self.ip_utils.extract_ip_from_cidr(address);
                                            reservations.push(json!({
                                                "ip-address": ip,
                                                "hw-address": mac
                                            }));
                                            debug!("Added reservation {} -> {} to subnet {}", mac, ip, prefix);
                                        }
                                    }
                                } else {
                                    debug!("Could not find prefix for IP address {}, skipping reservation", address);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Build final Kea configuration from subnet map
    fn build_kea_config(
        &self,
        subnet_map: std::collections::HashMap<String, (Vec<Value>, Vec<Value>)>,
    ) -> Result<Value, ControllerError> {
        let mut subnet4 = Vec::new();
        for (subnet_cidr, (pools, reservations)) in subnet_map {
            subnet4.push(json!({
                "subnet": subnet_cidr,
                "pools": pools,
                "reservations": reservations
            }));
        }
        
        let config = json!({
            "interfaces-config": {
                "interfaces": ["*"]
            },
            "lease-database": {
                "type": "memfile"
            },
            "subnet4": subnet4
        });
        
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Test build_kea_config logic directly without needing a full ConfigBuilder
    #[test]
    fn test_build_kea_config_logic() {
        let mut subnet_map = std::collections::HashMap::new();
        
        // Test empty map
        let config = build_kea_config_static(subnet_map.clone()).unwrap();
        assert!(config.get("subnet4").unwrap().as_array().unwrap().is_empty());
        assert_eq!(config.get("interfaces-config").unwrap().get("interfaces").unwrap().as_array().unwrap()[0], "*");
        assert_eq!(config.get("lease-database").unwrap().get("type").unwrap(), "memfile");
        
        // Test with one subnet
        subnet_map.insert(
            "192.168.1.0/24".to_string(),
            (
                vec![json!({"pool": "192.168.1.100-192.168.1.200"})],
                vec![json!({"ip-address": "192.168.1.10", "hw-address": "aa:bb:cc:dd:ee:ff"})],
            ),
        );
        
        let config = build_kea_config_static(subnet_map.clone()).unwrap();
        let subnet4 = config.get("subnet4").unwrap().as_array().unwrap();
        assert_eq!(subnet4.len(), 1);
        
        let subnet = &subnet4[0];
        assert_eq!(subnet.get("subnet").unwrap(), "192.168.1.0/24");
        assert_eq!(subnet.get("pools").unwrap().as_array().unwrap().len(), 1);
        assert_eq!(subnet.get("reservations").unwrap().as_array().unwrap().len(), 1);
        
        // Test with multiple subnets
        subnet_map.insert("10.0.0.0/16".to_string(), (Vec::new(), Vec::new()));
        let config = build_kea_config_static(subnet_map).unwrap();
        let subnet4 = config.get("subnet4").unwrap().as_array().unwrap();
        assert_eq!(subnet4.len(), 2);
    }
}

// Helper function for testing build_kea_config logic without needing K8s client
#[cfg(test)]
fn build_kea_config_static(
    subnet_map: std::collections::HashMap<String, (Vec<Value>, Vec<Value>)>,
) -> Result<Value, ControllerError> {
    let mut subnet4 = Vec::new();
    for (subnet_cidr, (pools, reservations)) in subnet_map {
        subnet4.push(json!({
            "subnet": subnet_cidr,
            "pools": pools,
            "reservations": reservations
        }));
    }
    
    let config = json!({
        "interfaces-config": {
            "interfaces": ["*"]
        },
        "lease-database": {
            "type": "memfile"
        },
        "subnet4": subnet4
    });
    
    Ok(config)
}
