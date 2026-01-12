//! DHCP Reconciler module
//!
//! Translates NetBox CRDs to Kea configuration and applies it via Control Agent API.

mod config_builder;
mod prefix_resolver;
mod ip_utils;
mod config_comparator;
mod resource_reconciler;

use crate::error::ControllerError;
use crate::kea::KeaClient;
use crds::{NetBoxPrefix, NetBoxIPRange, NetBoxIPAddress};
use kube::{Api, Client};
use std::sync::Arc;
use tracing::info;

// Re-export sub-modules for external use
pub use config_builder::ConfigBuilder;
pub use prefix_resolver::PrefixResolver;
pub use config_comparator::ConfigComparator;
pub use resource_reconciler::ResourceReconciler;

/// DHCP Reconciler that translates CRDs to Kea configuration
pub struct DhcpReconciler {
    #[allow(dead_code)] // Reserved for future use (e.g., incremental updates)
    kube_client: Client,
    kea_client: Arc<KeaClient>,
    #[allow(dead_code)] // Reserved for future use (e.g., incremental updates)
    prefix_api: Api<NetBoxPrefix>,
    #[allow(dead_code)] // Reserved for future use (e.g., incremental updates)
    ip_range_api: Api<NetBoxIPRange>,
    #[allow(dead_code)] // Reserved for future use (e.g., incremental updates)
    ip_address_api: Api<NetBoxIPAddress>,
    config_builder: ConfigBuilder,
    #[allow(dead_code)] // Used by ConfigBuilder, not directly by DhcpReconciler
    prefix_resolver: PrefixResolver,
    config_comparator: ConfigComparator,
    resource_reconciler: ResourceReconciler,
}

impl DhcpReconciler {
    /// Create a new DHCP Reconciler
    pub fn new(kube_client: Client, kea_client: Arc<KeaClient>) -> Self {
        let namespace = std::env::var("WATCH_NAMESPACE")
            .unwrap_or_else(|_| "default".to_string());
        
        let prefix_api = Api::namespaced(kube_client.clone(), &namespace);
        let ip_range_api = Api::namespaced(kube_client.clone(), &namespace);
        let ip_address_api = Api::namespaced(kube_client.clone(), &namespace);
        
        let config_builder = config_builder::ConfigBuilder::new(
            prefix_api.clone(),
            ip_range_api.clone(),
            ip_address_api.clone(),
        );
        
        let prefix_resolver = prefix_resolver::PrefixResolver::new(prefix_api.clone());
        let config_comparator = config_comparator::ConfigComparator::new();
        let resource_reconciler = resource_reconciler::ResourceReconciler::new(kea_client.clone());
        
        Self {
            prefix_api,
            ip_range_api,
            ip_address_api,
            kube_client,
            kea_client,
            config_builder,
            prefix_resolver,
            config_comparator,
            resource_reconciler,
        }
    }
    
    /// Perform full sync of all CRDs to Kea
    ///
    /// This is called at startup to ensure Kea configuration matches all CRDs.
    pub async fn full_sync(&self) -> Result<(), ControllerError> {
        info!("Starting full sync of NetBox CRDs to Kea");
        
        // Get current Kea configuration
        let current_config = self.kea_client.get_config().await?;
        
        // Build desired configuration from all CRDs
        let desired_config = self.config_builder.build_kea_config_from_crds().await?;
        
        // Compare and update if different
        if self.config_comparator.configs_differ(&current_config, &desired_config)? {
            info!("Kea configuration differs from CRDs, updating...");
            self.kea_client.test_config(&desired_config).await?;
            self.kea_client.set_config(&desired_config).await?;
            info!("✅ Kea configuration updated");
        } else {
            info!("✅ Kea configuration is already in sync with CRDs");
        }
        
        Ok(())
    }
    
    /// Reconcile a single NetBoxPrefix CRD
    pub async fn reconcile_prefix(&self, prefix: &NetBoxPrefix) -> Result<(), ControllerError> {
        self.resource_reconciler.reconcile_prefix(prefix, self).await
    }
    
    /// Reconcile a single NetBoxIPRange CRD
    pub async fn reconcile_ip_range(&self, ip_range: &NetBoxIPRange) -> Result<(), ControllerError> {
        self.resource_reconciler.reconcile_ip_range(ip_range, self).await
    }
    
    /// Reconcile a single NetBoxIPAddress CRD
    pub async fn reconcile_ip_address(&self, ip_address: &NetBoxIPAddress) -> Result<(), ControllerError> {
        self.resource_reconciler.reconcile_ip_address(ip_address, self).await
    }
}

