//! IPAM RIR operations
//!
//! This module provides methods for managing NetBox IPAM RIRs (Regional Internet Registries).

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::Rir;
use crate::types::RirId;
use tracing::debug;

/// Query RIRs by filters
pub async fn query_rirs(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<Rir>, NetBoxError> {
    let mut url = format!("{}/api/ipam/rirs/", core.base_url);
    
    if !filters.is_empty() {
        let query: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        url = format!("{}?{}", url, query.join("&"));
    }
    
    debug!("Querying RIRs with filters: {:?}", filters);
    
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
                "Failed to query RIRs: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<Rir> = response.json().await?;
        Ok(result.results)
    }
}

/// Get RIR by name (slug or name)
pub async fn get_rir_by_name(core: &NetBoxClientCore, name: &str) -> Result<Option<Rir>, NetBoxError> {
    // Try by name first
    let rirs = query_rirs(core, &[("name", name)], false).await?;
    if let Some(rir) = rirs.first() {
        return Ok(Some(rir.clone()));
    }
    
    // Try by slug
    let rirs = query_rirs(core, &[("slug", name)], false).await?;
    Ok(rirs.first().cloned())
}

/// Create a new RIR
pub async fn create_rir(
    core: &NetBoxClientCore,
    name: &str,
    slug: Option<&str>,
    description: Option<String>,
    comments: Option<String>,
    is_private: Option<bool>,
    tags: Option<Vec<String>>,
) -> Result<Rir, NetBoxError> {
    let url = format!("{}/api/ipam/rirs/", core.base_url);
    debug!("Creating RIR {} in NetBox", name);
    
    let slug_value = helpers::generate_slug(name, slug);
    let mut body = serde_json::json!({
        "name": name,
        "slug": slug_value,
    });
    
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    helpers::add_optional_string_field_owned(&mut body, "comments", comments);
    helpers::add_optional_bool_field(&mut body, "is_private", is_private);
    
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
            "Failed to create RIR: {} - {}",
            status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Update an existing RIR
pub async fn update_rir(
    core: &NetBoxClientCore,
    id: RirId,
    name: Option<&str>,
    slug: Option<&str>,
    description: Option<String>,
    comments: Option<String>,
    is_private: Option<bool>,
    tags: Option<Vec<String>>,
) -> Result<Rir, NetBoxError> {
    let id_value: u64 = id.into();
    let url = format!("{}/api/ipam/rirs/{}/", core.base_url, id_value);
    debug!("Updating RIR {} in NetBox", id_value);
    
    let mut body = serde_json::json!({});
    
    helpers::add_optional_string_field(&mut body, "name", name);
    helpers::add_optional_string_field(&mut body, "slug", slug);
    helpers::add_optional_string_field_owned(&mut body, "description", description);
    helpers::add_optional_string_field_owned(&mut body, "comments", comments);
    helpers::add_optional_bool_field(&mut body, "is_private", is_private);
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
            "Failed to update RIR {}: {} - {}",
            id_value, status, body_text
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

