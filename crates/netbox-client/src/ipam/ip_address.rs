//! IPAM IP Address operations
//!
//! This module provides methods for managing NetBox IPAM IP addresses.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
#[allow(unused_imports)] // Used in allocate_ip via get_available_ips return type
use crate::models::{AllocateIPRequest, AvailableIP, IPAddress};
use crate::ipam::prefix::get_available_ips;
use crate::types::*;
use tracing::debug;

/// Get an IP address by ID
pub async fn get_ip_address(core: &NetBoxClientCore, id: IpAddressId) -> Result<IPAddress, NetBoxError> {
    let id_value: u64 = id.into();
    let url = format!("{}/api/ipam/ip-addresses/{}/", core.base_url, id_value);
    debug!("Fetching IP address {} from NetBox", id_value);
    
    let response = core.client
        .get(&url)
        .header("Authorization", format!("Token {}", core.token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| NetBoxError::Http(e))?;
    
    if response.status() == 404 {
        return Err(NetBoxError::NotFound(format!("IP address {} not found", id_value)));
    }
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to get IP address {}: {} - {}",
            id_value, status, body
        )));
    }
    
    let ip: IPAddress = response.json().await
        .map_err(|e| NetBoxError::Http(e))?;
    Ok(ip)
}

/// Query IP addresses by filter
pub async fn query_ip_addresses(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<IPAddress>, NetBoxError> {
    let mut url = format!("{}/api/ipam/ip-addresses/", core.base_url);
    
    // Build query string
    if !filters.is_empty() {
        let query: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        url = format!("{}?{}", url, query.join("&"));
    }
    
    debug!("Querying IP addresses with filters: {:?}", filters);
    
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
                "Failed to query IP addresses: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<IPAddress> = response.json().await?;
        Ok(result.results)
    }
}

/// Create a new IP address
pub async fn create_ip_address(
    core: &NetBoxClientCore,
    address: &ipnet::IpNet,
    request: Option<AllocateIPRequest>,
) -> Result<IPAddress, NetBoxError> {
    let address_str = address.to_string();
    let mut body = serde_json::json!({
        "address": address_str,
    });
    
    if let Some(req) = request {
        if let Some(desc) = req.description {
            body["description"] = serde_json::Value::String(desc);
        }
        if let Some(comments) = req.comments {
            body["comments"] = serde_json::Value::String(comments);
        }
        if let Some(status) = req.status {
            body["status"] = serde_json::to_value(status)
                .map_err(|e| NetBoxError::Serialization(e))?;
        }
        if let Some(role) = req.role {
            body["role"] = serde_json::Value::String(role);
        }
        if let Some(dns_name) = req.dns_name {
            body["dns_name"] = serde_json::Value::String(dns_name);
        }
        // Use nested reference helper like Prefix does - NetBox requires {"id": X} not just X
        helpers::add_nested_reference(&mut body, "tenant", req.tenant);
        helpers::add_optional_tags_field(&mut body, req.tags)?;
        
        // Add assigned object (interface assignment)
        if let Some(obj_type) = &req.assigned_object_type {
            body["assigned_object_type"] = serde_json::Value::String(obj_type.clone());
        }
        if let Some(obj_id) = req.assigned_object_id {
            body["assigned_object_id"] = serde_json::json!(obj_id);
        }
    }
    
    let url = format!("{}/api/ipam/ip-addresses/", core.base_url);
    debug!("Creating IP address: {}", address);
    
    let response = core.client
        .post(&url)
        .header("Authorization", format!("Token {}", core.token))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| NetBoxError::Http(e))?;
    
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to create IP address: {} - {}",
            status, body
        )));
    }
    
    // Try to decode the response, with better error handling
    let response_text = response.text().await
        .map_err(|e| NetBoxError::Http(e))?;
    
    let ip: IPAddress = serde_json::from_str(&response_text)
        .map_err(|e| {
            NetBoxError::Api(format!(
                "Failed to decode IP address response: {} - Response body: {}",
                e, response_text
            ))
        })?;
    Ok(ip)
}

