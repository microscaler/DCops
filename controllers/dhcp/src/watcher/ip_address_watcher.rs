//! IP Address Watcher - Watches NetBoxIPAddress CRDs

use crate::reconciler::DhcpReconciler;
use crate::error::ControllerError;
use crds::NetBoxIPAddress;
use kube::Api;
use kube_runtime::{Controller, watcher, controller::Action};
use std::sync::Arc;
use tracing::{info, error, debug};
use std::time::Duration;
use futures::StreamExt;
use crate::types::DEFAULT_REQUEUE_DURATION_SECS;

/// Watches NetBoxIPAddress CRDs
pub struct IpAddressWatcher {
    api: Api<NetBoxIPAddress>,
    reconciler: Arc<DhcpReconciler>,
}

impl IpAddressWatcher {
    /// Create a new IP Address Watcher
    pub fn new(api: Api<NetBoxIPAddress>, reconciler: Arc<DhcpReconciler>) -> Self {
        Self { api, reconciler }
    }

    /// Start watching NetBoxIPAddress CRDs
    pub async fn start(&self) -> Result<(), ControllerError> {
        info!("Starting NetBoxIPAddress watcher");
        
        let reconciler = self.reconciler.clone();
        let reconciler_for_error = reconciler.clone();
        
        Controller::new(self.api.clone(), watcher::Config::default())
            .shutdown_on_signal()
            .run(
                move |obj: Arc<NetBoxIPAddress>, _ctx: Arc<DhcpReconciler>| {
                    let reconciler = reconciler.clone();
                    async move {
                        debug!("Reconciling NetBoxIPAddress: {:?}", obj.metadata.name);
                        reconciler.reconcile_ip_address(&obj).await
                            .map(|_| Action::requeue(Duration::from_secs(DEFAULT_REQUEUE_DURATION_SECS)))
                            .map_err(|e| {
                                error!("Failed to reconcile NetBoxIPAddress: {}", e);
                                e
                            })
                    }
                },
                move |_obj, error, _ctx| {
                    error!("Error watching NetBoxIPAddress: {}", error);
                    Action::requeue(Duration::from_secs(DEFAULT_REQUEUE_DURATION_SECS))
                },
                reconciler_for_error,
            )
            .for_each(|_| futures::future::ready(()))
            .await;
        
        Ok(())
    }
}

