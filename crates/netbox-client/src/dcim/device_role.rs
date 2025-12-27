//! DCIM Device Role operations
//!
//! This module provides methods for managing NetBox DCIM device roles.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::DeviceRole;
use tracing::debug;

/// Query device roles by filters
pub async fn query_device_roles(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<DeviceRole>, NetBoxError> {
    let mut url = format!("{}/api/dcim/device-roles/", core.base_url);
    
    if !filters.is_empty() {
        let query: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        url = format!("{}?{}", url, query.join("&"));
    }
    
    debug!("Querying device roles with filters: {:?}", filters);
    
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
                "Failed to query device roles: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<DeviceRole> = response.json().await?;
        Ok(result.results)
    }
}

/// Get device role by name or slug
pub async fn get_device_role_by_name(
    core: &NetBoxClientCore,
    name: &str,
) -> Result<Option<DeviceRole>, NetBoxError> {
    let roles = query_device_roles(core, &[("name", name)], false).await?;
    if let Some(role) = roles.first() {
        return Ok(Some(role.clone()));
    }
    
    let roles = query_device_roles(core, &[("slug", name)], false).await?;
    Ok(roles.first().cloned())
}

/// Create a new device role
pub async fn create_device_role(
    core: &NetBoxClientCore,
    name: &str,
    slug: Option<&str>,
    color: Option<&str>,
    vm_role: Option<bool>,
    description: Option<String>,
    comments: Option<String>,
) -> Result<DeviceRole, NetBoxError> {
    let url = format!("{}/api/dcim/device-roles/", core.base_url);
    debug!("Creating device role {} in NetBox", name);
    
    let slug_value = helpers::generate_slug(name, slug);
    let mut body = serde_json::json!({
        "name": name,
        "slug": slug_value,
    });
    
    helpers::add_optional_string_field(&mut body, "color", color);
    helpers::add_optional_bool_field(&mut body, "vm_role", vm_role);
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
    
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to create device role: {} - {}",
            status, body
        )));
    }
    
    // Capture response body for better error messages
    let response_text = response.text().await.map_err(|e| NetBoxError::Http(e))?;
    serde_json::from_str(&response_text).map_err(|e| {
        NetBoxError::Api(format!("error decoding response body: {} - Response: {}", e, response_text))
    })
}

