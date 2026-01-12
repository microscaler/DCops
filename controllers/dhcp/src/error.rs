//! Error types for DHCP Controller

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ControllerError {
    #[error("Kubernetes error: {0}")]
    Kube(#[from] kube::Error),
    
    #[error("Kea API error: {0}")]
    KeaApi(String),
    
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

