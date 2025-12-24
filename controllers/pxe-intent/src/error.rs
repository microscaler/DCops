//! Controller-specific error types.
//!
//! This module defines error types specific to the PXE Intent Controller
//! that are not covered by upstream library errors.

use thiserror::Error;

/// Errors that can occur in the PXE Intent Controller.
#[derive(Debug, Error)]
pub enum ControllerError {
    #[error("Reconciliation failed: {0}")]
    Reconciliation(String),
    
    #[error("Resource watch failed: {0}")]
    Watch(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
}

