//! DCIM MAC Address operations
//!
//! This module provides methods for managing NetBox DCIM MAC addresses.

use crate::common::PaginatedResponse;
use crate::core::NetBoxClientCore;
use crate::error::NetBoxError;
use crate::models::MACAddress;
use tracing::debug;

/// Query MAC addresses by filters
pub async fn query_mac_addresses(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<MACAddress>, NetBoxError> {
    let mut url = format!("{}/api/dcim/mac-addresses/", core.base_url);
    
    if !filters.is_empty() {
        let query: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        url = format!("{}?{}", url, query.join("&"));
    }
    
    debug!("Querying MAC addresses with filters: {:?}", filters);
    
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
            // Check if response is HTML (404 page) vs JSON error
            if body.trim_start().starts_with("<!DOCTYPE") || body.trim_start().starts_with("<html") {
                return Err(NetBoxError::NotFound(format!(
                    "MAC addresses endpoint not found (404): {}",
                    status
                )));
            }
            return Err(NetBoxError::Api(format!(
                "Failed to query MAC addresses: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<MACAddress> = response.json().await?;
        Ok(result.results)
    }
}

/// Get MAC address by address
pub async fn get_mac_address_by_address(
    core: &NetBoxClientCore,
    mac: &str,
) -> Result<Option<MACAddress>, NetBoxError> {
    let mac_addresses = query_mac_addresses(core, &[("mac_address", mac)], false).await?;
    Ok(mac_addresses.first().cloned())
}

/// Create a new MAC address
pub async fn create_mac_address(
    core: &NetBoxClientCore,
    mac_address: &str,
    assigned_object_type: &str, // e.g., "dcim.interface"
    assigned_object_id: u64,
    description: Option<String>,
    comments: Option<String>,
) -> Result<MACAddress, NetBoxError> {
    let url = format!("{}/api/dcim/mac-addresses/", core.base_url);
    debug!("Creating MAC address {} in NetBox", mac_address);
    
    let mut body = serde_json::json!({
        "mac_address": mac_address,
        "assigned_object_type": assigned_object_type,
        "assigned_object_id": assigned_object_id,
    });
    
    if let Some(desc) = description {
        body["description"] = serde_json::Value::String(desc);
    }
    
    if let Some(comments_str) = comments {
        body["comments"] = serde_json::Value::String(comments_str);
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
        // Check if response is HTML (404 page) vs JSON error
        if body.trim_start().starts_with("<!DOCTYPE") || body.trim_start().starts_with("<html") {
            return Err(NetBoxError::NotFound(format!(
                "MAC addresses endpoint not found (404): {}",
                status
            )));
        }
        return Err(NetBoxError::Api(format!(
            "Failed to create MAC address: {} - {}",
            status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

