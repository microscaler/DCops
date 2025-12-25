//! Controller-specific error types.
//!
//! This module defines error types specific to the unified NetBox Controller
//! that are not covered by upstream library errors.

use thiserror::Error;
use kube::Error as KubeError;
use netbox_client::NetBoxError;

/// Errors that can occur in the NetBox Controller.
#[derive(Debug, Error)]
pub enum ControllerError {
    /// Kubernetes API error
    #[error("Kubernetes error: {0}")]
    Kube(#[from] KubeError),
    
    /// NetBox API error
    #[error("NetBox error: {0}")]
    NetBox(#[from] NetBoxError),
    
    /// IPPool not found
    #[error("IPPool not found: {0}")]
    IPPoolNotFound(String),
    
    /// IPClaim not found
    #[error("IPClaim not found: {0}")]
    #[allow(dead_code)] // Reserved for future use
    IPClaimNotFound(String),
    
    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    /// NetBox prefix not found
    #[error("NetBox prefix not found: {0}")]
    PrefixNotFound(String),
    
    /// No available IPs in pool
    #[error("No available IPs in pool: {0}")]
    #[allow(dead_code)] // Reserved for future use
    NoAvailableIPs(String),
    
    /// IP allocation failed
    #[error("IP allocation failed: {0}")]
    AllocationFailed(String),
    
    /// Invalid IP address format
    #[error("Invalid IP address format: {0}")]
    #[allow(dead_code)] // Reserved for future use
    InvalidIPFormat(String),
    
    /// Reconciliation failed
    #[error("Reconciliation failed: {0}")]
    #[allow(dead_code)] // Reserved for future use
    Reconciliation(String),
    
    /// Resource watch failed
    #[error("Resource watch failed: {0}")]
    Watch(String),
}

