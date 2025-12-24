//! NetBox client errors

use thiserror::Error;

/// Errors that can occur when interacting with the NetBox API
#[derive(Debug, Error)]
pub enum NetBoxError {
    /// HTTP request/response error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    /// NetBox API returned an error
    #[error("NetBox API error: {0}")]
    Api(String),
    
    /// JSON serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// Authentication failed (invalid token, expired, etc.)
    #[error("Authentication failed: {0}")]
    Authentication(String),
    
    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),
    
    /// Invalid request (e.g., missing required fields)
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

