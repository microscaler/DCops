//! Main PXE server implementation.
//!
//! This module orchestrates the PXE boot server, coordinating DHCP,
//! TFTP, and HTTP services with dual-stack (IPv4 and IPv6) support.

use crate::dhcp::DhcpServer;
use crate::tftp::TftpServer;
use crate::http::HttpServer;
use crate::api::ApiServer;
use crate::error::PxeError;
use anyhow::Result;
use tracing::info;

/// Main PXE boot server with dual-stack support.
///
/// Coordinates DHCP (ProxyDHCP), TFTP, and HTTP services to provide
/// complete PXE boot functionality. Supports both IPv4 and IPv6 protocols.
pub struct PxeServer {
    dhcp: DhcpServer,
    tftp: TftpServer,
    http: HttpServer,
    api: ApiServer,
}

impl PxeServer {
    /// Creates a new PXE server instance.
    pub fn new() -> Result<Self, PxeError> {
        // TODO: Initialize DHCP, TFTP, HTTP, and API servers
        todo!("Implement PXE server initialization")
    }
    
    /// Starts the PXE server with dual-stack support.
    ///
    /// This will start all services (DHCP, TFTP, HTTP, API) and run
    /// until shutdown is requested. Supports both IPv4 and IPv6.
    pub async fn start(&self) -> Result<()> {
        info!("Starting PXE boot server (IPv4 and IPv6)");
        
        // TODO: Start all services concurrently
        // - DHCP server (ProxyDHCP) - IPv4 and IPv6
        // - TFTP server - IPv4 only (protocol limitation)
        // - HTTP server (for iPXE) - IPv4 and IPv6
        // - API server (for boot configuration) - IPv4 and IPv6
        
        todo!("Implement server startup")
    }
    
    /// Shuts down the PXE server gracefully.
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down PXE boot server");
        
        // TODO: Gracefully shutdown all services
        todo!("Implement graceful shutdown")
    }
}

