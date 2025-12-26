//! Query utilities for NetBox API
//!
//! Provides helpers for building queries and handling pagination.

use crate::common::{HttpClient, PaginatedResponse};
use crate::error::NetBoxError;
use serde::Deserialize;

/// Query resources with optional filtering and pagination
pub async fn query_resources<T: for<'de> Deserialize<'de>>(
    http: &HttpClient,
    endpoint: &str,
    filters: &[(&str, &str)],
    fetch_all: bool,
) -> Result<Vec<T>, NetBoxError> {
    let mut url = format!("/api/{}/", endpoint);
    
    if !filters.is_empty() {
        let query_string = http.build_query_string(filters);
        url = format!("{}?{}", url, query_string);
    }
    
    if fetch_all {
        http.fetch_all_pages(http.build_url(&url)).await
    } else {
        let response: PaginatedResponse<T> = http.get(&url).await?;
        Ok(response.results)
    }
}
