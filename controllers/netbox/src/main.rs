//! NetBox Controller
//!
//! Unified controller for managing all NetBox-related CRDs:
//! - NetBoxPrefix: Creates and manages prefixes in NetBox
//! - IPPool: Manages IP address pools (references NetBoxPrefix)
//! - IPClaim: Allocates IP addresses from IPPools via NetBox
//!
//! This controller ensures GitOps-style management of NetBox IPAM resources.

mod controller;
mod reconciler;
mod watcher;
mod error;
mod backoff;
mod reconcile_helpers;

use controller::Controller;
use crate::error::ControllerError;
use tracing::info;
use std::env;

#[tokio::main]
async fn main() -> Result<(), ControllerError> {
    tracing_subscriber::fmt::init();
    
    info!("Starting NetBox Controller");
    
    // Load configuration from environment variables
    let netbox_url = env::var("NETBOX_URL")
        .unwrap_or_else(|_| "http://netbox.netbox:80".to_string());
    let netbox_token = env::var("NETBOX_TOKEN")
        .map_err(|_| ControllerError::InvalidConfig(
            "NETBOX_TOKEN environment variable is required".to_string()
        ))?;
    let namespace = env::var("WATCH_NAMESPACE").ok();
    
    info!("Configuration:");
    info!("  NetBox URL: {}", netbox_url);
    info!("  Namespace: {}", namespace.as_deref().unwrap_or("all namespaces"));
    
    // Initialize and run controller
    let controller = Controller::new(netbox_url, netbox_token, namespace).await?;
    controller.run().await?;
    
    Ok(())
}

