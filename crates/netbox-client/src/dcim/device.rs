//! DCIM Device operations
//!
//! This module provides methods for managing NetBox DCIM devices.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::dcim::interface::query_interfaces;
use crate::error::NetBoxError;
use crate::models::Device;
use tracing::debug;

/// Query devices by filters
pub async fn query_devices(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<Device>, NetBoxError> {
    let mut url = format!("{}/api/dcim/devices/", core.base_url);
    
    if !filters.is_empty() {
        let query: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        url = format!("{}?{}", url, query.join("&"));
    }
    
    debug!("Querying devices with filters: {:?}", filters);
    
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
                "Failed to query devices: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<Device> = response.json().await?;
        Ok(result.results)
    }
}

/// Get a device by ID
pub async fn get_device(core: &NetBoxClientCore, id: u64) -> Result<Device, NetBoxError> {
    let url = format!("{}/api/dcim/devices/{}/", core.base_url, id);
    debug!("Fetching device {} from NetBox", id);
    
    let response = core.client
        .get(&url)
        .header("Authorization", format!("Token {}", core.token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| NetBoxError::Http(e))?;
    
    if response.status() == 404 {
        return Err(NetBoxError::NotFound(format!("Device {} not found", id)));
    }
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to get device {}: {} - {}",
            id, status, body
        )));
    }
    
    let device: Device = response.json().await
        .map_err(|e| NetBoxError::Http(e))?;
    Ok(device)
}

/// Get device by MAC address
pub async fn get_device_by_mac(core: &NetBoxClientCore, mac: &str) -> Result<Option<Device>, NetBoxError> {
    debug!("Querying device by MAC address: {}", mac);
    
    let interfaces = query_interfaces(core, &[("mac_address", mac)], false).await?;
    
    if interfaces.is_empty() {
        return Ok(None);
    }
    
    // Get the device from the first matching interface
    let interface = &interfaces[0];
    let device_id = interface.device.id;
    
    // Fetch the device
    let device = get_device(core, device_id).await?;
    Ok(Some(device))
}

/// Create a new device
pub async fn create_device(
    core: &NetBoxClientCore,
    device_type_id: u64,
    device_role_id: u64,
    site_id: u64,
    name: Option<&str>,
    tenant_id: Option<u64>,
    platform_id: Option<u64>,
    location_id: Option<u64>,
    serial: Option<&str>,
    asset_tag: Option<&str>,
    status: Option<&str>,
    primary_ip4_id: Option<u64>,
    primary_ip6_id: Option<u64>,
    description: Option<String>,
    comments: Option<String>,
) -> Result<Device, NetBoxError> {
    let url = format!("{}/api/dcim/devices/", core.base_url);
    debug!("Creating device in NetBox");
    
    let mut body = serde_json::json!({});
    helpers::add_required_nested_reference(&mut body, "device_type", device_type_id);
    helpers::add_required_nested_reference(&mut body, "role", device_role_id);
    helpers::add_required_nested_reference(&mut body, "site", site_id);
    
    helpers::add_optional_string_field(&mut body, "name", name);
    // For CREATE operations, NetBox 4.0 requires full tenant object (id, name, slug)
    helpers::add_tenant_for_create(&mut body, core, tenant_id).await;
    helpers::add_nested_reference(&mut body, "platform", platform_id);
    helpers::add_nested_reference(&mut body, "location", location_id);
    helpers::add_optional_string_field(&mut body, "serial", serial);
    helpers::add_optional_string_field(&mut body, "asset_tag", asset_tag);
    helpers::add_optional_string_field(&mut body, "status", status);
    helpers::add_nested_reference(&mut body, "primary_ip4", primary_ip4_id);
    helpers::add_nested_reference(&mut body, "primary_ip6", primary_ip6_id);
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    helpers::add_optional_string_field_owned(&mut body, "comments", comments);
    
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
            "Failed to create device: {} - {}",
            status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Update a device
pub async fn update_device(
    core: &NetBoxClientCore,
    id: u64,
    name: Option<&str>,
    tenant_id: Option<u64>,
    platform_id: Option<u64>,
    location_id: Option<u64>,
    serial: Option<&str>,
    asset_tag: Option<&str>,
    status: Option<&str>,
    primary_ip4_id: Option<u64>,
    primary_ip6_id: Option<u64>,
    description: Option<String>,
    comments: Option<String>,
) -> Result<Device, NetBoxError> {
    let url = format!("{}/api/dcim/devices/{}/", core.base_url, id);
    debug!("Updating device {} in NetBox", id);
    
    let mut body = serde_json::json!({});
    
    if let Some(name_str) = name {
        body["name"] = serde_json::Value::String(name_str.to_string());
    }
    
    // NetBox 4.0 PATCH updates: For nested objects, send only {"id": X}
    // Sending the full object causes NetBox to try to CREATE a new object
    helpers::add_nested_reference(&mut body, "tenant", tenant_id);
    helpers::add_nested_reference(&mut body, "platform", platform_id);
    helpers::add_nested_reference(&mut body, "location", location_id);
    
    if let Some(serial_str) = serial {
        body["serial"] = serde_json::Value::String(serial_str.to_string());
    }
    
    if let Some(asset) = asset_tag {
        body["asset_tag"] = serde_json::Value::String(asset.to_string());
    }
    
    if let Some(status_str) = status {
        body["status"] = serde_json::Value::String(status_str.to_string());
    }
    
    helpers::add_nested_reference(&mut body, "primary_ip4", primary_ip4_id);
    helpers::add_nested_reference(&mut body, "primary_ip6", primary_ip6_id);
    
    if let Some(desc) = description {
        body["description"] = serde_json::Value::String(desc);
    }
    
    if let Some(comments_str) = comments {
        body["comments"] = serde_json::Value::String(comments_str);
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
            "Failed to update device {}: {} - {}",
            id, status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

