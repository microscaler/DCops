//! IPAM operations for MockNetBoxClient
//!
//! Handles prefixes, IP addresses, aggregates, RIRs, and VLANs

use std::str::FromStr;

use super::MockNetBoxClient;
use crate::error::NetBoxError;
use crate::models::*;
use crate::types::*;

pub async fn get_prefix(client: &MockNetBoxClient, id: PrefixId) -> Result<Prefix, NetBoxError> {
        let id_value: u64 = id.into();
        client.prefixes
            .lock()
            .unwrap()
            .get(&id_value)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Prefix {} not found", id_value)))
}

pub async fn get_available_ips(client: &MockNetBoxClient, prefix_id: PrefixId, _limit: Option<u32>) -> Result<Vec<AvailableIP>, NetBoxError> {
        let prefix_id_value: u64 = prefix_id.into();
        Ok(client.available_ips
            .lock()
            .unwrap()
            .get(&prefix_id_value)
            .cloned()
            .unwrap_or_default())
}

pub async fn allocate_ip(client: &MockNetBoxClient, prefix_id: PrefixId, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError> {
        // Verify prefix exists
        get_prefix(client, prefix_id).await?;

        use std::str::FromStr;
        use ipnet::IpNet;
        
        let id = client.next_id();
        let address_str = request
            .as_ref()
            .and_then(|r| r.address.as_ref().map(|a| a.to_string()))
            .unwrap_or_else(|| format!("192.168.1.{}/24", id));
        
        let address_net = IpNet::from_str(&address_str)
            .map_err(|e| NetBoxError::Api(format!("Invalid IP address format: {} - {}", address_str, e)))?;

        let ip = IPAddress {
            id,
            url: format!("{}/api/ipam/ip-addresses/{}/", client.base_url, id),
            display: address_str.clone(),
            family: 4, // Default to IPv4
            address: address_net,
            vrf: None,
            tenant: None, // AllocateIPRequest doesn't have tenant field
            status: request
                .as_ref()
                .and_then(|r| r.status.clone())
                .unwrap_or(IPAddressStatus::Active),
            role: request.as_ref().and_then(|r| r.role.clone()),
            assigned_object_type: request.as_ref().and_then(|r| r.assigned_object_type.clone()),
            assigned_object_id: request.as_ref().and_then(|r| r.assigned_object_id),
            assigned_object: None,
            nat_inside: None,
            nat_outside: vec![],
            dns_name: request.as_ref().and_then(|r| r.dns_name.clone()).unwrap_or_default(),
            description: request.as_ref().and_then(|r| r.description.clone()).unwrap_or_default(),
            comments: String::new(),
            tags: request.as_ref().and_then(|r| r.tags.clone())
                .map(|tags_vec| {
                    tags_vec.into_iter()
                        .filter_map(|v| client.helpers().create_nested_tag(&v))
                        .collect()
                })
                .unwrap_or_default(),
            custom_fields: serde_json::json!({}),
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.ip_addresses.lock().unwrap().insert(id, ip.clone());
        Ok(ip)
}

pub async fn get_ip_address(client: &MockNetBoxClient, id: IpAddressId) -> Result<IPAddress, NetBoxError> {
        let id_value: u64 = id.into();
        client.ip_addresses
            .lock()
            .unwrap()
            .get(&id_value)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("IP address {} not found", id_value)))
}

