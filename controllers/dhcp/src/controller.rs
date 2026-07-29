//! Main DHCP Controller implementation
//!
//! This controller watches NetBox CRDs and syncs them to ISC Kea DHCP server.

use crate::reconciler::DhcpReconciler;
use crate::watcher::DhcpWatcher;
use crate::error::ControllerError;
use crate::kea::KeaClient;
use kube::Client;
use tracing::info;
use std::sync::Arc;

/// Main DHCP Controller
pub struct DhcpController {
    reconciler: Arc<DhcpReconciler>,
    watcher: DhcpWatcher,
}

impl DhcpController {
    /// Create a new DHCP Controller instance
    pub async fn new(kea_url: String) -> Result<Self, ControllerError> {
        info!("Initializing DHCP Controller");
        
        // Create Kubernetes client
        let kube_client = Client::try_default().await
            .map_err(|e| ControllerError::Kube(e))?;
        
        // Create Kea client
        let kea_client = Arc::new(KeaClient::new(kea_url));
        
        // Create reconciler
        let reconciler = Arc::new(DhcpReconciler::new(kube_client.clone(), kea_client.clone()));
        
        // Create watcher
        let watcher = DhcpWatcher::new(kube_client, reconciler.clone());
        
        info!("✅ DHCP Controller initialized");
        
        Ok(Self {
            reconciler,
            watcher,
        })
    }
    
    /// Run the controller until shutdown
    pub async fn run(&self) -> Result<(), ControllerError> {
        info!("Starting DHCP Controller watchers");
        
        // Perform full sync at startup (with retry logic for Kea availability)
        info!("Performing full sync of all CRDs to Kea...");
        match self.reconciler.full_sync().await {
            Ok(()) => {
                info!("✅ Full sync completed");
            }
            Err(e) => {
                use tracing::warn;
                warn!("⚠️  Full sync failed (Kea may not be available yet): {}. Will retry on next CRD change.", e);
                // Continue anyway - watchers will retry when CRDs change
            }
        }
        
        // Start watchers for event-driven sync
        self.watcher.start().await?;
        
        // Keep running until shutdown
        tokio::signal::ctrl_c().await
            .map_err(|e| ControllerError::InvalidConfig(format!("Failed to wait for shutdown signal: {}", e)))?;
        
        info!("Shutting down DHCP Controller");
        Ok(())
    }
}

