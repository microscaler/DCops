//! DHCP Controller Watcher module
//!
//! Watches NetBox CRDs and triggers reconciliation when they change.

mod prefix_watcher;
mod ip_range_watcher;
mod ip_address_watcher;

use crate::reconciler::DhcpReconciler;
use crate::error::ControllerError;
use crds::{NetBoxPrefix, NetBoxIPRange, NetBoxIPAddress};
use kube::{Api, Client};
use std::sync::Arc;
use tracing::error;

pub use prefix_watcher::PrefixWatcher;
pub use ip_range_watcher::IpRangeWatcher;
pub use ip_address_watcher::IpAddressWatcher;

/// DHCP Controller Watcher
pub struct DhcpWatcher {
    kube_client: Client,
    reconciler: Arc<DhcpReconciler>,
}

impl DhcpWatcher {
    /// Create a new DHCP Watcher
    pub fn new(kube_client: Client, reconciler: Arc<DhcpReconciler>) -> Self {
        Self {
            kube_client,
            reconciler,
        }
    }
    
    /// Start all watchers
    pub async fn start(&self) -> Result<(), ControllerError> {
        let namespace = std::env::var("WATCH_NAMESPACE")
            .unwrap_or_else(|_| "default".to_string());
        
        // Create API clients
        let prefix_api: Api<NetBoxPrefix> = Api::namespaced(self.kube_client.clone(), &namespace);
        let ip_range_api: Api<NetBoxIPRange> = Api::namespaced(self.kube_client.clone(), &namespace);
        let ip_address_api: Api<NetBoxIPAddress> = Api::namespaced(self.kube_client.clone(), &namespace);
        
        // Create individual watchers
        let prefix_watcher = PrefixWatcher::new(prefix_api, self.reconciler.clone());
        let ip_range_watcher = IpRangeWatcher::new(ip_range_api, self.reconciler.clone());
        let ip_address_watcher = IpAddressWatcher::new(ip_address_api, self.reconciler.clone());
        
        // Start watchers for each CRD type
        let prefix_watcher_task = prefix_watcher.start();
        let ip_range_watcher_task = ip_range_watcher.start();
        let ip_address_watcher_task = ip_address_watcher.start();
        
        // Run all watchers concurrently
        tokio::select! {
            result = prefix_watcher_task => {
                error!("Prefix watcher exited: {:?}", result);
                result
            }
            result = ip_range_watcher_task => {
                error!("IP range watcher exited: {:?}", result);
                result
            }
            result = ip_address_watcher_task => {
                error!("IP address watcher exited: {:?}", result);
                result
            }
        }
    }
}

