//! IPAM Aggregate operations
//!
//! This module provides methods for managing NetBox IPAM aggregates.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::Aggregate;
use crate::types::*;
use tracing::debug;

/// Query aggregates by filters
pub async fn query_aggregates(
    core: &NetBoxClientCore,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<Aggregate>, NetBoxError> {
    let mut url = format!("{}/api/ipam/aggregates/", core.base_url);
    
    if !filters.is_empty() {
        let query: Vec<String> = filters.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        url = format!("{}?{}", url, query.join("&"));
    }
    
    debug!("Querying aggregates with filters: {:?}", filters);
    
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
                "Failed to query aggregates: {} - {}",
                status, body
            )));
        }
        
        let result: PaginatedResponse<Aggregate> = response.json().await?;
        Ok(result.results)
    }
}

/// Get aggregate by ID
pub async fn get_aggregate(core: &NetBoxClientCore, id: AggregateId) -> Result<Aggregate, NetBoxError> {
    let id_value: u64 = id.into();
    let url = format!("{}/api/ipam/aggregates/{}/", core.base_url, id_value);
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
                "Aggregate {} not found: {}",
                id_value, body
            )));
        }
        
        return Err(NetBoxError::Api(format!(
            "Failed to get aggregate {}: {} - {}",
            id_value, status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Create a new aggregate
pub async fn create_aggregate(
    core: &NetBoxClientCore,
    prefix: &ipnet::IpNet,
    rir_id: Option<RirId>,
    date_allocated: Option<&str>, // ISO 8601 date
    description: Option<String>,
    comments: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<Aggregate, NetBoxError> {
    let url = format!("{}/api/ipam/aggregates/", core.base_url);
    let prefix_str = prefix.to_string();
    debug!("Creating aggregate {} in NetBox", prefix_str);
    
    let mut body = serde_json::json!({
        "prefix": prefix_str.clone(),
    });
    
    // RIR may be optional depending on deployment; if missing, proceed and let NetBox validation decide.
    // Explicitly log the absence for visibility during reconciliation.
    let rir_id_value: Option<u64> = rir_id.map(|id| id.into());
    if rir_id_value.is_none() {
        debug!("Creating aggregate {} without RIR (none provided/found)", prefix);
    }
    helpers::add_nested_reference(&mut body, "rir", rir_id_value);
    
    helpers::add_optional_string_field(&mut body, "date_allocated", date_allocated);
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
            "Failed to create aggregate: {} - {}",
            status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Update an existing aggregate
pub async fn update_aggregate(
    core: &NetBoxClientCore,
    id: AggregateId,
    rir_id: Option<RirId>,
    date_allocated: Option<&str>, // ISO 8601 date
    description: Option<String>,
    comments: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<Aggregate, NetBoxError> {
    let id_value: u64 = id.into();
    let url = format!("{}/api/ipam/aggregates/{}/", core.base_url, id_value);
    debug!("Updating aggregate {} in NetBox", id_value);
    
    let mut body = serde_json::json!({});
    
    let rir_id_value: Option<u64> = rir_id.map(|id| id.into());
    helpers::add_nested_reference(&mut body, "rir", rir_id_value);
    
    helpers::add_optional_string_field(&mut body, "date_allocated", date_allocated);
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
            "Failed to update aggregate {}: {} - {}",
            id_value, status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

