//! Tenancy Tenant operations
//!
//! This module provides methods for managing NetBox Tenancy tenants.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::Tenant;
use tracing::debug;

/// Query tenants by filters
pub async fn query_tenants(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<Tenant>, NetBoxError> {
    let mut url = format!("{}/api/tenancy/tenants/", core.base_url);
    
    if !filters.is_empty() {
        let query: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        url = format!("{}?{}", url, query.join("&"));
    }
    
    debug!("Querying tenants with filters: {:?}", filters);
    
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
                "Failed to query tenants: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<Tenant> = response.json().await?;
        Ok(result.results)
    }
}

/// Get tenant by ID
pub async fn get_tenant(core: &NetBoxClientCore, id: u64) -> Result<Tenant, NetBoxError> {
    let url = format!("{}/api/tenancy/tenants/{}/", core.base_url, id);
    let response = core.client
        .get(&url)
        .header("Authorization", format!("Token {}", core.token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| {
            // Convert reqwest errors to appropriate NetBoxError types
            if e.is_timeout() {
                NetBoxError::Api(format!("Request timeout: {}", e))
            } else if e.is_connect() {
                NetBoxError::Api(format!("Connection error: {}", e))
            } else {
                NetBoxError::Http(e)
            }
        })?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        
        // Handle 404 specifically as NotFound
        if status == 404 {
            return Err(NetBoxError::NotFound(format!(
                "Tenant {} not found: {}",
                id, body
            )));
        }
        
        return Err(NetBoxError::Api(format!(
            "Failed to get tenant {}: {} - {}",
            id, status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Create a new tenant
pub async fn create_tenant(
    core: &NetBoxClientCore,
    name: &str,
    slug: Option<&str>,
    description: Option<String>,
    comments: Option<String>,
    group: Option<u64>, // Tenant group ID
) -> Result<Tenant, NetBoxError> {
    let url = format!("{}/api/tenancy/tenants/", core.base_url);
    debug!("Creating tenant {} in NetBox", name);
    
    let slug_value = helpers::generate_slug(name, slug);
    let mut body = serde_json::json!({
        "name": name,
        "slug": slug_value,
    });
    
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    helpers::add_optional_string_field_owned(&mut body, "comments", comments);
    if let Some(group_id) = group {
        helpers::add_nested_reference(&mut body, "group", Some(group_id.into()));
    } else {
        body["group"] = serde_json::Value::Null;
    }
    
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
            "Failed to create tenant: {} - {}",
            status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Update an existing tenant
pub async fn update_tenant(
    core: &NetBoxClientCore,
    id: u64,
    name: Option<&str>,
    slug: Option<&str>,
    description: Option<String>,
    comments: Option<String>,
    group: Option<u64>, // Tenant group ID
) -> Result<Tenant, NetBoxError> {
    let url = format!("{}/api/tenancy/tenants/{}/", core.base_url, id);
    debug!("Updating tenant {} in NetBox", id);
    
    let mut body = serde_json::json!({});
    
    if let Some(name_val) = name {
        body["name"] = serde_json::Value::String(name_val.to_string());
    }
    
    if let Some(slug_val) = slug {
        body["slug"] = serde_json::Value::String(slug_val.to_string());
    }
    
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    helpers::add_optional_string_field_owned(&mut body, "comments", comments);
    helpers::add_nested_reference(&mut body, "group", group.map(|id| id.into()));
    
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
            "Failed to update tenant {}: {} - {}",
            id, status, body_text
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

