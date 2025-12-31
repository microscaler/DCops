//! Tenancy Tenant Group operations
//!
//! This module provides methods for managing NetBox Tenancy tenant groups.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::TenantGroup;
use crate::types::*;
use tracing::debug;

/// Query tenant groups by filters
pub async fn query_tenant_groups(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<TenantGroup>, NetBoxError> {
    let mut url = format!("{}/api/tenancy/tenant-groups/", core.base_url);
    
    if !filters.is_empty() {
        let query: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        url = format!("{}?{}", url, query.join("&"));
    }
    
    debug!("Querying tenant groups with filters: {:?}", filters);
    
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
                "Failed to query tenant groups: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<TenantGroup> = response.json().await?;
        Ok(result.results)
    }
}

/// Get tenant group by name (slug or name)
pub async fn get_tenant_group_by_name(
    core: &NetBoxClientCore,
    name: &str,
) -> Result<Option<TenantGroup>, NetBoxError> {
    // Try by name first
    let groups = query_tenant_groups(core, &[("name", name)], false).await?;
    if let Some(group) = groups.first() {
        return Ok(Some(group.clone()));
    }
    
    // Try by slug
    let groups = query_tenant_groups(core, &[("slug", name)], false).await?;
    Ok(groups.first().cloned())
}

/// Create a new tenant group
pub async fn create_tenant_group(
    core: &NetBoxClientCore,
    name: &str,
    slug: Option<&str>,
    description: Option<String>,
    comments: Option<String>,
    parent_id: Option<TenantGroupId>,
    tags: Option<Vec<String>>,
) -> Result<TenantGroup, NetBoxError> {
    let url = format!("{}/api/tenancy/tenant-groups/", core.base_url);
    debug!("Creating tenant group {} in NetBox", name);
    
    let slug_value = helpers::generate_slug(name, slug);
    let mut body = serde_json::json!({
        "name": name,
        "slug": slug_value,
    });
    
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    helpers::add_optional_string_field_owned(&mut body, "comments", comments);
    helpers::add_nested_reference(&mut body, "parent", parent_id.map(|id| id.into()));
    
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
            "Failed to create tenant group: {} - {}",
            status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Update an existing tenant group
pub async fn update_tenant_group(
    core: &NetBoxClientCore,
    id: TenantGroupId,
    name: Option<&str>,
    slug: Option<&str>,
    description: Option<String>,
    comments: Option<String>,
    parent_id: Option<TenantGroupId>,
    tags: Option<Vec<String>>,
) -> Result<TenantGroup, NetBoxError> {
    let id_value: u64 = id.into();
    let url = format!("{}/api/tenancy/tenant-groups/{}/", core.base_url, id_value);
    debug!("Updating tenant group {} in NetBox", id_value);
    
    let mut body = serde_json::json!({});
    
    helpers::add_optional_string_field(&mut body, "name", name);
    helpers::add_optional_string_field(&mut body, "slug", slug);
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    helpers::add_optional_string_field_owned(&mut body, "comments", comments);
    helpers::add_nested_reference(&mut body, "parent", parent_id.map(|id| id.into()));
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
            "Failed to update tenant group {}: {} - {}",
            id_value, status, body_text
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

