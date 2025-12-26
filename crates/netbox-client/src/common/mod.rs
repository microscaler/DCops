//! Common utilities for NetBox API client
//!
//! Provides shared functionality used across all API modules.

pub mod query;

use crate::error::NetBoxError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Paginated response wrapper from NetBox API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub count: u64,
    pub next: Option<String>,
    pub previous: Option<String>,
    pub results: Vec<T>,
}

/// HTTP client wrapper with authentication
pub struct HttpClient {
    client: Client,
    base_url: String,
    token: String,
}

impl HttpClient {
    /// Create a new HTTP client wrapper
    pub fn new(client: Client, base_url: String, token: String) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
        }
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Build a full URL from a path
    pub fn build_url(&self, path: &str) -> String {
        if path.starts_with("http") {
            path.to_string()
        } else {
            format!("{}{}", self.base_url, path)
        }
    }

    /// Get authorization header value
    pub fn auth_header(&self) -> String {
        format!("Token {}", self.token)
    }

    /// Get the underlying HTTP client
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Fetch all pages of a paginated response
    pub async fn fetch_all_pages<T: for<'de> Deserialize<'de>>(
        &self,
        mut url: String,
    ) -> Result<Vec<T>, NetBoxError> {
        let mut all_results = Vec::new();
        
        loop {
            debug!("Fetching page: {}", url);
            
            let response = self.client
                .get(&url)
                .header("Authorization", self.auth_header())
                .header("Accept", "application/json")
                .send()
                .await
                .map_err(NetBoxError::Http)?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to fetch page: {} - {}",
                    status, body
                )));
            }
            
            let response_text = response.text().await?;
            let page: PaginatedResponse<T> = serde_json::from_str(&response_text).map_err(|e| {
                NetBoxError::Api(format!(
                    "error decoding response body: {} - Response (first 500 chars): {}",
                    e,
                    response_text.chars().take(500).collect::<String>()
                ))
            })?;
            all_results.extend(page.results);
            
            match page.next {
                Some(next_url) => {
                    url = self.build_url(&next_url);
                }
                None => break,
            }
        }
        
        Ok(all_results)
    }

    /// Make a GET request
    pub async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
    ) -> Result<T, NetBoxError> {
        let url = self.build_url(path);
        debug!("GET {}", url);
        
        let response = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(NetBoxError::Http)?;
        
        let status = response.status();
        if status == 404 {
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::NotFound(format!(
                "Resource not found: {} - {}",
                path, body
            )));
        }
        
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "GET {} failed: {} - {}",
                path, status, body
            )));
        }
        
        response.json().await.map_err(NetBoxError::Http)
    }

    /// Make a POST request
    pub async fn post<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T, NetBoxError> {
        let url = self.build_url(path);
        debug!("POST {} with body: {}", url, serde_json::to_string_pretty(body).unwrap_or_default());
        
        let response = self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(NetBoxError::Http)?;
        
        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "POST {} failed: {} - {}",
                path, status, body_text
            )));
        }
        
        response.json().await.map_err(NetBoxError::Http)
    }

    /// Make a PATCH request
    pub async fn patch<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T, NetBoxError> {
        let url = self.build_url(path);
        debug!("PATCH {} with body: {}", url, serde_json::to_string_pretty(body).unwrap_or_default());
        
        let response = self.client
            .patch(&url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(NetBoxError::Http)?;
        
        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "PATCH {} failed: {} - {}",
                path, status, body_text
            )));
        }
        
        response.json().await.map_err(NetBoxError::Http)
    }

    /// Make a DELETE request
    pub async fn delete(&self, path: &str) -> Result<(), NetBoxError> {
        let url = self.build_url(path);
        debug!("DELETE {}", url);
        
        let response = self.client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(NetBoxError::Http)?;
        
        let status = response.status();
        if !status.is_success() && status != 204 {
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "DELETE {} failed: {} - {}",
                path, status, body
            )));
        }
        
        Ok(())
    }

    /// Build query string from filters
    pub fn build_query_string(&self, filters: &[(&str, &str)]) -> String {
        if filters.is_empty() {
            String::new()
        } else {
            filters
                .iter()
                .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                .collect::<Vec<_>>()
                .join("&")
        }
    }
}
