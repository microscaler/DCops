//! DCIM Site operations
//!
//! This module provides methods for managing NetBox DCIM sites.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::Site;
use tracing::debug;

/// Query sites by filters
pub async fn query_sites(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<Site>, NetBoxError> {
    let mut url = format!("{}/api/dcim/sites/", core.base_url);
    
    if !filters.is_empty() {
        let query: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        url = format!("{}?{}", url, query.join("&"));
    }
    
    debug!("Querying sites with filters: {:?}", filters);
    
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
                "Failed to query sites: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<Site> = response.json().await?;
        Ok(result.results)
    }
}

/// Get site by ID
pub async fn get_site(core: &NetBoxClientCore, id: u64) -> Result<Site, NetBoxError> {
    let url = format!("{}/api/dcim/sites/{}/", core.base_url, id);
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
            "Failed to get site {}: {} - {}",
            id, status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Create a new site
pub async fn create_site(
    core: &NetBoxClientCore,
    name: &str,
    slug: Option<&str>,
    description: Option<String>,
    physical_address: Option<String>,
    shipping_address: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    tenant_id: Option<u64>,
    region_id: Option<u64>,
    site_group_id: Option<u64>,
    status: Option<&str>, // "active", "planned", "retired", "staging"
    facility: Option<String>,
    time_zone: Option<String>,
    comments: Option<String>,
) -> Result<Site, NetBoxError> {
    let url = format!("{}/api/dcim/sites/", core.base_url);
    debug!("Creating site {} in NetBox", name);
    
    let slug_value = helpers::generate_slug(name, slug);
    let mut body = serde_json::json!({
        "name": name,
        "slug": slug_value,
    });
    
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    helpers::add_optional_string_field_owned(&mut body, "physical_address", physical_address);
    helpers::add_optional_string_field_owned(&mut body, "shipping_address", shipping_address);
    helpers::add_optional_number_field(&mut body, "latitude", latitude.map(|l| serde_json::Number::from_f64(l).unwrap()));
    helpers::add_optional_number_field(&mut body, "longitude", longitude.map(|l| serde_json::Number::from_f64(l).unwrap()));
    
    // For CREATE operations, NetBox 4.0 requires full tenant object (id, name, slug)
    // For PATCH, we can use just {"id": X}, but for POST we need the full object
    helpers::add_tenant_for_create(&mut body, core, tenant_id).await;
    
    // Region and site_group can use just ID for CREATE
    helpers::add_nested_reference(&mut body, "region", region_id);
    helpers::add_nested_reference(&mut body, "site_group", site_group_id);
    
    helpers::add_optional_string_field(&mut body, "status", status);
    helpers::add_optional_string_field_owned(&mut body, "facility", facility);
    helpers::add_optional_string_field_owned(&mut body, "time_zone", time_zone);
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
            "Failed to create site: {} - {}",
            status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Update a site
/// 
/// Note: For nested fields (tenant, region, site_group), only include them if they've changed.
/// NetBox's nested serializers expect either an integer PK or a dictionary with attributes.
/// When using PATCH, NetBox 4.0 requires nested objects as simple integer IDs.
pub async fn update_site(
    core: &NetBoxClientCore,
    id: u64,
    name: Option<&str>,
    slug: Option<&str>,
    description: Option<String>,
    physical_address: Option<String>,
    shipping_address: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    tenant_id: Option<u64>,
    region_id: Option<u64>,
    site_group_id: Option<u64>,
    status: Option<&str>, // "active", "planned", "retired", "staging"
    facility: Option<String>,
    time_zone: Option<String>,
    comments: Option<String>,
) -> Result<Site, NetBoxError> {
    let url = format!("{}/api/dcim/sites/{}/", core.base_url, id);
    debug!("Updating site {} in NetBox", id);
    
    let mut body = serde_json::json!({});
    
    // Debug: Log what we're sending for tenant
    if tenant_id.is_some() {
        debug!("update_site: Including tenant_id={:?} in update body", tenant_id);
    }
    
    helpers::add_optional_string_field(&mut body, "name", name);
    helpers::add_optional_string_field(&mut body, "slug", slug);
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    helpers::add_optional_string_field_owned(&mut body, "physical_address", physical_address);
    helpers::add_optional_string_field_owned(&mut body, "shipping_address", shipping_address);
    helpers::add_optional_number_field(&mut body, "latitude", latitude.map(|l| serde_json::Number::from_f64(l).unwrap()));
    helpers::add_optional_number_field(&mut body, "longitude", longitude.map(|l| serde_json::Number::from_f64(l).unwrap()));
    
    // NetBox 4.0 PATCH updates: For nested objects, send only {"id": X}
    // Sending the full object causes NetBox to try to CREATE a new object
    // If id is None, we don't include the field in the body (PATCH semantics - only send changed fields)
    helpers::add_nested_reference(&mut body, "tenant", tenant_id);
    helpers::add_nested_reference(&mut body, "region", region_id);
    helpers::add_nested_reference(&mut body, "site_group", site_group_id);
    
    helpers::add_optional_string_field(&mut body, "status", status);
    helpers::add_optional_string_field_owned(&mut body, "facility", facility);
    helpers::add_optional_string_field_owned(&mut body, "time_zone", time_zone);
    helpers::add_optional_string_field_owned(&mut body, "comments", comments);
    
    // Debug: Log the request body before sending
    debug!("update_site: Request body: {}", serde_json::to_string(&body).unwrap_or_default());
    
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
        debug!("update_site: Error response ({}): {}", status, body_text);
        // Check if response is HTML (404/error page) vs JSON error
        let body = if body_text.trim_start().starts_with("<!DOCTYPE") || body_text.trim_start().starts_with("<html") {
            // Extract error message from HTML if possible, otherwise use truncated HTML
            if body_text.len() > 500 {
                format!("HTML error page (first 500 chars): {}", &body_text[..500])
            } else {
                format!("HTML error page: {}", body_text)
            }
        } else {
            body_text
        };
        return Err(NetBoxError::Api(format!(
            "Failed to update site {}: {} - {}",
            id, status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

