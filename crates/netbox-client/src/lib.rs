//! NetBox REST API Client
//!
//! A Rust client library for interacting with the NetBox REST API.
//! Provides type-safe models and methods for IPAM, DCIM, and network operations.
//!
//! # Example
//!
//! ```no_run
//! use netbox_client::{NetBoxClient, AllocateIPRequest, IPAddressStatus};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a client
//! let client = NetBoxClient::new(
//!     "http://netbox:80".to_string(),
//!     "your-api-token".to_string(),
//! )?;
//!
//! // Query prefixes
//! let prefixes = client.query_prefixes(&[("status", "active")], false).await?;
//!
//! // Allocate an IP address from a prefix
//! let request = AllocateIPRequest {
//!     address: None,
//!     description: Some("PXE boot server".to_string()),
//!     status: Some(IPAddressStatus::Active),
//!     role: None,
//!     dns_name: None,
//!     tags: None,
//! };
//! let ip = client.allocate_ip(1, Some(request)).await?;
//!
//! // Query devices by MAC address
//! let device = client.get_device_by_mac("aa:bb:cc:dd:ee:ff").await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Features
//!
//! - **IPAM Operations**: Query and manage prefixes, IP addresses
//! - **DCIM Operations**: Query devices, interfaces
//! - **VLAN Management**: Query VLANs for network configuration
//! - **Retry Logic**: Automatic retry with exponential backoff
//! - **Pagination**: Support for fetching all pages of large result sets

pub mod client;
pub mod common;
pub mod error;
pub mod models;
#[path = "trait.rs"]
pub mod netbox_trait;
#[cfg(feature = "test-util")]
pub mod mock;

pub use client::NetBoxClient;
pub use common::{HttpClient, PaginatedResponse};
pub use error::NetBoxError;
pub use models::*;
pub use netbox_trait::NetBoxClientTrait;
#[cfg(feature = "test-util")]
pub use mock::MockNetBoxClient;
