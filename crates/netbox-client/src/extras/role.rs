//! Extras Role operations
//!
//! This module provides methods for managing NetBox Extras roles.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::Role;
use crate::types::RoleId;
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
    tags: Option<Vec<String>>,
) -> Result<Role, NetBoxError> {
    let url = format!("{}/api/ipam/roles/", core.base_url);
    debug!("Creating role {} in NetBox", name);
    
    let slug_value = helpers::generate_slug(name, slug);
    let mut body = serde_json::json!({
        "name": name,
        "slug": slug_value,
    });
    
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    if let Some(w) = weight {
        body["weight"] = serde_json::Value::Number(serde_json::Number::from(w));
    }
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

/// Update an existing role
pub async fn update_role(
    core: &NetBoxClientCore,
    id: RoleId,
    name: Option<&str>,
    slug: Option<&str>,
    description: Option<String>,
    weight: Option<u16>,
    comments: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<Role, NetBoxError> {
    let id_value: u64 = id.into();
    let url = format!("{}/api/ipam/roles/{}/", core.base_url, id_value);
    debug!("Updating role {} in NetBox", id_value);
    
    let mut body = serde_json::json!({});
    
    helpers::add_optional_string_field(&mut body, "name", name);
    helpers::add_optional_string_field(&mut body, "slug", slug);
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    if let Some(w) = weight {
        body["weight"] = serde_json::Value::Number(serde_json::Number::from(w));
    }
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
            "Failed to update role {}: {} - {}",
            id_value, status, body_text
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

