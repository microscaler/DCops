//! Kubernetes-backed boot configuration store.

use std::sync::Arc;

use crds::{BootIntent, BootProfile};
use kube::api::{Api, ListParams};
use kube::Client;

use crate::boot::{resolve_boot, BootResolution};
use crate::error::PxeError;

/// Loads boot intent and profile state from the Kubernetes API.
#[derive(Clone)]
pub struct K8sBootStore {
    client: Client,
}

impl K8sBootStore {
    /// Connect using in-cluster config or local kubeconfig.
    pub async fn new() -> Result<Self, PxeError> {
        let client = Client::try_default()
            .await
            .map_err(|e| PxeError::Configuration(format!("kubernetes client: {e}")))?;
        Ok(Self { client })
    }

    /// Resolve boot action for `mac` across all namespaces.
    pub async fn resolve_mac(&self, mac: &str) -> Result<BootResolution, PxeError> {
        let intents = Api::<BootIntent>::all(self.client.clone())
            .list(&ListParams::default())
            .await
            .map_err(|e| PxeError::Http(format!("list BootIntent: {e}")))?;

        let profiles = Api::<BootProfile>::all(self.client.clone())
            .list(&ListParams::default())
            .await
            .map_err(|e| PxeError::Http(format!("list BootProfile: {e}")))?;

        resolve_boot(mac, &intents.items, &profiles.items)
    }
}

/// Shared handle passed into axum state.
pub type SharedBootStore = Arc<K8sBootStore>;
