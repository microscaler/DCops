//! DCIM Interface operations
//!
//! This module provides methods for managing NetBox DCIM interfaces.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::Interface;
use tracing::debug;

/// Query interfaces by filters
pub async fn query_interfaces(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<Interface>, NetBoxError> {
    let mut url = format!("{}/api/dcim/interfaces/", core.base_url);
    
    if !filters.is_empty() {
        let query: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        url = format!("{}?{}", url, query.join("&"));
    }
    
    debug!("Querying interfaces with filters: {:?}", filters);
    
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
                "Failed to query interfaces: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<Interface> = response.json().await?;
        Ok(result.results)
    }
}

/// Get interface by ID
pub async fn get_interface(core: &NetBoxClientCore, id: u64) -> Result<Interface, NetBoxError> {
    let url = format!("{}/api/dcim/interfaces/{}/", core.base_url, id);
    debug!("Fetching interface {} from NetBox", id);
    
    let response = core.client
        .get(&url)
        .header("Authorization", format!("Token {}", core.token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| NetBoxError::Http(e))?;
    
    if response.status() == 404 {
        return Err(NetBoxError::NotFound(format!("Interface {} not found", id)));
    }
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to get interface {}: {} - {}",
            id, status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Create a new interface
pub async fn create_interface(
    core: &NetBoxClientCore,
    device_id: u64,
    name: &str,
    interface_type: &str,
    enabled: Option<bool>,
    mac_address: Option<&str>,
    mtu: Option<u16>,
    description: Option<String>,
) -> Result<Interface, NetBoxError> {
    let url = format!("{}/api/dcim/interfaces/", core.base_url);
    debug!("Creating interface {} on device {} in NetBox", name, device_id);
    
    let mut body = serde_json::json!({
        "device": device_id,
        "name": name,
        "type": interface_type,
    });
    
    helpers::add_optional_bool_field(&mut body, "enabled", enabled);
    helpers::add_optional_string_field(&mut body, "mac_address", mac_address);
    helpers::add_optional_number_field(&mut body, "mtu", mtu);
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    
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
            "Failed to create interface: {} - {}",
            status, body
        )));
    }
    
    // Try to deserialize, but capture the response body for better error messages
    let response_text = response.text().await?;
    let interface: Interface = serde_json::from_str(&response_text).map_err(|e| {
        NetBoxError::Api(format!(
            "error decoding response body: {} - Response (first 500 chars): {}",
            e,
            response_text.chars().take(500).collect::<String>()
        ))
    })?;
    Ok(interface)
}

/// Update an interface
pub async fn update_interface(
    core: &NetBoxClientCore,
    id: u64,
    name: Option<&str>,
    interface_type: Option<&str>,
    enabled: Option<bool>,
    mac_address: Option<&str>,
    mtu: Option<u16>,
    description: Option<String>,
) -> Result<Interface, NetBoxError> {
    let url = format!("{}/api/dcim/interfaces/{}/", core.base_url, id);
    debug!("Updating interface {} in NetBox", id);
    
    let mut body = serde_json::json!({});
    
    if let Some(name_str) = name {
        body["name"] = serde_json::Value::String(name_str.to_string());
    }
    
    if let Some(if_type) = interface_type {
        body["type"] = serde_json::Value::String(if_type.to_string());
    }
    
    helpers::add_optional_bool_field(&mut body, "enabled", enabled);
    helpers::add_optional_string_field(&mut body, "mac_address", mac_address);
    helpers::add_optional_number_field(&mut body, "mtu", mtu);
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    
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
            "Failed to update interface {}: {} - {}",
            id, status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

