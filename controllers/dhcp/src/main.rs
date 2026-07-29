//! DHCP Controller
//!
//! Syncs NetBox CRDs (NetBoxPrefix, NetBoxIPRange, NetBoxIPAddress) to ISC Kea DHCP server.
//!
//! This controller:
//! 1. Watches NetBox CRDs via Kubernetes watch API
//! 2. Translates CRD data to Kea configuration format
//! 3. Pushes configuration to Kea via Control Agent REST API
//! 4. Reacts instantly to CRD status changes (event-driven, no polling)

mod controller;
mod reconciler;
mod watcher;
mod error;
mod kea;
mod types;

use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    info!("Starting DHCP Controller");
    
    // Load configuration from environment
    let kea_url = std::env::var("KEA_CONTROL_AGENT_URL")
        .unwrap_or_else(|_| types::KEA_CONTROL_AGENT_DEFAULT_URL.to_string());
    
    info!("Kea Control Agent URL: {}", kea_url);
    
    info!("Kea Control Agent URL: {}", kea_url);
    
    // Initialize controller
    let controller = controller::DhcpController::new(kea_url).await?;
    
    // Start controller
    controller.run().await?;
    
    Ok(())
}

