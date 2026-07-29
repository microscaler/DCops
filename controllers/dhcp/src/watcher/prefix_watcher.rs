//! Prefix Watcher - Watches NetBoxPrefix CRDs

use crate::reconciler::DhcpReconciler;
use crate::error::ControllerError;
use crds::NetBoxPrefix;
use kube::Api;
use kube_runtime::{Controller, watcher, controller::Action};
use std::sync::Arc;
use tracing::{info, error, debug};
use std::time::Duration;
use futures::StreamExt;
use crate::types::DEFAULT_REQUEUE_DURATION_SECS;

/// Watches NetBoxPrefix CRDs
pub struct PrefixWatcher {
    api: Api<NetBoxPrefix>,
    reconciler: Arc<DhcpReconciler>,
}

impl PrefixWatcher {
    /// Create a new Prefix Watcher
    pub fn new(api: Api<NetBoxPrefix>, reconciler: Arc<DhcpReconciler>) -> Self {
        Self { api, reconciler }
    }

    /// Start watching NetBoxPrefix CRDs
    pub async fn start(&self) -> Result<(), ControllerError> {
        info!("Starting NetBoxPrefix watcher");
        
        let reconciler = self.reconciler.clone();
        let reconciler_for_error = reconciler.clone();
        
        Controller::new(self.api.clone(), watcher::Config::default())
            .shutdown_on_signal()
            .run(
                move |obj: Arc<NetBoxPrefix>, _ctx: Arc<DhcpReconciler>| {
                    let reconciler = reconciler.clone();
                    async move {
                        debug!("Reconciling NetBoxPrefix: {:?}", obj.metadata.name);
                        reconciler.reconcile_prefix(&obj).await
                            .map(|_| Action::requeue(Duration::from_secs(DEFAULT_REQUEUE_DURATION_SECS)))
                            .map_err(|e| {
                                error!("Failed to reconcile NetBoxPrefix: {}", e);
                                e
                            })
                    }
                },
                move |_obj, error, _ctx| {
                    error!("Error watching NetBoxPrefix: {}", error);
                    Action::requeue(Duration::from_secs(DEFAULT_REQUEUE_DURATION_SECS))
                },
                reconciler_for_error,
            )
            .for_each(|_| futures::future::ready(()))
            .await;
        
        Ok(())
    }
}

