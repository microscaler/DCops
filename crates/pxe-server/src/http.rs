//! HTTP server for iPXE boot.
//!
//! This module implements an HTTP server for serving boot files
//! to iPXE clients, which prefer HTTP over TFTP for faster transfers.
//! Supports both IPv4 and IPv6.

use crate::error::PxeError;
use anyhow::Result;

/// HTTP server for iPXE boot file delivery with dual-stack support.
///
/// Serves kernel images, initrd files, and iPXE scripts
/// via HTTP protocol for faster boot times. Supports both IPv4 and IPv6.
pub struct HttpServer {
    // TODO: Add fields for dual-stack HTTP server
    // - IPv4 listener
    // - IPv6 listener
    // - Axum router
}

impl HttpServer {
    /// Creates a new HTTP server instance with dual-stack support.
    pub fn new() -> Result<Self, PxeError> {
        // TODO: Initialize HTTP server for both IPv4 and IPv6
        // Use axum for HTTP server (supports dual-stack)
        // - Configure IPv4 listener
        // - Configure IPv6 listener
        todo!("Implement HTTP server initialization with dual-stack support")
    }
    
    /// Starts the HTTP server with dual-stack support.
    ///
    /// Listens on both IPv4 and IPv6 addresses for HTTP requests.
    pub async fn start(&self) -> Result<()> {
        // TODO: Start HTTP server on both IPv4 and IPv6
        // Serve boot files and iPXE scripts
        // - Start IPv4 listener
        // - Start IPv6 listener
        todo!("Implement HTTP server start with dual-stack support")
    }
}

