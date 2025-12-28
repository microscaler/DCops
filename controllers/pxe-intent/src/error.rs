//! Controller-specific error types.
//!
//! This module defines error types specific to the PXE Intent Controller
//! that are not covered by upstream library errors.
//!
//! **Status:** Phase 2+ (stub implementation - not yet implemented)

use thiserror::Error;

/// Errors that can occur in the PXE Intent Controller.
/// 
/// **Phase 2+ Stub:** This is a placeholder for future implementation.
#[derive(Debug, Error)]
#[allow(dead_code)] // Stub implementation - will be used when controller is implemented
pub enum Phase2StubControllerError {
    #[error("Reconciliation failed: {0}")]
    Reconciliation(String),
    
    #[error("Resource watch failed: {0}")]
    Watch(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
}

