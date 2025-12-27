//! DCIM Region operations
//!
//! This module provides methods for managing NetBox DCIM regions.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::Region;
use tracing::debug;

/// Query regions by filters
pub async fn query_regions(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<Region>, NetBoxError> {
    let mut url = format!("{}/api/dcim/regions/", core.base_url);
    
    if !filters.is_empty() {
        let query_params: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect();
        url.push('?');
        url.push_str(&query_params.join("&"));
    }
    
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
                "Failed to query regions: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<Region> = response.json().await?;
        Ok(result.results)
    }
}

/// Get region by name
pub async fn get_region_by_name(core: &NetBoxClientCore, name: &str) -> Result<Option<Region>, NetBoxError> {
    let regions = query_regions(core, &[("name", name)], false).await?;
    Ok(regions.first().cloned())
}

/// Get region by ID
pub async fn get_region(core: &NetBoxClientCore, id: u64) -> Result<Region, NetBoxError> {
    let url = format!("{}/api/dcim/regions/{}/", core.base_url, id);
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
            "Failed to get region {}: {} - {}",
            id, status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Create a new region
pub async fn create_region(
    core: &NetBoxClientCore,
    name: &str,
    slug: Option<&str>,
    parent_id: Option<u64>,
    description: Option<String>,
    comments: Option<String>,
) -> Result<Region, NetBoxError> {
    let url = format!("{}/api/dcim/regions/", core.base_url);
    debug!("Creating region {} in NetBox", name);
    
    let slug_value = helpers::generate_slug(name, slug);
    let mut body = serde_json::json!({
        "name": name,
        "slug": slug_value,
    });
    
    helpers::add_nested_reference(&mut body, "parent", parent_id);
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
            "Failed to create region: {} - {}",
            status, body
        )));
    }
    
    // Capture response body for better error messages
    let response_text = response.text().await.map_err(|e| NetBoxError::Http(e))?;
    serde_json::from_str(&response_text).map_err(|e| {
        NetBoxError::Api(format!("error decoding response body: {} - Response: {}", e, response_text))
    })
}

