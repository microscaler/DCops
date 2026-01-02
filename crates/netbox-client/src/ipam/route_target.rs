//! IPAM Route Target operations
//!
//! This module provides methods for managing NetBox IPAM Route Targets.
//! Route targets are extended BGP communities used to manage route redistribution among VRF tables.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::RouteTarget;
use crate::types::*;
use tracing::debug;

/// Query Route Targets by filters
pub async fn query_route_targets(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<RouteTarget>, NetBoxError> {
    let mut url = format!("{}/api/ipam/route-targets/", core.base_url);
    
    if !filters.is_empty() {
        let query: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        url = format!("{}?{}", url, query.join("&"));
    }
    
    debug!("Querying Route Targets with filters: {:?}", filters);
    
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
                "Failed to query Route Targets: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<RouteTarget> = response.json().await?;
        Ok(result.results)
    }
}

/// Get a Route Target by ID
pub async fn get_route_target(core: &NetBoxClientCore, id: RouteTargetId) -> Result<RouteTarget, NetBoxError> {
    let id_value: u64 = id.into();
    let url = format!("{}/api/ipam/route-targets/{}/", core.base_url, id_value);
    debug!("Fetching Route Target {} from NetBox", id_value);
    
    let response = core.client
        .get(&url)
        .header("Authorization", format!("Token {}", core.token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| NetBoxError::Http(e))?;
    
    if response.status() == 404 {
        return Err(NetBoxError::NotFound(format!("Route Target {} not found", id_value)));
    }
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to get Route Target {}: {} - {}",
            id_value, status, body
        )));
    }
    
    let route_target: RouteTarget = response.json().await
        .map_err(|e| NetBoxError::Http(e))?;
    Ok(route_target)
}

/// Get a Route Target by name
pub async fn get_route_target_by_name(core: &NetBoxClientCore, name: &str) -> Result<Option<RouteTarget>, NetBoxError> {
    let route_targets = query_route_targets(core, &[("name", name)], false).await?;
    Ok(route_targets.into_iter().next())
}

/// Create a new Route Target
pub async fn create_route_target(
    core: &NetBoxClientCore,
    name: &str,
    tenant_id: Option<TenantId>,
    description: Option<String>,
    comments: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<RouteTarget, NetBoxError> {
    let url = format!("{}/api/ipam/route-targets/", core.base_url);
    debug!("Creating Route Target {} in NetBox", name);
    
    let mut body = serde_json::json!({
        "name": name,
    });
    
    helpers::add_nested_reference(&mut body, "tenant", tenant_id.map(|id| id.into()));
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
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to create Route Target: {} - {}",
            status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Update a Route Target
pub async fn update_route_target(
    core: &NetBoxClientCore,
    id: RouteTargetId,
    name: Option<&str>,
    tenant_id: Option<TenantId>,
    description: Option<String>,
    comments: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<RouteTarget, NetBoxError> {
    let id_value: u64 = id.into();
    let url = format!("{}/api/ipam/route-targets/{}/", core.base_url, id_value);
    debug!("Updating Route Target {} in NetBox", id_value);
    
    let mut body = serde_json::json!({});
    
    helpers::add_optional_string_field(&mut body, "name", name);
    helpers::add_nested_reference(&mut body, "tenant", tenant_id.map(|id| id.into()));
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
        let body = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to update Route Target {}: {} - {}",
            id_value, status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Delete a Route Target
pub async fn delete_route_target(core: &NetBoxClientCore, id: RouteTargetId) -> Result<(), NetBoxError> {
    let id_value: u64 = id.into();
    let url = format!("{}/api/ipam/route-targets/{}/", core.base_url, id_value);
    debug!("Deleting Route Target {} from NetBox", id_value);
    
    let response = core.client
        .delete(&url)
        .header("Authorization", format!("Token {}", core.token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| NetBoxError::Http(e))?;
    
    if response.status() == 404 {
        return Err(NetBoxError::NotFound(format!("Route Target {} not found", id_value)));
    }
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to delete Route Target {}: {} - {}",
            id_value, status, body
        )));
    }
    
    Ok(())
}

