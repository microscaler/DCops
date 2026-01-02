//! IPAM VRF operations
//!
//! This module provides methods for managing NetBox IPAM VRFs (Virtual Routing and Forwarding).

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::Vrf;
use crate::types::*;
use tracing::debug;

/// Query VRFs by filters
pub async fn query_vrfs(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<Vrf>, NetBoxError> {
    let mut url = format!("{}/api/ipam/vrfs/", core.base_url);
    
    if !filters.is_empty() {
        let query: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        url = format!("{}?{}", url, query.join("&"));
    }
    
    debug!("Querying VRFs with filters: {:?}", filters);
    
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
                "Failed to query VRFs: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<Vrf> = response.json().await?;
        Ok(result.results)
    }
}

/// Get a VRF by ID
pub async fn get_vrf(core: &NetBoxClientCore, id: VrfId) -> Result<Vrf, NetBoxError> {
    let id_value: u64 = id.into();
    let url = format!("{}/api/ipam/vrfs/{}/", core.base_url, id_value);
    debug!("Fetching VRF {} from NetBox", id_value);
    
    let response = core.client
        .get(&url)
        .header("Authorization", format!("Token {}", core.token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| NetBoxError::Http(e))?;
    
    if response.status() == 404 {
        return Err(NetBoxError::NotFound(format!("VRF {} not found", id_value)));
    }
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to get VRF {}: {} - {}",
            id_value, status, body
        )));
    }
    
    let vrf: Vrf = response.json().await
        .map_err(|e| NetBoxError::Http(e))?;
    Ok(vrf)
}

/// Get a VRF by name
pub async fn get_vrf_by_name(core: &NetBoxClientCore, name: &str) -> Result<Option<Vrf>, NetBoxError> {
    let vrfs = query_vrfs(core, &[("name", name)], false).await?;
    Ok(vrfs.into_iter().next())
}

/// Create a new VRF
pub async fn create_vrf(
    core: &NetBoxClientCore,
    name: &str,
    rd: Option<&str>,
    enforce_unique: Option<bool>,
    tenant_id: Option<TenantId>,
    description: Option<String>,
    comments: Option<String>,
    import_targets: Option<Vec<RouteTargetId>>,
    export_targets: Option<Vec<RouteTargetId>>,
    tags: Option<Vec<String>>,
) -> Result<Vrf, NetBoxError> {
    let url = format!("{}/api/ipam/vrfs/", core.base_url);
    debug!("Creating VRF {} in NetBox", name);
    
    let mut body = serde_json::json!({
        "name": name,
    });
    
    helpers::add_optional_string_field(&mut body, "rd", rd);
    helpers::add_optional_bool_field(&mut body, "enforce_unique", enforce_unique);
    helpers::add_nested_reference(&mut body, "tenant", tenant_id.map(|id| id.into()));
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    helpers::add_optional_string_field_owned(&mut body, "comments", comments);
    
    // Add import/export targets as arrays of IDs
    if let Some(import_ids) = import_targets {
        let import_array: Vec<u64> = import_ids.into_iter().map(|id| id.into()).collect();
        body["import_targets"] = serde_json::json!(import_array);
    }
    if let Some(export_ids) = export_targets {
        let export_array: Vec<u64> = export_ids.into_iter().map(|id| id.into()).collect();
        body["export_targets"] = serde_json::json!(export_array);
    }
    
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
            "Failed to create VRF: {} - {}",
            status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Update a VRF
pub async fn update_vrf(
    core: &NetBoxClientCore,
    id: VrfId,
    name: Option<&str>,
    rd: Option<&str>,
    enforce_unique: Option<bool>,
    tenant_id: Option<TenantId>,
    description: Option<String>,
    comments: Option<String>,
    import_targets: Option<Vec<RouteTargetId>>,
    export_targets: Option<Vec<RouteTargetId>>,
    tags: Option<Vec<String>>,
) -> Result<Vrf, NetBoxError> {
    let id_value: u64 = id.into();
    let url = format!("{}/api/ipam/vrfs/{}/", core.base_url, id_value);
    debug!("Updating VRF {} in NetBox", id_value);
    
    let mut body = serde_json::json!({});
    
    helpers::add_optional_string_field(&mut body, "name", name);
    helpers::add_optional_string_field(&mut body, "rd", rd);
    helpers::add_optional_bool_field(&mut body, "enforce_unique", enforce_unique);
    helpers::add_nested_reference(&mut body, "tenant", tenant_id.map(|id| id.into()));
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    helpers::add_optional_string_field_owned(&mut body, "comments", comments);
    
    // Add import/export targets as arrays of IDs
    if let Some(import_ids) = import_targets {
        let import_array: Vec<u64> = import_ids.into_iter().map(|id| id.into()).collect();
        body["import_targets"] = serde_json::json!(import_array);
    }
    if let Some(export_ids) = export_targets {
        let export_array: Vec<u64> = export_ids.into_iter().map(|id| id.into()).collect();
        body["export_targets"] = serde_json::json!(export_array);
    }
    
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
            "Failed to update VRF {}: {} - {}",
            id_value, status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Delete a VRF
pub async fn delete_vrf(core: &NetBoxClientCore, id: VrfId) -> Result<(), NetBoxError> {
    let id_value: u64 = id.into();
    let url = format!("{}/api/ipam/vrfs/{}/", core.base_url, id_value);
    debug!("Deleting VRF {} from NetBox", id_value);
    
    let response = core.client
        .delete(&url)
        .header("Authorization", format!("Token {}", core.token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| NetBoxError::Http(e))?;
    
    if response.status() == 404 {
        return Err(NetBoxError::NotFound(format!("VRF {} not found", id_value)));
    }
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to delete VRF {}: {} - {}",
            id_value, status, body
        )));
    }
    
    Ok(())
}

