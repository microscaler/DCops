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
mod kube_api_trait;
mod token_resolver;
mod secret_fetcher;
mod events;
#[cfg(test)]
mod events_test;
#[cfg(test)]
mod test_utils;
#[cfg(test)]
mod reconcile_helpers_test;

use controller::Controller;
use crate::error::ControllerError;
use tracing::info;
use std::env;

// Build-time constants (set during Docker build)
// Use a macro to handle the unwrap_or in a const context
macro_rules! build_env {
    ($name:expr, $default:expr) => {
        match option_env!($name) {
            Some(val) => val,
            None => $default,
        }
    };
}

const GIT_HASH: &str = build_env!("BUILD_GIT_HASH", "unknown");
const BUILD_TIMESTAMP: &str = build_env!("BUILD_TIMESTAMP", "unknown");
const BUILD_DATETIME: &str = build_env!("BUILD_DATETIME", "unknown");

#[tokio::main]
async fn main() -> Result<(), ControllerError> {
    tracing_subscriber::fmt::init();
    
    info!("Starting NetBox Controller");
    info!("Version: git_hash={}, build_timestamp={}, build_datetime={}", 
          GIT_HASH, BUILD_TIMESTAMP, BUILD_DATETIME);
    
    // Load configuration from environment variables
    let netbox_url = env::var("NETBOX_URL")
        .unwrap_or_else(|_| "http://netbox.netbox:80".to_string());
    let namespace = env::var("WATCH_NAMESPACE").ok();
    
    info!("Configuration:");
    info!("  NetBox URL: {}", netbox_url);
    info!("  Namespace: {}", namespace.as_deref().unwrap_or("all namespaces"));
    info!("  Multi-Tenant Mode: Enabled (tokens resolved from Tenant CRDs)");
    
    // Initialize and run controller
    // Note: NETBOX_TOKEN is no longer required - tokens are resolved from Tenant CRDs
    let controller = Controller::new(netbox_url, namespace).await?;
    controller.run().await?;
    
    Ok(())
}

