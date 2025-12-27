//! IPAM Aggregate operations
//!
//! This module provides methods for managing NetBox IPAM aggregates.

use crate::common::PaginatedResponse;
use crate::core::{NetBoxClientCore, helpers};
use crate::error::NetBoxError;
use crate::models::Aggregate;
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
pub async fn get_aggregate(core: &NetBoxClientCore, id: u64) -> Result<Aggregate, NetBoxError> {
    let url = format!("{}/api/ipam/aggregates/{}/", core.base_url, id);
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
            "Failed to get aggregate {}: {} - {}",
            id, status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

/// Create a new aggregate
pub async fn create_aggregate(
    core: &NetBoxClientCore,
    prefix: &str,
    rir_id: Option<u64>, // RIR ID (not name)
    date_allocated: Option<&str>, // ISO 8601 date
    description: Option<String>,
    comments: Option<String>,
) -> Result<Aggregate, NetBoxError> {
    let url = format!("{}/api/ipam/aggregates/", core.base_url);
    debug!("Creating aggregate {} in NetBox", prefix);
    
    let mut body = serde_json::json!({
        "prefix": prefix,
    });
    
    // RIR is required for aggregates - must be provided
    let rir_id_value = rir_id.ok_or_else(|| NetBoxError::Api(
        "RIR is required for aggregates but was not provided".to_string()
    ))?;
    helpers::add_required_nested_reference(&mut body, "rir", rir_id_value);
    
    helpers::add_optional_string_field(&mut body, "date_allocated", date_allocated);
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
            "Failed to create aggregate: {} - {}",
            status, body
        )));
    }
    
    response.json().await.map_err(|e| NetBoxError::Http(e))
}

