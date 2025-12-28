//! IPAM Prefix operations
//!
//! This module provides methods for managing NetBox IPAM prefixes.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::{AvailableIP, Prefix};
use crate::types::*;
use tracing::debug;

/// Get a prefix by ID
pub async fn get_prefix(core: &NetBoxClientCore, id: PrefixId) -> Result<Prefix, NetBoxError> {
    let id_value: u64 = id.into();
    let url = format!("{}/api/ipam/prefixes/{}/", core.base_url, id_value);
    debug!("Fetching prefix {} from NetBox", id_value);
    
    let response = core.client
        .get(&url)
        .header("Authorization", format!("Token {}", core.token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| NetBoxError::Http(e))?;
    
    if response.status() == 404 {
        return Err(NetBoxError::NotFound(format!("Prefix {} not found", id_value)));
    }
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to get prefix {}: {} - {}",
            id_value, status, body
        )));
    }
    
    let prefix: Prefix = response.json().await?;
    Ok(prefix)
}

/// Query prefixes by filters
pub async fn query_prefixes(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<Prefix>, NetBoxError> {
    let mut url = format!("{}/api/ipam/prefixes/", core.base_url);
    
    if !filters.is_empty() {
        let query: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        url = format!("{}?{}", url, query.join("&"));
    }
    
    debug!("Querying prefixes with filters: {:?}", filters);
    
    if fetch_all {
        core.fetch_all_pages(url).await
    } else {
        let response = core.client
            .get(&url)
            .header("Authorization", format!("Token {}", core.token))
            .header("Accept", "application/json")
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to query prefixes: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<Prefix> = response.json().await?;
        Ok(result.results)
    }
}

/// Get available IP addresses from a prefix
pub async fn get_available_ips(
    core: &NetBoxClientCore,
    prefix_id: PrefixId,
    limit: Option<u32>,
) -> Result<Vec<AvailableIP>, NetBoxError> {
    let prefix_id_value: u64 = prefix_id.into();
    let mut url = format!("{}/api/ipam/prefixes/{}/available-ips/", core.base_url, prefix_id_value);
    if let Some(limit) = limit {
        url = format!("{}?limit={}", url, limit);
    }
    
    debug!("Fetching available IPs from prefix {}", prefix_id);
    
    let response = core.client
        .get(&url)
        .header("Authorization", format!("Token {}", core.token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| NetBoxError::Http(e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to get available IPs from prefix {}: {} - {}",
            prefix_id_value, status, body
        )));
    }
    
    let ips: Vec<AvailableIP> = response.json().await
        .map_err(|e| NetBoxError::Http(e))?;
    Ok(ips)
}

/// Create a new prefix in NetBox
pub async fn create_prefix(
    core: &NetBoxClientCore,
    prefix: &str,
    description: Option<String>,
    site_id: Option<u64>,
    vlan_id: Option<u32>,
    status: Option<&str>,
    role_id: Option<u64>,
    tenant_id: Option<u64>,
    tags: Option<Vec<String>>,
) -> Result<Prefix, NetBoxError> {
    let url = format!("{}/api/ipam/prefixes/", core.base_url);
    debug!("Creating prefix {} in NetBox", prefix);
    
    let mut body = serde_json::json!({
        "prefix": prefix,
    });
    
    if let Some(desc) = description {
        body["description"] = serde_json::Value::String(desc);
    }
    
    if let Some(status_str) = status {
        body["status"] = serde_json::Value::String(status_str.to_string());
    }
    
    // For CREATE operations, NetBox 4.0 requires full tenant object (id, name, slug)
    helpers::add_nested_reference(&mut body, "site", site_id.map(|id| id.into()));
    helpers::add_nested_reference(&mut body, "vlan", vlan_id.map(|id| id as u64));
    helpers::add_nested_reference(&mut body, "role", role_id.map(|id| id.into()));
    // For CREATE operations, NetBox 4.0 requires full tenant object (id, name, slug)
    helpers::add_tenant_for_create(&mut body, core, tenant_id).await;
    
    if let Some(tags_vec) = tags {
        body["tags"] = serde_json::to_value(tags_vec)
            .map_err(|e| NetBoxError::Serialization(e))?;
    }
    
    let response = core.client
        .post(&url)
        .header("Authorization", format!("Token {}", core.token))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| NetBoxError::Http(e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to create prefix {}: {} - {}",
            prefix, status, body
        )));
    }
    
    let prefix_obj: Prefix = response.json().await
        .map_err(|e| NetBoxError::Http(e))?;
    Ok(prefix_obj)
}

/// Update an existing prefix in NetBox
pub async fn update_prefix(
    core: &NetBoxClientCore,
    id: PrefixId,
    prefix: Option<&str>,
    description: Option<String>,
    status: Option<&str>,
    role: Option<String>,
    tenant_id: Option<TenantId>,
    site_id: Option<SiteId>,
    vlan_id: Option<VlanId>,
    tags: Option<Vec<String>>,
) -> Result<Prefix, NetBoxError> {
    let id_value: u64 = id.into();
    let url = format!("{}/api/ipam/prefixes/{}/", core.base_url, id_value);
    debug!("Updating prefix {} in NetBox", id_value);
    
    let mut body = serde_json::json!({});
    
    if let Some(prefix_str) = prefix {
        body["prefix"] = serde_json::Value::String(prefix_str.to_string());
    }
    
    if let Some(desc) = description {
        body["description"] = serde_json::Value::String(desc);
    }
    
    if let Some(status_str) = status {
        body["status"] = serde_json::Value::String(status_str.to_string());
    }
    
    helpers::add_optional_string_field_owned(&mut body, "role", role);
    
    // NetBox 4.0 PATCH updates: For nested objects, send only {"id": X}
    // Sending the full object causes NetBox to try to CREATE a new object
    helpers::add_nested_reference(&mut body, "tenant", tenant_id.map(|id| id.into()));
    helpers::add_nested_reference(&mut body, "site", site_id.map(|id| id.into()));
    helpers::add_nested_reference(&mut body, "vlan", vlan_id.map(|id: VlanId| <VlanId as Into<u32>>::into(id) as u64));
    
    if let Some(tags_vec) = tags {
        body["tags"] = serde_json::to_value(tags_vec)
            .map_err(|e| NetBoxError::Serialization(e))?;
    }
    
    let response = core.client
        .patch(&url)
        .header("Authorization", format!("Token {}", core.token))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| NetBoxError::Http(e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to update prefix {}: {} - {}",
            id_value, status, body
        )));
    }
    
    let prefix_obj: Prefix = response.json().await
        .map_err(|e| NetBoxError::Http(e))?;
    Ok(prefix_obj)
}

