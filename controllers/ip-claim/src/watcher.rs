//! Kubernetes resource watchers.
//!
//! This module handles watching Kubernetes resources for changes
//! and triggering reconciliation.

use crate::reconciler::Reconciler;
use crate::error::ControllerError;
use crds::{IPClaim, IPPool};
use kube::Api;
use std::sync::Arc;
use tracing::{info, error, warn, debug};
use futures::TryStreamExt;
use kube_runtime::watcher;

/// Watches Kubernetes resources for changes.
pub struct Watcher {
    reconciler: Arc<Reconciler>,
    ip_claim_api: Api<IPClaim>,
    ip_pool_api: Api<IPPool>,
}

impl Watcher {
    /// Creates a new watcher instance.
    pub fn new(
        reconciler: Arc<Reconciler>,
        ip_claim_api: Api<IPClaim>,
        ip_pool_api: Api<IPPool>,
    ) -> Self {
        Self {
            reconciler,
            ip_claim_api,
            ip_pool_api,
        }
    }
    
    /// Starts watching IPClaim resources.
    pub async fn watch_ip_claims(&self) -> Result<(), ControllerError> {
        info!("Starting IPClaim watcher");
        
        let mut stream = Box::pin(watcher(self.ip_claim_api.clone(), watcher::Config::default()));
        
        while let Some(result) = stream.try_next().await
            .map_err(|e| ControllerError::Watch(format!("Watcher stream error: {}", e)))?
        {
            match result {
                watcher::Event::Apply(claim) => {
                    let name = claim.metadata.name.as_deref()
                        .unwrap_or("<unknown>");
                    info!("IPClaim applied: {}", name);
                    
                    if let Err(e) = self.reconciler.reconcile_ip_claim(&claim).await {
                        error!("Failed to reconcile IPClaim {}: {}", name, e);
                    }
                }
                watcher::Event::Delete(claim) => {
                    let name = claim.metadata.name.as_deref()
                        .unwrap_or("<unknown>");
                    info!("IPClaim deleted: {}", name);
                    // TODO: Handle deletion (release IP in NetBox?)
                }
                watcher::Event::Init => {
                    info!("IPClaim watcher initialized");
                }
                watcher::Event::InitApply(claim) => {
                    let name = claim.metadata.name.as_deref()
                        .unwrap_or("<unknown>");
                    debug!("IPClaim init apply: {}", name);
                    
                    if let Err(e) = self.reconciler.reconcile_ip_claim(&claim).await {
                        warn!("Failed to reconcile IPClaim {}: {}", name, e);
                    }
                }
                watcher::Event::InitDone => {
                    info!("IPClaim watcher initialization complete");
                }
            }
        }
        
        Ok(())
    }
    
    /// Starts watching IPPool resources.
    pub async fn watch_ip_pools(&self) -> Result<(), ControllerError> {
        info!("Starting IPPool watcher");
        
        let mut stream = Box::pin(watcher(self.ip_pool_api.clone(), watcher::Config::default()));
        
        while let Some(result) = stream.try_next().await
            .map_err(|e| ControllerError::Watch(format!("Watcher stream error: {}", e)))?
        {
            match result {
                watcher::Event::Apply(pool) => {
                    let name = pool.metadata.name.as_deref()
                        .unwrap_or("<unknown>");
                    debug!("IPPool applied: {}", name);
                    
                    if let Err(e) = self.reconciler.reconcile_ip_pool(&pool).await {
                        warn!("Failed to reconcile IPPool {}: {}", name, e);
                    }
                }
                watcher::Event::Delete(pool) => {
                    let name = pool.metadata.name.as_deref()
                        .unwrap_or("<unknown>");
                    info!("IPPool deleted: {}", name);
                }
                watcher::Event::Init => {
                    debug!("IPPool watcher initialized");
                }
                watcher::Event::InitApply(pool) => {
                    let name = pool.metadata.name.as_deref()
                        .unwrap_or("<unknown>");
                    debug!("IPPool init apply: {}", name);
                    
                    if let Err(e) = self.reconciler.reconcile_ip_pool(&pool).await {
                        warn!("Failed to reconcile IPPool {}: {}", name, e);
                    }
                }
                watcher::Event::InitDone => {
                    debug!("IPPool watcher initialization complete");
                }
            }
        }
        
        Ok(())
    }
}
