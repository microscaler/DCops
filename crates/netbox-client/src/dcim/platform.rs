//! DCIM Platform operations
//!
//! This module provides methods for managing NetBox DCIM platforms.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::Platform;
use crate::types::*;
use tracing::debug;

/// Query platforms by filters
pub async fn query_platforms(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<Platform>, NetBoxError> {
    let mut url = format!("{}/api/dcim/platforms/", core.base_url);
    
    if !filters.is_empty() {
        let query: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        url = format!("{}?{}", url, query.join("&"));
    }
    
    debug!("Querying platforms with filters: {:?}", filters);
    
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
                "Failed to query platforms: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<Platform> = response.json().await?;
        Ok(result.results)
    }
}

/// Get platform by name or slug
pub async fn get_platform_by_name(
    core: &NetBoxClientCore,
    name: &str,
) -> Result<Option<Platform>, NetBoxError> {
    let platforms = query_platforms(core, &[("name", name)], false).await?;
    if let Some(platform) = platforms.first() {
        return Ok(Some(platform.clone()));
    }
    
    let platforms = query_platforms(core, &[("slug", name)], false).await?;
    Ok(platforms.first().cloned())
}

/// Create a new platform
pub async fn create_platform(
    core: &NetBoxClientCore,
    name: &str,
    slug: Option<&str>,
    manufacturer_id: Option<ManufacturerId>,
    napalm_driver: Option<&str>,
    napalm_args: Option<&str>,
    description: Option<String>,
    comments: Option<String>,
) -> Result<Platform, NetBoxError> {
    let url = format!("{}/api/dcim/platforms/", core.base_url);
    debug!("Creating platform {} in NetBox", name);
    
    let slug_value = helpers::generate_slug(name, slug);
    let mut body = serde_json::json!({
        "name": name,
        "slug": slug_value,
    });
    
    helpers::add_nested_reference(&mut body, "manufacturer", manufacturer_id.map(|id| id.into()));
    helpers::add_optional_string_field(&mut body, "napalm_driver", napalm_driver);
    helpers::add_optional_string_field(&mut body, "napalm_args", napalm_args);
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
            "Failed to create platform: {} - {}",
            status, body
        )));
    }
    
    // Capture response body for better error messages
    let response_text = response.text().await.map_err(|e| NetBoxError::Http(e))?;
    serde_json::from_str(&response_text).map_err(|e| {
        NetBoxError::Api(format!("error decoding response body: {} - Response: {}", e, response_text))
    })
}

