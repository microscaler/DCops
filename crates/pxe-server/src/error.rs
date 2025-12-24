//! PXE server errors

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PxeError {
    #[error("DHCP error: {0}")]
    Dhcp(String),
    
    #[error("TFTP error: {0}")]
    Tftp(String),
    
    #[error("HTTP error: {0}")]
    Http(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
}

