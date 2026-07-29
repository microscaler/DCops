//! IP Range Watcher - Watches NetBoxIPRange CRDs

use crate::reconciler::DhcpReconciler;
use crate::error::ControllerError;
use crds::NetBoxIPRange;
use kube::Api;
use kube_runtime::{Controller, watcher, controller::Action};
use std::sync::Arc;
use tracing::{info, error, debug};
use std::time::Duration;
use futures::StreamExt;
use crate::types::DEFAULT_REQUEUE_DURATION_SECS;

/// Watches NetBoxIPRange CRDs
pub struct IpRangeWatcher {
    api: Api<NetBoxIPRange>,
    reconciler: Arc<DhcpReconciler>,
}

impl IpRangeWatcher {
    /// Create a new IP Range Watcher
    pub fn new(api: Api<NetBoxIPRange>, reconciler: Arc<DhcpReconciler>) -> Self {
        Self { api, reconciler }
    }

    /// Start watching NetBoxIPRange CRDs
    pub async fn start(&self) -> Result<(), ControllerError> {
        info!("Starting NetBoxIPRange watcher");
        
        let reconciler = self.reconciler.clone();
        let reconciler_for_error = reconciler.clone();
        
        Controller::new(self.api.clone(), watcher::Config::default())
            .shutdown_on_signal()
            .run(
                move |obj: Arc<NetBoxIPRange>, _ctx: Arc<DhcpReconciler>| {
                    let reconciler = reconciler.clone();
                    async move {
                        debug!("Reconciling NetBoxIPRange: {:?}", obj.metadata.name);
                        reconciler.reconcile_ip_range(&obj).await
                            .map(|_| Action::requeue(Duration::from_secs(DEFAULT_REQUEUE_DURATION_SECS)))
                            .map_err(|e| {
                                error!("Failed to reconcile NetBoxIPRange: {}", e);
                                e
                            })
                    }
                },
                move |_obj, error, _ctx| {
                    error!("Error watching NetBoxIPRange: {}", error);
                    Action::requeue(Duration::from_secs(DEFAULT_REQUEUE_DURATION_SECS))
                },
                reconciler_for_error,
            )
            .for_each(|_| futures::future::ready(()))
            .await;
        
        Ok(())
    }
}

