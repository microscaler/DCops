//! TFTP server implementation.
//!
//! This module implements a TFTP server for serving kernel and initrd
//! files during PXE boot.
//!
//! **Note:** TFTP protocol is IPv4-only. For IPv6 boot, clients should
//! use HTTP/iPXE instead.

use crate::error::PxeError;
use anyhow::Result;

/// TFTP server for PXE boot file delivery (IPv4 only).
///
/// Serves kernel images, initrd files, and other boot files
/// via TFTP protocol. TFTP is IPv4-only; IPv6 clients should use
/// HTTP/iPXE for boot file delivery.
pub struct TftpServer {
    // TODO: Add fields for IPv4 listener
}

impl TftpServer {
    /// Creates a new TFTP server instance (IPv4 only).
    pub fn new() -> Result<Self, PxeError> {
        // TODO: Initialize TFTP server (IPv4 only)
        // Use async-tftp crate for TFTP protocol handling
        // TFTP protocol is IPv4-only by specification
        todo!("Implement TFTP server initialization (IPv4 only)")
    }
    
    /// Starts the TFTP server (IPv4 only).
    pub async fn start(&self) -> Result<()> {
        // TODO: Start listening for TFTP requests on IPv4 (port 69)
        // Serve kernel/initrd files
        // Note: IPv6 clients should use HTTP/iPXE instead
        todo!("Implement TFTP server start (IPv4 only)")
    }
}

