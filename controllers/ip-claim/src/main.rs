//! IP Claim Controller
//!
//! Provides deterministic IP allocation without hardcoding addresses.
//!
//! This controller reconciles `IPClaim` and `IPPool` CRDs to allocate
//! IP addresses from NetBox, ensuring Git-defined IP assignments.

mod controller;
mod reconciler;
mod watcher;
mod error;

use controller::Controller;
use crate::error::ControllerError;
use tracing::info;
use std::env;

#[tokio::main]
async fn main() -> Result<(), ControllerError> {
    tracing_subscriber::fmt::init();
    
    info!("Starting IP Claim Controller");
    
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

