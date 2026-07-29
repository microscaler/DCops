//! PXE Boot Server
//!
//! Custom Rust PXE boot server. Phase 2 lab delivers HTTP/iPXE first; Kea provides DHCP.
//! ProxyDHCP and TFTP modules are reserved for later milestones.

pub mod api;
pub mod boot;
pub mod config;
pub mod dhcp;
pub mod error;
pub mod http;
pub mod server;
pub mod store;
pub mod tftp;

pub use config::ServerConfig;
pub use error::PxeError;
pub use http::serve as serve_http;
pub use store::K8sBootStore;