/// Update an existing IP address
pub async fn update_ip_address(
    core: &NetBoxClientCore,
    id: u64,
    request: AllocateIPRequest,
) -> Result<IPAddress, NetBoxError> {
    let mut body = serde_json::json!({});
    
    helpers::add_optional_string_field_owned(&mut body, "description", request.description);
    helpers::add_optional_string_field_owned(&mut body, "comments", request.comments);
    helpers::add_optional_enum_field(&mut body, "status", request.status)?;
    helpers::add_optional_string_field(&mut body, "role", request.role.as_deref());
    helpers::add_optional_string_field(&mut body, "dns_name", request.dns_name.as_deref());
    // Use nested reference helper like Prefix does - NetBox requires {"id": X} not just X
    helpers::add_nested_reference(&mut body, "tenant", request.tenant);
    helpers::add_optional_enum_field(&mut body, "tags", request.tags)?;
    
    // Add assigned object (interface assignment)
    if let Some(obj_type) = &request.assigned_object_type {
        body["assigned_object_type"] = serde_json::Value::String(obj_type.clone());
    }
    if let Some(obj_id) = request.assigned_object_id {
        body["assigned_object_id"] = serde_json::json!(obj_id);
    }
    
    let url = format!("{}/api/ipam/ip-addresses/{}/", core.base_url, id);
    debug!("Updating IP address: {}", id);
    
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
            "Failed to update IP address {}: {} - {}",
            id, status, body
        )));
    }
    
    let ip: IPAddress = response.json().await
        .map_err(|e| NetBoxError::Http(e))?;
    Ok(ip)
}

/// Delete an IP address
pub async fn delete_ip_address(core: &NetBoxClientCore, id: u64) -> Result<(), NetBoxError> {
    let url = format!("{}/api/ipam/ip-addresses/{}/", core.base_url, id);
    debug!("Deleting IP address: {}", id);
    
    let response = core.client
        .delete(&url)
        .header("Authorization", format!("Token {}", core.token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| NetBoxError::Http(e))?;
    
    if response.status() == 404 {
        return Err(NetBoxError::NotFound(format!("IP address {} not found", id)));
    }
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to delete IP address {}: {} - {}",
            id, status, body
        )));
    }
    
    Ok(())
}

/// Allocate an IP address from a prefix
///
/// This method:
/// 1. Gets available IPs from the prefix
/// 2. Takes the first available IP
/// 3. Creates an IPAddress object in NetBox
/// 4. Returns the allocated IP
pub async fn allocate_ip(
    core: &NetBoxClientCore,
    prefix_id: PrefixId,
    request: Option<AllocateIPRequest>,
) -> Result<IPAddress, NetBoxError> {
    let prefix_id_value: u64 = prefix_id.into();
    // Get available IPs
    let available_ips = get_available_ips(core, prefix_id, Some(1)).await?;
    
    if available_ips.is_empty() {
        return Err(NetBoxError::Api(format!(
            "No available IPs in prefix {}",
            prefix_id_value
        )));
    }
    
    let available_ip = &available_ips[0];
    
    // Build request body
    let mut body = serde_json::json!({
        "address": available_ip.address.to_string(),
    });
    
    if let Some(req) = request {
        if let Some(desc) = req.description {
            body["description"] = serde_json::Value::String(desc);
        }
        if let Some(comments) = req.comments {
            body["comments"] = serde_json::Value::String(comments);
        }
        if let Some(status) = req.status {
            body["status"] = serde_json::to_value(status)
                .map_err(|e| NetBoxError::Serialization(e))?;
        }
        if let Some(role) = req.role {
            body["role"] = serde_json::Value::String(role);
        }
        if let Some(dns_name) = req.dns_name {
            body["dns_name"] = serde_json::Value::String(dns_name);
        }
        helpers::add_optional_tags_field(&mut body, req.tags)?;
        
        // Add assigned object (interface assignment)
        if let Some(obj_type) = &req.assigned_object_type {
            body["assigned_object_type"] = serde_json::Value::String(obj_type.clone());
        }
        if let Some(obj_id) = req.assigned_object_id {
            body["assigned_object_id"] = serde_json::json!(obj_id);
        }
    }
    
    // Create IP address via POST to available-ips endpoint
    let url = format!("{}/api/ipam/prefixes/{}/available-ips/", core.base_url, prefix_id_value);
    debug!("Allocating IP {} from prefix {}", available_ip.address, prefix_id_value);
    
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
            "Failed to allocate IP from prefix {}: {} - {}",
            prefix_id_value, status, body
        )));
    }
    
    // NetBox returns an array of created IP addresses
    let created_ips: Vec<IPAddress> = response.json().await
        .map_err(|e| NetBoxError::Http(e))?;
    
    if created_ips.is_empty() {
        return Err(NetBoxError::Api("No IP address was created".to_string()));
    }
    
    Ok(created_ips[0].clone())
}

