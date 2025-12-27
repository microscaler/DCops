//! DCIM Location operations
//!
//! This module provides methods for managing NetBox DCIM locations.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::Location;
use tracing::debug;

/// Query locations
pub async fn query_locations(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<Location>, NetBoxError> {
    let mut url = format!("{}/api/dcim/locations/", core.base_url);
    
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
                "Failed to query locations: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<Location> = response.json().await?;
        Ok(result.results)
    }
}

/// Get location by name and site
pub async fn get_location_by_name(
    core: &NetBoxClientCore,
    site_id: u64,
    name: &str,
) -> Result<Option<Location>, NetBoxError> {
    let locations = query_locations(core, &[("site_id", &site_id.to_string()), ("name", name)], false).await?;
    Ok(locations.first().cloned())
}

/// Get location by ID
pub async fn get_location(core: &NetBoxClientCore, id: u64) -> Result<Location, NetBoxError> {
    let url = format!("{}/api/dcim/locations/{}/", core.base_url, id);
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
            "Failed to get location {}: {} - {}",
            id, status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Create a new location
pub async fn create_location(
    core: &NetBoxClientCore,
    site_id: u64,
    name: &str,
    slug: Option<&str>,
    parent_id: Option<u64>,
    tenant_id: Option<u64>,
    facility: Option<&str>,
    description: Option<String>,
    comments: Option<String>,
) -> Result<Location, NetBoxError> {
    let url = format!("{}/api/dcim/locations/", core.base_url);
    debug!("Creating location {} in NetBox", name);
    
    let slug_value = helpers::generate_slug(name, slug);
    let mut body = serde_json::json!({
        "site": {"id": site_id},
        "name": name,
        "slug": slug_value,
    });
    
    // NetBox requires parent field - send null if not provided (top-level location)
    if let Some(parent) = parent_id {
        helpers::add_nested_reference(&mut body, "parent", Some(parent));
    } else {
        body["parent"] = serde_json::Value::Null; // Top-level location
    }
    
    // For CREATE operations, NetBox 4.0 requires full tenant object (id, name, slug)
    helpers::add_tenant_for_create(&mut body, core, tenant_id).await;
    helpers::add_optional_string_field(&mut body, "facility", facility);
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
            "Failed to create location: {} - {}",
            status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

