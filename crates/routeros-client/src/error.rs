//! RouterOS client errors

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RouterOSError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("RouterOS API error: {0}")]
    Api(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Authentication failed")]
    Authentication,
}

