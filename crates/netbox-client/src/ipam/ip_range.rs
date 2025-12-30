//! IPAM IP Range operations
//!
//! This module provides methods for managing NetBox IPAM IP ranges.
//! IP ranges represent contiguous sequences of IP addresses, commonly used for DHCP pools.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::{IPRange, IPRangeStatus};
use crate::types::*;
use ipnet::IpNet;
use tracing::debug;

/// Get an IP range by ID
pub async fn get_ip_range(core: &NetBoxClientCore, id: IPRangeId) -> Result<IPRange, NetBoxError> {
    let id_value: u64 = id.into();
    let url = format!("{}/api/ipam/ip-ranges/{}/", core.base_url, id_value);
    debug!("Fetching IP range {} from NetBox", id_value);
    
    let response = core.client
        .get(&url)
        .header("Authorization", format!("Token {}", core.token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| NetBoxError::Http(e))?;
    
    if response.status() == 404 {
        return Err(NetBoxError::NotFound(format!("IP range {} not found", id_value)));
    }
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to get IP range {}: {} - {}",
            id_value, status, body
        )));
    }
    
    let ip_range: IPRange = response.json().await?;
    Ok(ip_range)
}

/// Query IP ranges by filters
pub async fn query_ip_ranges(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<IPRange>, NetBoxError> {
    let mut url = format!("{}/api/ipam/ip-ranges/", core.base_url);
    
    if !filters.is_empty() {
        let query: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        url = format!("{}?{}", url, query.join("&"));
    }
    
    debug!("Querying IP ranges with filters: {:?}", filters);
    
    if fetch_all {
        core.fetch_all_pages(url).await
    } else {
        let response = core.client
            .get(&url)
            .header("Authorization", format!("Token {}", core.token))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        let status = response.status();
        let response_body = response.text().await.unwrap_or_default();
        
        if !status.is_success() {
            return Err(NetBoxError::Api(format!(
                "Failed to query IP ranges: {} - {}",
                status, response_body
            )));
        }
        
        // Parse JSON response, handling empty responses gracefully
        if response_body.is_empty() {
            debug!("Empty response body for IP range query, returning empty list");
            return Ok(Vec::new());
        }
        
        let result: PaginatedResponse<IPRange> = serde_json::from_str(&response_body)
            .map_err(|e| NetBoxError::Serialization(e))?;
        Ok(result.results)
    }
}

/// Create a new IP range
pub async fn create_ip_range(
    core: &NetBoxClientCore,
    start_address: &IpNet,
    end_address: &IpNet,
    vrf_id: Option<u64>,
    tenant_id: Option<TenantId>,
    role_id: Option<RoleId>,
    status: Option<IPRangeStatus>,
    description: Option<String>,
    mark_utilized: Option<bool>,
    mark_populated: Option<bool>,
    tags: Option<Vec<String>>,
) -> Result<IPRange, NetBoxError> {
    let start_str = start_address.to_string();
    let end_str = end_address.to_string();
    let mut body = serde_json::json!({
        "start_address": start_str,
        "end_address": end_str,
    });
    
    helpers::add_nested_reference(&mut body, "vrf", vrf_id);
    if let Some(tenant_id) = tenant_id {
        body["tenant"] = serde_json::json!(tenant_id.0);
    }
    if let Some(role_id) = role_id {
        body["role"] = serde_json::json!(role_id.0);
    }
    if let Some(status) = status {
        body["status"] = serde_json::to_value(status)
            .map_err(|e| NetBoxError::Serialization(e))?;
    }
    if let Some(desc) = description {
        body["description"] = serde_json::Value::String(desc);
    }
    if let Some(utilized) = mark_utilized {
        body["mark_utilized"] = serde_json::Value::Bool(utilized);
    }
    if let Some(populated) = mark_populated {
        body["mark_populated"] = serde_json::Value::Bool(populated);
    }
    if let Some(tags) = tags {
        body["tags"] = serde_json::to_value(tags)
            .map_err(|e| NetBoxError::Serialization(e))?;
    }
    
    let url = format!("{}/api/ipam/ip-ranges/", core.base_url);
    debug!("Creating IP range: {} - {}", start_str, end_str);
    
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
            "Failed to create IP range: {} - {}",
            status, body
        )));
    }
    
    let ip_range: IPRange = response.json().await
        .map_err(|e| NetBoxError::Http(e))?;
    Ok(ip_range)
}

/// Update an existing IP range
pub async fn update_ip_range(
    core: &NetBoxClientCore,
    id: IPRangeId,
    start_address: Option<&IpNet>,
    end_address: Option<&IpNet>,
    vrf_id: Option<u64>,
    tenant_id: Option<TenantId>,
    role_id: Option<RoleId>,
    status: Option<IPRangeStatus>,
    description: Option<String>,
    mark_utilized: Option<bool>,
    mark_populated: Option<bool>,
    tags: Option<Vec<String>>,
) -> Result<IPRange, NetBoxError> {
    let mut body = serde_json::json!({});
    
    if let Some(start) = start_address {
        body["start_address"] = serde_json::Value::String(start.to_string());
    }
    if let Some(end) = end_address {
        body["end_address"] = serde_json::Value::String(end.to_string());
    }
    helpers::add_nested_reference(&mut body, "vrf", vrf_id);
    if let Some(tenant_id) = tenant_id {
        body["tenant"] = serde_json::json!(tenant_id.0);
    }
    if let Some(role_id) = role_id {
        body["role"] = serde_json::json!(role_id.0);
    }
    if let Some(status) = status {
        body["status"] = serde_json::to_value(status)
            .map_err(|e| NetBoxError::Serialization(e))?;
    }
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    if let Some(utilized) = mark_utilized {
        body["mark_utilized"] = serde_json::Value::Bool(utilized);
    }
    if let Some(populated) = mark_populated {
        body["mark_populated"] = serde_json::Value::Bool(populated);
    }
    if let Some(tags) = tags {
        body["tags"] = serde_json::to_value(tags)
            .map_err(|e| NetBoxError::Serialization(e))?;
    }
    
    let id_value: u64 = id.into();
    let url = format!("{}/api/ipam/ip-ranges/{}/", core.base_url, id_value);
    debug!("Updating IP range: {}", id_value);
    
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
            "Failed to update IP range {}: {} - {}",
            id_value, status, body
        )));
    }
    
    let ip_range: IPRange = response.json().await
        .map_err(|e| NetBoxError::Http(e))?;
    Ok(ip_range)
}

/// Delete an IP range
pub async fn delete_ip_range(core: &NetBoxClientCore, id: IPRangeId) -> Result<(), NetBoxError> {
    let id_value: u64 = id.into();
    let url = format!("{}/api/ipam/ip-ranges/{}/", core.base_url, id_value);
    debug!("Deleting IP range: {}", id_value);
    
    let response = core.client
        .delete(&url)
        .header("Authorization", format!("Token {}", core.token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| NetBoxError::Http(e))?;
    
    if response.status() == 404 {
        return Err(NetBoxError::NotFound(format!("IP range {} not found", id_value)));
    }
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to delete IP range {}: {} - {}",
            id_value, status, body
        )));
    }
    
    Ok(())
}

