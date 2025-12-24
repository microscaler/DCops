//! PXE Boot Server
//!
//! Custom Rust PXE boot server implementation using `dhcproto`, `async-tftp`, and `axum`.
//!
//! This server provides:
//! - ProxyDHCP support for PXE boot (IPv4 and IPv6)
//! - TFTP server for kernel/initrd delivery (IPv4)
//! - HTTP server for iPXE boot files (IPv4 and IPv6)
//! - API endpoint for boot configuration (compatible with Pixiecore API mode, IPv4 and IPv6)
//!
//! # Dual-Stack Support
//!
//! The server supports both IPv4 and IPv6 from the start:
//! - **DHCP/ProxyDHCP**: Supports both DHCPv4 and DHCPv6
//! - **HTTP/iPXE**: Supports both IPv4 and IPv6
//! - **TFTP**: IPv4 only (TFTP protocol limitation)
//! - **API**: Supports both IPv4 and IPv6

pub mod server;
pub mod dhcp;
pub mod tftp;
pub mod http;
pub mod api;
pub mod error;

pub use server::*;
pub use error::*;

