//! Main controller implementation.
//!
//! This module contains the `Controller` struct that orchestrates
//! reconciliation and resource watching for the IP Claim Controller.

use crate::reconciler::Reconciler;
use crate::watcher::Watcher;
use crate::error::ControllerError;
use crds::{IPClaim, IPPool};
use kube::{Api, Client};
use netbox_client::NetBoxClient;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::info;

/// Main controller for IP Claim management.
pub struct Controller {
    ip_claim_watcher: JoinHandle<Result<(), ControllerError>>,
    ip_pool_watcher: JoinHandle<Result<(), ControllerError>>,
}

impl Controller {
    /// Creates a new controller instance.
    pub async fn new(
        netbox_url: String,
        netbox_token: String,
        namespace: Option<String>,
    ) -> Result<Self, ControllerError> {
        info!("Initializing IP Claim Controller");
        
        // Create Kubernetes client
        let kube_client = Client::try_default().await
            .map_err(|e| ControllerError::Kube(e.into()))?;
        
        // Create NetBox client
        let netbox_client = NetBoxClient::new(netbox_url, netbox_token)
            .map_err(|e| ControllerError::NetBox(e))?;
        
        // Create API clients
        let ns = namespace.as_deref().unwrap_or("default");
        let ip_claim_api: Api<IPClaim> = Api::namespaced(kube_client.clone(), ns);
        let ip_pool_api: Api<IPPool> = Api::namespaced(kube_client.clone(), ns);
        
        // Create reconciler
        let reconciler = Reconciler::new(
            netbox_client,
            ip_claim_api.clone(),
            ip_pool_api.clone(),
        );
        
        // Create watchers - use Arc to share reconciler
        let reconciler_arc = Arc::new(reconciler);
        
        let ip_claim_watcher_instance = Watcher::new(
            reconciler_arc.clone(),
            ip_claim_api.clone(),
            ip_pool_api.clone(),
        );
        
        let ip_pool_watcher_instance = Watcher::new(
            reconciler_arc,
            ip_claim_api,
            ip_pool_api,
        );
        
        // Start watchers in background tasks
        let ip_claim_watcher = tokio::spawn(async move {
            ip_claim_watcher_instance.watch_ip_claims().await
        });
        
        let ip_pool_watcher = tokio::spawn(async move {
            ip_pool_watcher_instance.watch_ip_pools().await
        });
        
        Ok(Self {
            ip_claim_watcher,
            ip_pool_watcher,
        })
    }
    
    /// Runs the controller until shutdown.
    pub async fn run(mut self) -> Result<(), ControllerError> {
        info!("IP Claim Controller running");
        
        // Wait for either watcher to exit (they should run forever)
        tokio::select! {
            result = &mut self.ip_claim_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("IPClaim watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("IPClaim watcher error: {}", e)))?;
            }
            result = &mut self.ip_pool_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("IPPool watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("IPPool watcher error: {}", e)))?;
            }
        }
        
        Ok(())
    }
}


