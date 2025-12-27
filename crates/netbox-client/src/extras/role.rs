//! Extras Role operations
//!
//! This module provides methods for managing NetBox Extras roles.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::Role;
use tracing::debug;

/// Query roles by filters
pub async fn query_roles(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<Role>, NetBoxError> {
    let mut url = format!("{}/api/ipam/roles/", core.base_url);
    
    if !filters.is_empty() {
        let query: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        url = format!("{}?{}", url, query.join("&"));
    }
    
    debug!("Querying roles with filters: {:?}", filters);
    
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
                "Failed to query roles: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<Role> = response.json().await?;
        Ok(result.results)
    }
}

/// Get role by ID
pub async fn get_role(core: &NetBoxClientCore, id: u64) -> Result<Role, NetBoxError> {
    let url = format!("{}/api/ipam/roles/{}/", core.base_url, id);
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
            "Failed to get role {}: {} - {}",
            id, status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Create a new role
pub async fn create_role(
    core: &NetBoxClientCore,
    name: &str,
    slug: Option<&str>,
    description: Option<String>,
    weight: Option<u16>,
    comments: Option<String>,
) -> Result<Role, NetBoxError> {
    let url = format!("{}/api/ipam/roles/", core.base_url);
    debug!("Creating role {} in NetBox", name);
    
    let slug_value = helpers::generate_slug(name, slug);
    let mut body = serde_json::json!({
        "name": name,
        "slug": slug_value,
    });
    
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    helpers::add_optional_number_field(&mut body, "weight", weight);
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
            "Failed to create role: {} - {}",
            status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

