//! Core NetBox client infrastructure
//!
//! This module provides the core `NetBoxClientCore` struct that contains
//! the HTTP client, base URL, and authentication token. It also provides
//! shared utilities like pagination and token validation.

use crate::common::PaginatedResponse;
use crate::error::NetBoxError;
use reqwest::Client;
use std::time::Duration;
use tracing::debug;

/// Core NetBox client infrastructure
///
/// This struct contains the essential components needed for making
/// NetBox API requests: HTTP client, base URL, and authentication token.
pub struct NetBoxClientCore {
    pub(crate) client: Client,
    pub(crate) base_url: String,
    pub(crate) token: String,
}

impl NetBoxClientCore {
    /// Create a new NetBox client core
    ///
    /// # Arguments
    /// * `base_url` - NetBox base URL (e.g., "http://netbox:80")
    /// * `token` - API token for authentication
    pub fn new(base_url: String, token: String) -> Result<Self, NetBoxError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| NetBoxError::Http(e))?;
        
        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
        })
    }
    
    /// Validate the API token by making a simple authenticated request.
    ///
    /// This method tests connectivity and token validity before proceeding with operations.
    /// It makes a lightweight request to the NetBox status endpoint.
    ///
    /// # Returns
    /// * `Ok(())` - Token is valid and NetBox is reachable
    /// * `Err(NetBoxError)` - Token is invalid or NetBox is unreachable
    pub async fn validate_token(&self) -> Result<(), NetBoxError> {
        // Use the status endpoint as it's lightweight and requires authentication
        let url = format!("{}/api/status/", self.base_url);
        debug!("Validating NetBox token and connectivity");
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        
        if status == 401 || status == 403 {
            return Err(NetBoxError::Api(format!(
                "Invalid token: {} - {}",
                status,
                body
            )));
        }
        
        if !status.is_success() {
            return Err(NetBoxError::Api(format!(
                "Failed to validate token: {} - {}",
                status, body
            )));
        }
        
        debug!("Token validated successfully");
        Ok(())
    }
    
    /// Fetch all pages of a paginated response
    pub async fn fetch_all_pages<T: for<'de> serde::Deserialize<'de>>(
        &self,
        mut url: String,
    ) -> Result<Vec<T>, NetBoxError> {
        let mut all_results = Vec::new();
        
        loop {
            debug!("Fetching page: {}", url);
            
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to fetch page: {} - {}",
                    status, body
                )));
            }
            
            // Try to deserialize, but capture the response body for better error messages
            let response_text = response.text().await?;
            let page: PaginatedResponse<T> = serde_json::from_str(&response_text).map_err(|e| {
                NetBoxError::Api(format!(
                    "error decoding response body: {} - Response (first 500 chars): {}",
                    e,
                    response_text.chars().take(500).collect::<String>()
                ))
            })?;
            all_results.extend(page.results);
            
            // Check if there's a next page
            match page.next {
                Some(next_url) => {
                    // Extract the path from the full URL
                    url = if next_url.starts_with("http") {
                        next_url
                    } else {
                        format!("{}{}", self.base_url, next_url)
                    };
                }
                None => break,
            }
        }
        
        Ok(all_results)
    }
    
    /// Get a reference to the HTTP client
    pub fn client(&self) -> &Client {
        &self.client
    }
    
    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
    
    /// Get the authentication token
    pub fn token(&self) -> &str {
        &self.token
    }
}

pub mod helpers;

