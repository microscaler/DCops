//! DCIM Device Type operations
//!
//! This module provides methods for managing NetBox DCIM device types.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::DeviceType;
use crate::types::*;
use tracing::debug;

/// Query device types by filters
pub async fn query_device_types(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<DeviceType>, NetBoxError> {
    let mut url = format!("{}/api/dcim/device-types/", core.base_url);
    
    if !filters.is_empty() {
        let query: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        url = format!("{}?{}", url, query.join("&"));
    }
    
    debug!("Querying device types with filters: {:?}", filters);
    
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
                "Failed to query device types: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<DeviceType> = response.json().await?;
        Ok(result.results)
    }
}

/// Get device type by manufacturer and model
pub async fn get_device_type_by_model(
    core: &NetBoxClientCore,
    manufacturer_id: ManufacturerId,
    model: &str,
) -> Result<Option<DeviceType>, NetBoxError> {
    let manufacturer_id_value: u64 = manufacturer_id.into();
    let device_types = query_device_types(core, &[("manufacturer_id", &manufacturer_id_value.to_string()), ("model", model)], false).await?;
    Ok(device_types.first().cloned())
}

/// Create a new device type
pub async fn create_device_type(
    core: &NetBoxClientCore,
    manufacturer_id: ManufacturerId,
    model: &str,
    slug: Option<&str>,
    part_number: Option<&str>,
    u_height: Option<f64>,
    is_full_depth: Option<bool>,
    description: Option<String>,
    comments: Option<String>,
) -> Result<DeviceType, NetBoxError> {
    let manufacturer_id_value: u64 = manufacturer_id.into();
    let url = format!("{}/api/dcim/device-types/", core.base_url);
    debug!("Creating device type {} in NetBox", model);
    
    let slug_value = if let Some(slug_str) = slug {
        slug_str.to_string()
    } else {
        model.to_lowercase().replace(' ', "-")
    };
    
    let mut body = serde_json::json!({
        "manufacturer": manufacturer_id_value,
        "model": model,
        "slug": slug_value,
    });
    
    helpers::add_optional_string_field(&mut body, "part_number", part_number);
    helpers::add_optional_number_field(&mut body, "u_height", u_height.map(|h| serde_json::Number::from_f64(h).unwrap_or(serde_json::Number::from(1))));
    helpers::add_optional_bool_field(&mut body, "is_full_depth", is_full_depth);
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
            "Failed to create device type: {} - {}",
            status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

