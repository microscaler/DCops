//! IPAM operations for MockNetBoxClient
//!
//! Handles prefixes, IP addresses, aggregates, RIRs, and VLANs

use super::MockNetBoxClient;
use crate::error::NetBoxError;
use crate::models::*;

pub async fn get_prefix(client: &MockNetBoxClient, id: u64) -> Result<Prefix, NetBoxError> {
        client.prefixes
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Prefix {} not found", id)))
}

pub async fn get_available_ips(client: &MockNetBoxClient, prefix_id: u64, _limit: Option<u32>) -> Result<Vec<AvailableIP>, NetBoxError> {
        Ok(client.available_ips
            .lock()
            .unwrap()
            .get(&prefix_id)
            .cloned()
            .unwrap_or_default())
}

pub async fn allocate_ip(client: &MockNetBoxClient, prefix_id: u64, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError> {
        // Verify prefix exists
        get_prefix(client, prefix_id).await?;

        let id = client.next_id();
        let address = request
            .as_ref()
            .and_then(|r| r.address.clone())
            .unwrap_or_else(|| format!("192.168.1.{}", id));

        let ip = IPAddress {
            id,
            url: format!("{}/api/ipam/ip-addresses/{}/", client.base_url, id),
            display: address.clone(),
            family: 4, // Default to IPv4
            address: address.clone(),
            vrf: None,
            tenant: None, // AllocateIPRequest doesn't have tenant field
            status: request
                .as_ref()
                .and_then(|r| r.status.clone())
                .unwrap_or(IPAddressStatus::Active),
            role: request.as_ref().and_then(|r| r.role.clone()),
            assigned_object_type: None,
            assigned_object_id: None,
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

pub async fn get_ip_address(client: &MockNetBoxClient, id: u64) -> Result<IPAddress, NetBoxError> {
        client.ip_addresses
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("IP address {} not found", id)))
}

pub async fn query_ip_addresses(client: &MockNetBoxClient, filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<IPAddress>, NetBoxError> {
        let ips = client.ip_addresses.lock().unwrap();
        let mut results: Vec<IPAddress> = ips.values().cloned().collect();

        // Apply filters (simplified - only handles prefix filter)
        for (key, value) in filters {
            if *key == "prefix" {
                results.retain(|ip| ip.address.starts_with(value));
            }
        }

        Ok(results)
}

pub async fn query_prefixes(client: &MockNetBoxClient, filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Prefix>, NetBoxError> {
        let prefixes = client.prefixes.lock().unwrap();
        Ok(prefixes.values().cloned().collect())
}

pub async fn create_ip_address(client: &MockNetBoxClient, address: &str, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError> {
        let id = client.next_id();
        let ip = IPAddress {
            id,
            url: format!("{}/api/ipam/ip-addresses/{}/", client.base_url, id),
            display: address.to_string(),
            family: if address.contains(':') { 6 } else { 4 },
            address: address.to_string(),
            vrf: None,
            tenant: None, // AllocateIPRequest doesn't have tenant field
            status: request
                .as_ref()
                .and_then(|r| r.status.clone())
                .unwrap_or(IPAddressStatus::Active),
            role: request.as_ref().and_then(|r| r.role.clone()),
            assigned_object_type: None,
            assigned_object_id: None,
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

pub async fn update_ip_address(client: &MockNetBoxClient, id: u64, request: AllocateIPRequest) -> Result<IPAddress, NetBoxError> {
        let mut ips = client.ip_addresses.lock().unwrap();
        let ip = ips
            .get_mut(&id)
            .ok_or_else(|| NetBoxError::NotFound(format!("IP address {} not found", id)))?;

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

pub async fn create_prefix(client: &MockNetBoxClient, prefix: &str, site_id: Option<u64>, tenant_id: Option<u64>, vlan_id: Option<u32>, role_id: Option<u64>, status: Option<&str>, description: Option<&str>, tags: Option<Vec<serde_json::Value>>) -> Result<Prefix, NetBoxError> {
        let id = client.next_id();
        let status_str = status.unwrap_or("active");
        let prefix_status = match status_str {
            "active" => PrefixStatus::Active,
            "reserved" => PrefixStatus::Reserved,
            "deprecated" => PrefixStatus::Deprecated,
            "container" => PrefixStatus::Container,
            _ => PrefixStatus::Active,
        };
        
        let tags_vec: Vec<NestedTag> = tags
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_str().map(|s| NestedTag {
                id: 0,
                url: format!("{}/api/extras/tags/{}/", client.base_url, 0),
                display: s.to_string(),
                name: s.to_string(),
                slug: s.to_lowercase().replace(' ', "-"),
            }))
            .collect();
        
        let prefix_obj = Prefix {
            id,
            url: format!("{}/api/ipam/prefixes/{}/", client.base_url, id),
            display: prefix.to_string(),
            family: if prefix.contains(':') { 6 } else { 4 },
            prefix: prefix.to_string(),
            vrf: None,
            tenant: tenant_id.map(|id| client.helpers().create_nested_tenant(id, None)),
            vlan: vlan_id.map(|id| client.helpers().create_nested_vlan(id as u64, id as u16, None)),
            status: prefix_status,
            role: role_id.map(|id| client.helpers().create_nested_role(id, None)),
            is_pool: false,
            mark_utilized: false,
            description: description.map(|s| s.to_string()).unwrap_or_default(),
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

pub async fn update_prefix(client: &MockNetBoxClient, id: u64, site_id: Option<u64>, tenant_id: Option<u64>, vlan_id: Option<u32>, role_id: Option<u64>, status: Option<&str>, description: Option<&str>, tags: Option<Vec<serde_json::Value>>) -> Result<Prefix, NetBoxError> {
        let mut prefixes = client.prefixes.lock().unwrap();
        let prefix = prefixes
            .get_mut(&id)
            .ok_or_else(|| NetBoxError::NotFound(format!("Prefix {} not found", id)))?;

        // Prefix model doesn't have a site field - skip
        if let Some(tenant) = tenant_id {
            prefix.tenant = Some(client.helpers().create_nested_tenant(tenant, None));
        }
        if let Some(vlan) = vlan_id {
            prefix.vlan = Some(client.helpers().create_nested_vlan(vlan as u64, vlan as u16, None));
        }
        if let Some(role) = role_id {
            prefix.role = Some(client.helpers().create_nested_role(role, None));
        }
        if let Some(status_str) = status {
            prefix.status = match status_str {
                "active" => PrefixStatus::Active,
                "reserved" => PrefixStatus::Reserved,
                "deprecated" => PrefixStatus::Deprecated,
                "container" => PrefixStatus::Container,
                _ => PrefixStatus::Active,
            };
        }
        if let Some(desc) = description {
            prefix.description = desc.to_string();
        }
        if let Some(tags_val) = tags {
            prefix.tags = tags_val.into_iter()
                .filter_map(|v| v.as_str().map(|s| NestedTag {
                    id: 0,
                    url: format!("{}/api/extras/tags/0/", client.base_url),
                    display: s.to_string(),
                    name: s.to_string(),
                    slug: s.to_lowercase().replace(' ', "-"),
                }))
                .collect();
        }

        Ok(prefix.clone())
}

pub async fn query_aggregates(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Aggregate>, NetBoxError> {
        let aggregates = client.aggregates.lock().unwrap();
        Ok(aggregates.values().cloned().collect())
}

pub async fn get_aggregate(client: &MockNetBoxClient, id: u64) -> Result<Aggregate, NetBoxError> {
        client.aggregates
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Aggregate {} not found", id)))
}

pub async fn create_aggregate(client: &MockNetBoxClient, prefix: &str, rir_id: u64, description: Option<&str>) -> Result<Aggregate, NetBoxError> {
        let id = client.next_id();
        let aggregate = Aggregate {
            id,
            url: format!("{}/api/ipam/aggregates/{}/", client.base_url, id),
            display: prefix.to_string(),
            prefix: prefix.to_string(),
            rir: Some(NestedRir {
                id: rir_id,
                url: format!("{}/api/ipam/rirs/{}/", client.base_url, rir_id),
                display: format!("RIR {}", rir_id),
                name: format!("RIR {}", rir_id),
                slug: format!("rir-{}", rir_id),
            }),
            date_allocated: None,
            description: description.map(|s| s.to_string()),
            comments: None,
            tags: vec![],
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.aggregates.lock().unwrap().insert(id, aggregate.clone());
        Ok(aggregate)
}

pub async fn query_rirs(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Rir>, NetBoxError> {
        let rirs = client.rirs.lock().unwrap();
        Ok(rirs.values().cloned().collect())
}

pub async fn get_rir_by_name(client: &MockNetBoxClient, name: &str) -> Result<Option<Rir>, NetBoxError> {
        Ok(client.rirs.lock().unwrap().get(name).cloned())
}

pub async fn create_rir(client: &MockNetBoxClient, name: &str, slug: &str, description: Option<&str>) -> Result<Rir, NetBoxError> {
        let id = client.next_id();
        let rir = Rir {
            id,
            url: format!("{}/api/ipam/rirs/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug.to_string(),
            description: description.map(|s| s.to_string()),
            is_private: false,
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.rirs.lock().unwrap().insert(name.to_string(), rir.clone());
        Ok(rir)
}

pub async fn create_vlan(client: &MockNetBoxClient, site_id: u64, vid: u32, name: &str, status: Option<&str>, description: Option<&str>) -> Result<Vlan, NetBoxError> {
        let id = client.next_id();
        let status_str = status.unwrap_or("active");
        let vlan_status = match status_str {
            "active" => VlanStatus::Active,
            "reserved" => VlanStatus::Reserved,
            "deprecated" => VlanStatus::Deprecated,
            _ => VlanStatus::Active,
        };
        
        let vlan = Vlan {
            id,
            url: format!("{}/api/ipam/vlans/{}/", client.base_url, id),
            display: name.to_string(),
            site: Some(client.helpers().create_nested_site(site_id, None)),
            group: None,
            vid: vid as u16,
            name: name.to_string(),
            tenant: None,
            status: vlan_status,
            role: None,
            description: description.map(|s| s.to_string()).unwrap_or_default(),
            comments: String::new(),
            tags: vec![],
            custom_fields: serde_json::json!({}),
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.vlans.lock().unwrap().insert(id, vlan.clone());
        Ok(vlan)
}

pub async fn update_vlan(client: &MockNetBoxClient, id: u64, site_id: Option<u64>, vid: Option<u32>, name: Option<&str>, status: Option<&str>, description: Option<&str>) -> Result<Vlan, NetBoxError> {
        let mut vlans = client.vlans.lock().unwrap();
        let vlan = vlans
            .get_mut(&id)
            .ok_or_else(|| NetBoxError::NotFound(format!("VLAN {} not found", id)))?;

        if let Some(site_id_val) = site_id {
            vlan.site = Some(client.helpers().create_nested_site(site_id_val, None));
        }
        if let Some(vid_val) = vid {
            vlan.vid = vid_val as u16;
        }
        if let Some(name_str) = name {
            vlan.name = name_str.to_string();
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
            vlan.description = desc.to_string();
        }

        Ok(vlan.clone())
}

pub async fn query_vlans(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Vlan>, NetBoxError> {
        let vlans = client.vlans.lock().unwrap();
        Ok(vlans.values().cloned().collect())
}

pub async fn get_vlan(client: &MockNetBoxClient, id: u64) -> Result<Vlan, NetBoxError> {
        client.vlans
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("VLAN {} not found", id)))
}
