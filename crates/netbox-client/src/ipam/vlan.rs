//! IPAM VLAN operations
//!
//! This module provides methods for managing NetBox IPAM VLANs.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::Vlan;
use crate::types::*;
use tracing::debug;

/// Query VLANs by filters
pub async fn query_vlans(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<Vlan>, NetBoxError> {
    let mut url = format!("{}/api/ipam/vlans/", core.base_url);
    
    if !filters.is_empty() {
        let query: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        url = format!("{}?{}", url, query.join("&"));
    }
    
    debug!("Querying VLANs with filters: {:?}", filters);
    
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
                "Failed to query VLANs: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<Vlan> = response.json().await?;
        Ok(result.results)
    }
}

/// Get a VLAN by ID
pub async fn get_vlan(core: &NetBoxClientCore, id: VlanId) -> Result<Vlan, NetBoxError> {
    let id_value: u32 = id.into();
    let url = format!("{}/api/ipam/vlans/{}/", core.base_url, id_value);
    debug!("Fetching VLAN {} from NetBox", id_value);
    
    let response = core.client
        .get(&url)
        .header("Authorization", format!("Token {}", core.token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| NetBoxError::Http(e))?;
    
    if response.status() == 404 {
        return Err(NetBoxError::NotFound(format!("VLAN {} not found", id_value)));
    }
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(NetBoxError::Api(format!(
            "Failed to get VLAN {}: {} - {}",
            id_value, status, body
        )));
    }
    
    let vlan: Vlan = response.json().await
        .map_err(|e| NetBoxError::Http(e))?;
    Ok(vlan)
}

/// Create a new VLAN
pub async fn create_vlan(
    core: &NetBoxClientCore,
    vid: u16,
    name: &str,
    site_id: Option<SiteId>,
    group_id: Option<VlanGroupId>,
    tenant_id: Option<TenantId>,
    role_id: Option<RoleId>,
    status: Option<&str>,
    description: Option<String>,
    comments: Option<String>,
) -> Result<Vlan, NetBoxError> {
    let url = format!("{}/api/ipam/vlans/", core.base_url);
    debug!("Creating VLAN {} ({}) in NetBox", vid, name);
    
    let mut body = serde_json::json!({
        "vid": vid,
        "name": name,
    });
    
    // For CREATE operations, NetBox 4.0 requires full tenant object (id, name, slug)
    helpers::add_nested_reference(&mut body, "site", site_id.map(|id| id.into()));
    helpers::add_nested_reference(&mut body, "group", group_id.map(|id| id.into()));
    helpers::add_tenant_for_create(&mut body, core, tenant_id.map(|id| id.into())).await;
    helpers::add_nested_reference(&mut body, "role", role_id.map(|id| id.into()));
    helpers::add_optional_string_field(&mut body, "status", status);
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
            "Failed to create VLAN: {} - {}",
            status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Update a VLAN
pub async fn update_vlan(
    core: &NetBoxClientCore,
    id: u64,
    vid: Option<u16>,
    name: Option<&str>,
    site_id: Option<u64>,
    group_id: Option<u64>,
    tenant_id: Option<u64>,
    role_id: Option<u64>,
    status: Option<&str>,
    description: Option<String>,
    comments: Option<String>,
) -> Result<Vlan, NetBoxError> {
    let url = format!("{}/api/ipam/vlans/{}/", core.base_url, id);
    debug!("Updating VLAN {} in NetBox", id);
    
    let mut body = serde_json::json!({});
    
    helpers::add_optional_number_field(&mut body, "vid", vid);
    helpers::add_optional_string_field(&mut body, "name", name);
    helpers::add_nested_reference(&mut body, "site", site_id.map(|id| id.into()));
    helpers::add_nested_reference(&mut body, "group", group_id.map(|id| id.into()));
    helpers::add_nested_reference(&mut body, "tenant", tenant_id.map(|id| id.into()));
    helpers::add_nested_reference(&mut body, "role", role_id.map(|id| id.into()));
    helpers::add_optional_string_field(&mut body, "status", status);
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    helpers::add_optional_string_field_owned(&mut body, "comments", comments);
    
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
            "Failed to update VLAN {}: {} - {}",
            id, status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