pub async fn query_ip_addresses(client: &MockNetBoxClient, filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<IPAddress>, NetBoxError> {
        let ips = client.ip_addresses.lock().unwrap();
        let mut results: Vec<IPAddress> = ips.values().cloned().collect();

        // Apply filters (properly handles prefix filter using ipnet for IP network checking)
        for (key, value) in filters {
            if *key == "prefix" {
                // Parse the prefix as an IP network using ipnet
                let prefix_net = match ipnet::IpNet::from_str(value) {
                    Ok(net) => net,
                    Err(_) => {
                        // If prefix parsing fails, skip this filter (invalid prefix format)
                        continue;
                    }
                };
                
                // Filter IPs that are within the prefix network
                results.retain(|ip| {
                    // Check if the IP address network is contained within the prefix network
                    prefix_net.contains(&ip.address.addr())
                });
            }
        }

        Ok(results)
}

pub async fn query_prefixes(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Prefix>, NetBoxError> {
        let prefixes = client.prefixes.lock().unwrap();
        Ok(prefixes.values().cloned().collect())
}

pub async fn create_ip_address(client: &MockNetBoxClient, address: &str, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError> {
        // Note: This function still accepts &str for internal mock use
        // The trait method converts IpNet to string before calling this
        use std::str::FromStr;
        use ipnet::IpNet;
        
        let id = client.next_id();
        let address_net = IpNet::from_str(address)
            .map_err(|e| NetBoxError::Api(format!("Invalid IP address format: {} - {}", address, e)))?;
        let ip = IPAddress {
            id,
            url: format!("{}/api/ipam/ip-addresses/{}/", client.base_url, id),
            display: address.to_string(),
            family: if address.contains(':') { 6 } else { 4 },
            address: address_net,
            vrf: None,
            tenant: None, // AllocateIPRequest doesn't have tenant field
            status: request
                .as_ref()
                .and_then(|r| r.status.clone())
                .unwrap_or(IPAddressStatus::Active),
            role: request.as_ref().and_then(|r| r.role.clone()),
            assigned_object_type: request.as_ref().and_then(|r| r.assigned_object_type.clone()),
            assigned_object_id: request.as_ref().and_then(|r| r.assigned_object_id),
            assigned_object: None,
            nat_inside: None,
            nat_outside: vec![],
            dns_name: request.as_ref().and_then(|r| r.dns_name.clone()).unwrap_or_default(),
            description: request.as_ref().and_then(|r| r.description.clone()).unwrap_or_default(),
            comments: String::new(),
            tags: request.as_ref().and_then(|r| r.tags.clone())
                .map(|tags_vec| {
                    tags_vec.into_iter()
                        .filter_map(|v| client.helpers().create_nested_tag(&v))
                        .collect()
                })
                .unwrap_or_default(),
            custom_fields: serde_json::json!({}),
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.ip_addresses.lock().unwrap().insert(id, ip.clone());
        Ok(ip)
}

pub async fn update_ip_address(client: &MockNetBoxClient, id: IpAddressId, request: AllocateIPRequest) -> Result<IPAddress, NetBoxError> {
        let id_value: u64 = id.into();
        let mut ips = client.ip_addresses.lock().unwrap();
        let ip = ips
            .get_mut(&id_value)
            .ok_or_else(|| NetBoxError::NotFound(format!("IP address {} not found", id_value)))?;

        if let Some(description) = request.description {
            ip.description = description;
        }
        if let Some(status) = request.status {
            ip.status = status;
        }
        if let Some(dns_name) = request.dns_name {
            ip.dns_name = dns_name;
        }
        if let Some(role) = request.role {
            ip.role = Some(role);
        }
        if let Some(tags) = request.tags {
            ip.tags = tags.into_iter()
                .filter_map(|v| v.as_str().map(|s| NestedTag {
                    id: 0,
                    url: format!("{}/api/extras/tags/0/", client.base_url),
                    display: s.to_string(),
                    name: s.to_string(),
                    slug: s.to_lowercase().replace(' ', "-"),
                }))
                .collect();
        }

        Ok(ip.clone())
}

pub async fn delete_ip_address(client: &MockNetBoxClient, id: u64) -> Result<(), NetBoxError> {
        client.ip_addresses
            .lock()
            .unwrap()
            .remove(&id)
            .ok_or_else(|| NetBoxError::NotFound(format!("IP address {} not found", id)))
            .map(|_| ())
}

// IP Range operations
pub async fn get_ip_range(client: &MockNetBoxClient, id: IPRangeId) -> Result<IPRange, NetBoxError> {
    let id_value: u64 = id.into();
    client.ip_ranges
        .lock()
        .unwrap()
        .get(&id_value)
        .cloned()
        .ok_or_else(|| NetBoxError::NotFound(format!("IP range {} not found", id_value)))
}

pub async fn query_ip_ranges(client: &MockNetBoxClient, filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<IPRange>, NetBoxError> {
    let ranges = client.ip_ranges.lock().unwrap();
    let mut results: Vec<IPRange> = ranges.values().cloned().collect();
    
    // Apply filters (simplified - just check tenant_id for now)
    if !filters.is_empty() {
        results = results.into_iter()
            .filter(|range| {
                filters.iter().all(|(key, value)| {
                    match *key {
                        "tenant_id" => {
                            range.tenant.as_ref()
                                .map(|t| t.id.to_string() == *value)
                                .unwrap_or(false)
                        }
                        _ => true, // Ignore unknown filters
                    }
                })
            })
            .collect();
    }
    
    Ok(results)
}

pub async fn create_ip_range(client: &MockNetBoxClient, start_address: &ipnet::IpNet, end_address: &ipnet::IpNet, vrf_id: Option<u64>, tenant_id: Option<TenantId>, role_id: Option<RoleId>, status: Option<IPRangeStatus>, description: Option<String>, mark_utilized: Option<bool>, mark_populated: Option<bool>, tags: Option<Vec<String>>) -> Result<IPRange, NetBoxError> {
    let id = client.next_id();
    let status_value = status.unwrap_or(IPRangeStatus::Active);
    
    let vrf = vrf_id.map(|id| {
        use crate::models::NestedVrf;
        NestedVrf {
            id,
            url: format!("{}/api/ipam/vrfs/{}/", client.base_url, id),
            display: format!("VRF {}", id),
            name: format!("VRF {}", id),
        }
    });
    
    let tenant = tenant_id.map(|id| {
        use crate::models::NestedTenant;
        let id_u64 = u64::from(id);
        NestedTenant {
            id: id_u64,
            url: format!("{}/api/tenancy/tenants/{}/", client.base_url, id_u64),
            display: format!("Tenant {}", id_u64),
            name: format!("Tenant {}", id_u64),
            slug: format!("tenant-{}", id_u64),
        }
    });
    
    let role = role_id.map(|id| {
        use crate::models::NestedRole;
        let id_u64 = u64::from(id);
        NestedRole {
            id: id_u64,
            url: format!("{}/api/ipam/roles/{}/", client.base_url, id_u64),
            display: format!("Role {}", id_u64),
            name: format!("Role {}", id_u64),
            slug: format!("role-{}", id_u64),
        }
    });
    
    let tags_vec: Vec<NestedTag> = if let Some(tags) = tags {
        let tags_json: Vec<serde_json::Value> = tags.into_iter()
            .map(|s| serde_json::Value::String(s))
            .collect();
        client.helpers().convert_tags(tags_json)
    } else {
        Vec::new()
    };
    
    let ip_range = IPRange {
        id,
        url: format!("{}/api/ipam/ip-ranges/{}/", client.base_url, id),
        display: format!("{} - {}", start_address, end_address),
        family: if start_address.addr().is_ipv6() { 6 } else { 4 },
        start_address: *start_address,
        end_address: *end_address,
        vrf,
        tenant,
        status: status_value,
        role,
        description: description.unwrap_or_default(),
        mark_utilized: mark_utilized.unwrap_or(false),
        mark_populated: mark_populated.unwrap_or(false),
        tags: tags_vec,
        custom_fields: serde_json::json!({}),
        created: chrono::Utc::now().to_rfc3339(),
        last_updated: chrono::Utc::now().to_rfc3339(),
    };
    
    client.ip_ranges.lock().unwrap().insert(id, ip_range.clone());
    Ok(ip_range)
}

pub async fn update_ip_range(client: &MockNetBoxClient, id: IPRangeId, start_address: Option<&ipnet::IpNet>, end_address: Option<&ipnet::IpNet>, vrf_id: Option<u64>, tenant_id: Option<TenantId>, role_id: Option<RoleId>, status: Option<IPRangeStatus>, description: Option<String>, mark_utilized: Option<bool>, mark_populated: Option<bool>, tags: Option<Vec<String>>) -> Result<IPRange, NetBoxError> {
    let id_value: u64 = id.into();
    let mut ranges = client.ip_ranges.lock().unwrap();
    let range = ranges.get_mut(&id_value)
        .ok_or_else(|| NetBoxError::NotFound(format!("IP range {} not found", id_value)))?;
    
    if let Some(start) = start_address {
        range.start_address = *start;
    }
    if let Some(end) = end_address {
        range.end_address = *end;
    }
    if let Some(vrf) = vrf_id {
        use crate::models::NestedVrf;
        range.vrf = Some(NestedVrf {
            id: vrf,
            url: format!("{}/api/ipam/vrfs/{}/", client.base_url, vrf),
            display: format!("VRF {}", vrf),
            name: format!("VRF {}", vrf),
        });
    }
    if let Some(tenant) = tenant_id {
        use crate::models::NestedTenant;
        let tenant_u64 = u64::from(tenant);
        range.tenant = Some(NestedTenant {
            id: tenant_u64,
            url: format!("{}/api/tenancy/tenants/{}/", client.base_url, tenant_u64),
            display: format!("Tenant {}", tenant_u64),
            name: format!("Tenant {}", tenant_u64),
            slug: format!("tenant-{}", tenant_u64),
        });
    }
    if let Some(role) = role_id {
        use crate::models::NestedRole;
        let role_u64 = u64::from(role);
        range.role = Some(NestedRole {
            id: role_u64,
            url: format!("{}/api/ipam/roles/{}/", client.base_url, role_u64),
            display: format!("Role {}", role_u64),
            name: format!("Role {}", role_u64),
            slug: format!("role-{}", role_u64),
        });
    }
    if let Some(status_value) = status {
        range.status = status_value;
    }
    if let Some(desc) = description {
        range.description = desc;
    }
    if let Some(utilized) = mark_utilized {
        range.mark_utilized = utilized;
    }
    if let Some(populated) = mark_populated {
        range.mark_populated = populated;
    }
    if let Some(tags) = tags {
        let tags_json: Vec<serde_json::Value> = tags.into_iter()
            .map(|s| serde_json::Value::String(s))
            .collect();
        range.tags = client.helpers().convert_tags(tags_json);
    }
    
    range.last_updated = chrono::Utc::now().to_rfc3339();
    Ok(range.clone())
}

pub async fn delete_ip_range(client: &MockNetBoxClient, id: u64) -> Result<(), NetBoxError> {
    client.ip_ranges
        .lock()
        .unwrap()
        .remove(&id)
        .ok_or_else(|| NetBoxError::NotFound(format!("IP range {} not found", id)))
        .map(|_| ())
}

pub async fn create_prefix(client: &MockNetBoxClient, prefix: &str, description: Option<String>, _site_id: Option<SiteId>, vlan_id: Option<VlanId>, status: Option<&str>, role_id: Option<RoleId>, tenant_id: Option<TenantId>, tags: Option<Vec<String>>) -> Result<Prefix, NetBoxError> {
        // Note: This function still accepts &str for internal mock use
        // The trait method converts IpNet to string before calling this
        let id = client.next_id();
        let status_str = status.unwrap_or("active");
        let prefix_status = match status_str {
            "active" => PrefixStatus::Active,
            "reserved" => PrefixStatus::Reserved,
            "deprecated" => PrefixStatus::Deprecated,
            "container" => PrefixStatus::Container,
            _ => PrefixStatus::Active,
        };
        
        // Convert Vec<String> to Vec<serde_json::Value> for helper
        let tags_vec: Vec<NestedTag> = if let Some(tags) = tags {
            let tags_json: Vec<serde_json::Value> = tags.into_iter()
                .map(|s| serde_json::Value::String(s))
                .collect();
            client.helpers().convert_tags(tags_json)
        } else {
            Vec::new()
        };
        
        use std::str::FromStr;
        use ipnet::IpNet;
        
        let prefix_net = IpNet::from_str(prefix)
            .map_err(|e| NetBoxError::Api(format!("Invalid prefix format: {} - {}", prefix, e)))?;
        let prefix_obj = Prefix {
            id,
            url: format!("{}/api/ipam/prefixes/{}/", client.base_url, id),
            display: prefix.to_string(),
            family: if prefix.contains(':') { 6 } else { 4 },
            prefix: prefix_net,
            vrf: None,
            tenant: tenant_id.map(|id| client.helpers().create_nested_tenant(id.into(), None)),
            vlan: vlan_id.map(|id| {
                let vlan_id_value: u32 = id.into();
                client.helpers().create_nested_vlan(vlan_id_value as u64, vlan_id_value as u16, None)
            }),
            status: prefix_status,
            role: role_id.map(|id| client.helpers().create_nested_role(id.into(), None)),
            is_pool: false,
            mark_utilized: false,
            description: description.unwrap_or_default(),
            comments: String::new(),
            tags: tags_vec,
            custom_fields: serde_json::json!({}),
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
            children: 0,
            _depth: 0,
        };

        client.prefixes.lock().unwrap().insert(id, prefix_obj.clone());
        Ok(prefix_obj)
    }

pub async fn update_prefix(client: &MockNetBoxClient, id: PrefixId, prefix: Option<&str>, description: Option<String>, status: Option<&str>, role: Option<String>, tenant_id: Option<TenantId>, _site_id: Option<SiteId>, vlan_id: Option<VlanId>, tags: Option<Vec<String>>) -> Result<Prefix, NetBoxError> {
        // Note: This function still accepts Option<&str> for internal mock use
        // The trait method converts Option<&IpNet> to Option<&str> before calling this
        let id_value: u64 = id.into();
        let mut prefixes = client.prefixes.lock().unwrap();
        let prefix_obj = prefixes
            .get_mut(&id_value)
            .ok_or_else(|| NetBoxError::NotFound(format!("Prefix {} not found", id)))?;

        if let Some(prefix_str) = prefix {
            use std::str::FromStr;
            use ipnet::IpNet;
            let prefix_net = IpNet::from_str(prefix_str)
                .map_err(|e| NetBoxError::Api(format!("Invalid prefix format: {} - {}", prefix_str, e)))?;
            prefix_obj.prefix = prefix_net;
            prefix_obj.display = prefix_str.to_string();
        }
        if let Some(tenant) = tenant_id {
            prefix_obj.tenant = Some(client.helpers().create_nested_tenant(tenant.into(), None));
        }
        if let Some(vlan) = vlan_id {
            let vlan_id_value: u32 = vlan.into();
            prefix_obj.vlan = Some(client.helpers().create_nested_vlan(vlan_id_value as u64, vlan_id_value as u16, None));
        }
        if let Some(role_str) = role {
            // Parse role string to ID (simplified - in real mock would look up)
            if let Ok(role_id) = role_str.parse::<u64>() {
                prefix_obj.role = Some(client.helpers().create_nested_role(role_id, None));
            }
        }
        if let Some(status_str) = status {
            prefix_obj.status = match status_str {
                "active" => PrefixStatus::Active,
                "reserved" => PrefixStatus::Reserved,
                "deprecated" => PrefixStatus::Deprecated,
                "container" => PrefixStatus::Container,
                _ => PrefixStatus::Active,
            };
        }
        if let Some(desc) = description {
            prefix_obj.description = desc;
        }
        if let Some(tags_val) = tags {
            prefix_obj.tags = tags_val.into_iter()
                .map(|s| NestedTag {
                    id: 0,
                    url: format!("{}/api/extras/tags/0/", client.base_url),
                    display: s.clone(),
                    name: s.clone(),
                    slug: s.to_lowercase().replace(' ', "-"),
                })
                .collect();
        }

        Ok(prefix_obj.clone())
    }

pub async fn query_aggregates(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Aggregate>, NetBoxError> {
        let aggregates = client.aggregates.lock().unwrap();
        Ok(aggregates.values().cloned().collect())
}

pub async fn get_aggregate(client: &MockNetBoxClient, id: AggregateId) -> Result<Aggregate, NetBoxError> {
        let id_value: u64 = id.into();
        client.aggregates
            .lock()
            .unwrap()
            .get(&id_value)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Aggregate {} not found", id_value)))
}

pub async fn create_aggregate(client: &MockNetBoxClient, prefix: &str, rir_id: Option<RirId>, date_allocated: Option<&str>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Aggregate, NetBoxError> {
        // Note: This function still accepts &str for internal mock use
        // The trait method converts IpNet to string before calling this
        let id = client.next_id();
        use std::str::FromStr;
        use ipnet::IpNet;
        
        let prefix_net = IpNet::from_str(prefix)
            .map_err(|e| NetBoxError::Api(format!("Invalid prefix format: {} - {}", prefix, e)))?;
        
        // Convert tag IDs to NestedTag
        let nested_tags: Vec<crate::models::NestedTag> = tags
            .unwrap_or_default()
            .iter()
            .map(|tag_id| {
                // Try to parse as ID, otherwise use as name
                let tag_id_num = tag_id.parse::<u64>().unwrap_or(0);
                crate::models::NestedTag {
                    id: tag_id_num,
                    url: format!("{}/api/extras/tags/{}/", client.base_url, tag_id_num),
                    display: tag_id.clone(),
                    name: tag_id.clone(),
                    slug: tag_id.to_lowercase().replace(' ', "-"),
                }
            })
            .collect();
        
        let aggregate = Aggregate {
            id,
            url: format!("{}/api/ipam/aggregates/{}/", client.base_url, id),
            display: prefix.to_string(),
            prefix: prefix_net,
            rir: rir_id.map(|id| {
                let rir_id_value: u64 = id.into();
                NestedRir {
                    id: rir_id_value,
                    url: format!("{}/api/ipam/rirs/{}/", client.base_url, rir_id_value),
                    display: format!("RIR {}", rir_id_value),
                    name: format!("RIR {}", rir_id_value),
                    slug: format!("rir-{}", rir_id_value),
                }
            }),
            date_allocated: date_allocated.map(|s| s.to_string()),
            description,
            comments,
            tags: nested_tags,
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.aggregates.lock().unwrap().insert(id, aggregate.clone());
        Ok(aggregate)
    }

pub async fn update_aggregate(client: &MockNetBoxClient, id: crate::types::AggregateId, rir_id: Option<RirId>, date_allocated: Option<&str>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Aggregate, NetBoxError> {
        let id_value: u64 = id.into();
        let mut aggregates = client.aggregates.lock().unwrap();
        let aggregate = aggregates.get_mut(&id_value)
            .ok_or_else(|| NetBoxError::NotFound(format!("Aggregate {} not found", id_value)))?;
        
        // Update fields
        if let Some(rir_id) = rir_id {
            let rir_id_value: u64 = rir_id.into();
            aggregate.rir = Some(NestedRir {
                id: rir_id_value,
                url: format!("{}/api/ipam/rirs/{}/", client.base_url, rir_id_value),
                display: format!("RIR {}", rir_id_value),
                name: format!("RIR {}", rir_id_value),
                slug: format!("rir-{}", rir_id_value),
            });
        }
        if date_allocated.is_some() {
            aggregate.date_allocated = date_allocated.map(|s| s.to_string());
        }
        if description.is_some() {
            aggregate.description = description;
        }
        if comments.is_some() {
            aggregate.comments = comments;
        }
        if tags.is_some() {
            // Convert tag IDs to NestedTag
            aggregate.tags = tags
                .unwrap_or_default()
                .iter()
                .map(|tag_id| {
                    let tag_id_num = tag_id.parse::<u64>().unwrap_or(0);
                    crate::models::NestedTag {
                        id: tag_id_num,
                        url: format!("{}/api/extras/tags/{}/", client.base_url, tag_id_num),
                        display: tag_id.clone(),
                        name: tag_id.clone(),
                        slug: tag_id.to_lowercase().replace(' ', "-"),
                    }
                })
                .collect();
        }
        
        aggregate.last_updated = chrono::Utc::now().to_rfc3339();
        Ok(aggregate.clone())
    }

pub async fn query_rirs(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Rir>, NetBoxError> {
        let rirs = client.rirs.lock().unwrap();
        Ok(rirs.values().cloned().collect())
}

pub async fn get_rir_by_name(client: &MockNetBoxClient, name: &str) -> Result<Option<Rir>, NetBoxError> {
        Ok(client.rirs.lock().unwrap().get(name).cloned())
}

pub async fn create_rir(client: &MockNetBoxClient, name: &str, slug: Option<&str>, description: Option<String>, is_private: Option<bool>, tags: Option<Vec<String>>) -> Result<Rir, NetBoxError> {
        let id = client.next_id();
        let slug_value = slug.map(|s| s.to_string()).unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));
        let rir = Rir {
            id,
            url: format!("{}/api/ipam/rirs/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug_value,
            description,
            is_private: is_private.unwrap_or(false),
            tags: tags.unwrap_or_default().into_iter().map(|t| NestedTag {
                id: 0,
                url: String::new(),
                display: t.clone(),
                name: t,
                slug: String::new(),
            }).collect(),
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.rirs.lock().unwrap().insert(name.to_string(), rir.clone());
        Ok(rir)
    }

pub async fn update_rir(client: &MockNetBoxClient, id: u64, name: Option<&str>, slug: Option<&str>, description: Option<String>, is_private: Option<bool>, tags: Option<Vec<String>>) -> Result<Rir, NetBoxError> {
        let rirs = client.rirs.lock().unwrap();
        let rir = rirs.values().find(|r| r.id == id)
            .ok_or_else(|| NetBoxError::NotFound(format!("RIR {} not found", id)))?;
        let mut updated = rir.clone();
        
        if let Some(name_val) = name {
            updated.name = name_val.to_string();
            updated.display = name_val.to_string();
        }
        
        if let Some(slug_val) = slug {
            updated.slug = slug_val.to_string();
        }
        
        if description.is_some() {
            updated.description = description;
        }
        
        if is_private.is_some() {
            updated.is_private = is_private.unwrap_or(false);
        }
        
        if let Some(tags_vec) = tags {
            updated.tags = tags_vec.into_iter().map(|t| NestedTag {
                id: 0,
                url: String::new(),
                display: t.clone(),
                name: t,
                slug: String::new(),
            }).collect();
        }
        
        updated.last_updated = chrono::Utc::now().to_rfc3339();
        drop(rirs);
        client.rirs.lock().unwrap().insert(updated.name.clone(), updated.clone());
        Ok(updated)
    }

pub async fn create_vlan(client: &MockNetBoxClient, vid: u16, name: &str, site_id: Option<SiteId>, _group_id: Option<VlanGroupId>, tenant_id: Option<TenantId>, role_id: Option<RoleId>, status: Option<&str>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Vlan, NetBoxError> {
        let id = client.next_id();
        let status_str = status.unwrap_or("active");
        let vlan_status = match status_str {
            "active" => VlanStatus::Active,
            "reserved" => VlanStatus::Reserved,
            "deprecated" => VlanStatus::Deprecated,
            _ => VlanStatus::Active,
        };
        
        // Convert tag IDs to NestedTag
        let nested_tags: Vec<crate::models::NestedTag> = tags
            .unwrap_or_default()
            .iter()
            .map(|tag_id| {
                let tag_id_num = tag_id.parse::<u64>().unwrap_or(0);
                crate::models::NestedTag {
                    id: tag_id_num,
                    url: format!("{}/api/extras/tags/{}/", client.base_url, tag_id_num),
                    display: tag_id.clone(),
                    name: tag_id.clone(),
                    slug: tag_id.to_lowercase().replace(' ', "-"),
                }
            })
            .collect();
        
        let vlan = Vlan {
            id,
            url: format!("{}/api/ipam/vlans/{}/", client.base_url, id),
            display: name.to_string(),
            site: site_id.map(|id| client.helpers().create_nested_site(id.into(), None)),
            group: None, // VLAN group not yet implemented in mock helpers
            vid: vid as u16,
            name: name.to_string(),
            tenant: tenant_id.map(|id| client.helpers().create_nested_tenant(id.into(), None)),
            status: vlan_status,
            role: role_id.map(|id| client.helpers().create_nested_role(id.into(), None)),
            description: description.unwrap_or_default(),
            comments: comments.unwrap_or_default(),
            tags: nested_tags,
            custom_fields: serde_json::json!({}),
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.vlans.lock().unwrap().insert(id, vlan.clone());
        Ok(vlan)
    }

pub async fn update_vlan(client: &MockNetBoxClient, id: VlanId, vid: Option<u16>, name: Option<&str>, site_id: Option<SiteId>, _group_id: Option<VlanGroupId>, tenant_id: Option<TenantId>, role_id: Option<RoleId>, status: Option<&str>, description: Option<String>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Vlan, NetBoxError> {
        let id_value: u32 = id.into();
        let id_value_u64 = id_value as u64;
        let mut vlans = client.vlans.lock().unwrap();
        let vlan = vlans
            .get_mut(&id_value_u64)
            .ok_or_else(|| NetBoxError::NotFound(format!("VLAN {} not found", id_value_u64)))?;

        if let Some(vid_val) = vid {
            vlan.vid = vid_val;
        }
        if let Some(name_str) = name {
            vlan.name = name_str.to_string();
        }
        if let Some(site_id_val) = site_id {
            vlan.site = Some(client.helpers().create_nested_site(site_id_val.into(), None));
        }
        if let Some(tenant) = tenant_id {
            vlan.tenant = Some(client.helpers().create_nested_tenant(tenant.into(), None));
        }
        if let Some(role) = role_id {
            vlan.role = Some(client.helpers().create_nested_role(role.into(), None));
        }
        if let Some(status_str) = status {
            vlan.status = match status_str {
                "active" => VlanStatus::Active,
                "reserved" => VlanStatus::Reserved,
                "deprecated" => VlanStatus::Deprecated,
                _ => VlanStatus::Active,
            };
        }
        if let Some(desc) = description {
            vlan.description = desc;
        }
        if let Some(comments_str) = comments {
            vlan.comments = comments_str;
        }
        if tags.is_some() {
            // Convert tag IDs to NestedTag
            vlan.tags = tags
                .unwrap_or_default()
                .iter()
                .map(|tag_id| {
                    let tag_id_num = tag_id.parse::<u64>().unwrap_or(0);
                    crate::models::NestedTag {
                        id: tag_id_num,
                        url: format!("{}/api/extras/tags/{}/", client.base_url, tag_id_num),
                        display: tag_id.clone(),
                        name: tag_id.clone(),
                        slug: tag_id.to_lowercase().replace(' ', "-"),
                    }
                })
                .collect();
        }

        Ok(vlan.clone())
    }

pub async fn query_vlans(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Vlan>, NetBoxError> {
        let vlans = client.vlans.lock().unwrap();
        Ok(vlans.values().cloned().collect())
}

pub async fn get_vlan(client: &MockNetBoxClient, id: VlanId) -> Result<Vlan, NetBoxError> {
        let id_value: u32 = id.into();
        let id_value_u64 = id_value as u64;
        client.vlans
            .lock()
            .unwrap()
            .get(&id_value_u64)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("VLAN {} not found", id_value_u64)))
}
