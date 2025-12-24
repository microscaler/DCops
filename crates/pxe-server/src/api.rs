//! API server for boot configuration.
//!
//! This module implements an API endpoint compatible with Pixiecore's
//! API mode, allowing the PXE Intent Controller to configure boot
//! intent via REST API. Supports both IPv4 and IPv6.

use crate::error::PxeError;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// API server for boot configuration with dual-stack support.
///
/// Provides REST API endpoints for configuring boot intent,
/// compatible with Pixiecore API mode (`GET /v1/boot/<mac-address>`).
/// Supports both IPv4 and IPv6.
pub struct ApiServer {
    // TODO: Add fields for dual-stack API server
    // - IPv4 listener
    // - IPv6 listener
    // - Axum router
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootConfig {
    pub kernel: String,
    #[serde(default)]
    pub initrd: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cmdline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ApiServer {
    /// Creates a new API server instance with dual-stack support.
    pub fn new() -> Result<Self, PxeError> {
        // TODO: Initialize API server for both IPv4 and IPv6
        // Use axum for HTTP API (supports dual-stack)
        // - Configure IPv4 listener
        // - Configure IPv6 listener
        todo!("Implement API server initialization with dual-stack support")
    }
    
    /// Starts the API server with dual-stack support.
    ///
    /// Listens on both IPv4 and IPv6 addresses for API requests.
    pub async fn start(&self) -> Result<()> {
        // TODO: Start API server on both IPv4 and IPv6
        // Implement GET /v1/boot/<mac-address> endpoint
        // - Start IPv4 listener
        // - Start IPv6 listener
        todo!("Implement API server start with dual-stack support")
    }
    
    /// Sets boot configuration for a MAC address.
    pub async fn set_boot_config(&self, _mac: &str, _config: BootConfig) -> Result<()> {
        // TODO: Store boot configuration
        todo!("Implement boot config storage")
    }
    
    /// Gets boot configuration for a MAC address.
    pub async fn get_boot_config(&self, _mac: &str) -> Result<Option<BootConfig>> {
        // TODO: Retrieve boot configuration
        todo!("Implement boot config retrieval")
    }
}

