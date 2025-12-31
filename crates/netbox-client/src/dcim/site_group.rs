//! DCIM Site Group operations
//!
//! This module provides methods for managing NetBox DCIM site groups.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::SiteGroup;
use crate::types::*;
use tracing::debug;

/// Query site groups
pub async fn query_site_groups(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<SiteGroup>, NetBoxError> {
    let mut url = format!("{}/api/dcim/site-groups/", core.base_url);
    
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
                "Failed to query site groups: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<SiteGroup> = response.json().await?;
        Ok(result.results)
    }
}

/// Get site group by name
pub async fn get_site_group_by_name(core: &NetBoxClientCore, name: &str) -> Result<Option<SiteGroup>, NetBoxError> {
    let site_groups = query_site_groups(core, &[("name", name)], false).await?;
    Ok(site_groups.first().cloned())
}

/// Get site group by ID
pub async fn get_site_group(core: &NetBoxClientCore, id: SiteGroupId) -> Result<SiteGroup, NetBoxError> {
    let id_value: u64 = id.into();
    let url = format!("{}/api/dcim/site-groups/{}/", core.base_url, id_value);
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
            "Failed to get site group {}: {} - {}",
            id_value, status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Create a new site group
pub async fn create_site_group(
    core: &NetBoxClientCore,
    name: &str,
    slug: Option<&str>,
    parent_id: Option<SiteGroupId>,
    description: Option<String>,
    comments: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<SiteGroup, NetBoxError> {
    let url = format!("{}/api/dcim/site-groups/", core.base_url);
    debug!("Creating site group {} in NetBox", name);
    
    let slug_value = helpers::generate_slug(name, slug);
    let mut body = serde_json::json!({
        "name": name,
        "slug": slug_value,
    });
    
    helpers::add_nested_reference(&mut body, "parent", parent_id.map(|id| id.into()));
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
            "Failed to create site group: {} - {}",
            status, body
        )));
    }
    
    // Capture response body for better error messages
    let response_text = response.text().await.map_err(|e| NetBoxError::Http(e))?;
    serde_json::from_str(&response_text).map_err(|e| {
        NetBoxError::Api(format!("error decoding response body: {} - Response: {}", e, response_text))
    })
}

/// Update an existing site group
pub async fn update_site_group(
    core: &NetBoxClientCore,
    id: SiteGroupId,
    name: Option<&str>,
    slug: Option<&str>,
    parent_id: Option<SiteGroupId>,
    description: Option<String>,
    comments: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<SiteGroup, NetBoxError> {
    let id_value: u64 = id.into();
    let url = format!("{}/api/dcim/site-groups/{}/", core.base_url, id_value);
    debug!("Updating site group {} in NetBox", id_value);
    
    let mut body = serde_json::json!({});
    
    helpers::add_optional_string_field(&mut body, "name", name);
    helpers::add_optional_string_field(&mut body, "slug", slug);
    helpers::add_nested_reference(&mut body, "parent", parent_id.map(|id| id.into()));
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
            "Failed to update site group {}: {} - {}",
            id_value, status, body_text
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

