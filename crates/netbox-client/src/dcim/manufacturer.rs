//! DCIM Manufacturer operations
//!
//! This module provides methods for managing NetBox DCIM manufacturers.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::Manufacturer;
use crate::types::ManufacturerId;
use tracing::debug;

/// Query manufacturers by filters
pub async fn query_manufacturers(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<Manufacturer>, NetBoxError> {
    let mut url = format!("{}/api/dcim/manufacturers/", core.base_url);
    
    if !filters.is_empty() {
        let query: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        url = format!("{}?{}", url, query.join("&"));
    }
    
    debug!("Querying manufacturers with filters: {:?}", filters);
    
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
                "Failed to query manufacturers: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<Manufacturer> = response.json().await?;
        Ok(result.results)
    }
}

/// Get manufacturer by name or slug
pub async fn get_manufacturer_by_name(
    core: &NetBoxClientCore,
    name: &str,
) -> Result<Option<Manufacturer>, NetBoxError> {
    let manufacturers = query_manufacturers(core, &[("name", name)], false).await?;
    if let Some(mfg) = manufacturers.first() {
        return Ok(Some(mfg.clone()));
    }
    
    let manufacturers = query_manufacturers(core, &[("slug", name)], false).await?;
    Ok(manufacturers.first().cloned())
}

/// Create a new manufacturer
pub async fn create_manufacturer(
    core: &NetBoxClientCore,
    name: &str,
    slug: Option<&str>,
    description: Option<String>,
    comments: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<Manufacturer, NetBoxError> {
    let url = format!("{}/api/dcim/manufacturers/", core.base_url);
    debug!("Creating manufacturer {} in NetBox", name);
    
    let slug_value = helpers::generate_slug(name, slug);
    let mut body = serde_json::json!({
        "name": name,
        "slug": slug_value,
    });
    
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    helpers::add_optional_string_field_owned(&mut body, "comments", comments);
    
    helpers::add_optional_tags_field(&mut body, tags)?;
    
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
            "Failed to create manufacturer: {} - {}",
            status, body
        )));
    }
    
    // Capture response body for better error messages
    let response_text = response.text().await.map_err(|e| NetBoxError::Http(e))?;
    serde_json::from_str(&response_text).map_err(|e| {
        NetBoxError::Api(format!("error decoding response body: {} - Response: {}", e, response_text))
    })
}

/// Update an existing manufacturer
pub async fn update_manufacturer(
    core: &NetBoxClientCore,
    id: ManufacturerId,
    name: Option<&str>,
    slug: Option<&str>,
    description: Option<String>,
    comments: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<Manufacturer, NetBoxError> {
    let id_value: u64 = id.into();
    let url = format!("{}/api/dcim/manufacturers/{}/", core.base_url, id_value);
    debug!("Updating manufacturer {} in NetBox", id_value);
    
    let mut body = serde_json::json!({});
    
    helpers::add_optional_string_field(&mut body, "name", name);
    helpers::add_optional_string_field(&mut body, "slug", slug);
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    helpers::add_optional_string_field_owned(&mut body, "comments", comments);
    helpers::add_optional_tags_field(&mut body, tags)?;
    
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
        let body_text = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to update manufacturer {}: {} - {}",
            id_value, status, body_text
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

