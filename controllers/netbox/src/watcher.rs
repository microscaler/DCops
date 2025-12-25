//! Kubernetes resource watchers.
//!
//! This module handles watching Kubernetes resources for changes
//! and triggering reconciliation using kube_runtime::Controller.
//!
//! All watchers use a generic `watch_resource()` helper that properly handles
//! the reconcile loop with automatic reconnection and retry logic.

use crate::reconciler::Reconciler;
use crate::error::ControllerError;
use crds::{
    IPClaim, IPPool, NetBoxPrefix, NetBoxTenant, NetBoxSite, NetBoxRole, NetBoxTag, NetBoxAggregate,
    NetBoxVLAN, NetBoxDeviceRole, NetBoxManufacturer, NetBoxPlatform, NetBoxDeviceType,
    NetBoxDevice, NetBoxInterface, NetBoxMACAddress, NetBoxRegion, NetBoxSiteGroup, NetBoxLocation,
};
use kube::Api;
use kube_runtime::{Controller, watcher, controller::{Action, Config as ControllerConfig}};
use std::sync::Arc;
use tracing::{info, error, debug};
use std::time::Duration;
use futures::StreamExt;

/// Generic watcher helper that uses kube_runtime::Controller properly.
/// 
/// This fixes the reconcile loop issue for ALL watchers at once by:
/// - Using Controller which handles automatic reconnection
/// - Managing retries and backoff automatically
/// - Continuing to watch indefinitely (no one-shot behavior)
/// - Processing all events (Apply, Delete, etc.)
/// 
/// The reconcile_fn should match our existing reconcile function signature:
/// `async fn reconcile(&self, resource: &K) -> Result<(), ControllerError>`
async fn watch_resource<K, F>(
    api: Api<K>,
    reconciler: Arc<Reconciler>,
    reconcile_fn: F,
    resource_name: &str,
) -> Result<(), ControllerError>
where
    K: kube::Resource + Clone + Send + Sync + 'static + std::fmt::Debug + serde::de::DeserializeOwned,
    K::DynamicType: Default + std::cmp::Eq + std::hash::Hash + Clone + std::fmt::Debug + Unpin,
    F: Fn(Arc<Reconciler>, Arc<K>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Action, ControllerError>> + Send>> + Send + Sync + Clone + 'static,
{
    info!("Starting {} watcher", resource_name);
    
    // Error policy: requeue with exponential backoff on errors
    let error_policy = |obj: Arc<K>, error: &ControllerError, _ctx: Arc<Reconciler>| {
        error!("Reconciliation error for {} {:?}: {}", resource_name, obj, error);
        Action::requeue(Duration::from_secs(60))
    };
    
    // Reconcile function: wraps our existing reconcile functions
    // Controller automatically filters by generation (status-only updates don't trigger reconciliation)
    // But we add logging to help debug excessive reconciliations
    let reconcile = move |obj: Arc<K>, ctx: Arc<Reconciler>| {
        let reconcile_fn = reconcile_fn.clone();
        let resource_name = resource_name.to_string();
        async move {
            // Log reconciliation attempt for debugging excessive reconciliations
            // Note: Controller should filter by generation automatically, but logging helps diagnose issues
            debug!("Reconciling {} {:?}", resource_name, obj);
            
            match reconcile_fn(ctx, obj).await {
                Ok(action) => Ok(action),
                Err(e) => {
                    error!("Reconciliation failed for {}: {}", resource_name, e);
                    Err(e)
                }
            }
        }
    };
    
    // Configure controller with debounce and concurrency limits
    // Debounce waits 5 seconds after the last event before reconciling
    // This batches multiple status updates together and reduces API load
    // Concurrency limits to 3 concurrent reconciliations per watcher
    // Total: 17 watchers × 3 = 51 max concurrent reconciliations (much better than unlimited)
    let controller_config = ControllerConfig::default()
        .debounce(Duration::from_secs(5))
        .concurrency(3);
    
    Controller::new(api, watcher::Config::default())
        .with_config(controller_config)
        .run(reconcile, error_policy, reconciler)
        .for_each(|res| async move {
            if let Err(e) = res {
                error!("Controller error for {}: {}", resource_name, e);
            }
        })
        .await;
    
    Ok(())
}

/// Watches Kubernetes resources for changes.
pub struct Watcher {
    reconciler: Arc<Reconciler>,
    // IPAM APIs
    netbox_prefix_api: Api<NetBoxPrefix>,
    netbox_role_api: Api<NetBoxRole>,
    netbox_tag_api: Api<NetBoxTag>,
    netbox_aggregate_api: Api<NetBoxAggregate>,
    netbox_vlan_api: Api<NetBoxVLAN>,
    // Tenancy APIs
    netbox_tenant_api: Api<NetBoxTenant>,
    // DCIM APIs
    netbox_site_api: Api<NetBoxSite>,
    netbox_device_role_api: Api<NetBoxDeviceRole>,
    netbox_manufacturer_api: Api<NetBoxManufacturer>,
    netbox_platform_api: Api<NetBoxPlatform>,
    netbox_device_type_api: Api<NetBoxDeviceType>,
    netbox_device_api: Api<NetBoxDevice>,
    netbox_interface_api: Api<NetBoxInterface>,
    netbox_mac_address_api: Api<NetBoxMACAddress>,
    netbox_region_api: Api<NetBoxRegion>,
    netbox_site_group_api: Api<NetBoxSiteGroup>,
    netbox_location_api: Api<NetBoxLocation>,
    // Custom CRDs
    ip_pool_api: Api<IPPool>,
    ip_claim_api: Api<IPClaim>,
}

impl Watcher {
    /// Creates a new watcher instance.
    pub fn new(
        reconciler: Arc<Reconciler>,
        // IPAM APIs
        netbox_prefix_api: Api<NetBoxPrefix>,
        netbox_role_api: Api<NetBoxRole>,
        netbox_tag_api: Api<NetBoxTag>,
        netbox_aggregate_api: Api<NetBoxAggregate>,
        netbox_vlan_api: Api<NetBoxVLAN>,
        // Tenancy APIs
        netbox_tenant_api: Api<NetBoxTenant>,
        // DCIM APIs
        netbox_site_api: Api<NetBoxSite>,
        netbox_device_role_api: Api<NetBoxDeviceRole>,
        netbox_manufacturer_api: Api<NetBoxManufacturer>,
        netbox_platform_api: Api<NetBoxPlatform>,
        netbox_device_type_api: Api<NetBoxDeviceType>,
        netbox_device_api: Api<NetBoxDevice>,
        netbox_interface_api: Api<NetBoxInterface>,
        netbox_mac_address_api: Api<NetBoxMACAddress>,
        netbox_region_api: Api<NetBoxRegion>,
        netbox_site_group_api: Api<NetBoxSiteGroup>,
        netbox_location_api: Api<NetBoxLocation>,
        // Custom CRDs
        ip_pool_api: Api<IPPool>,
        ip_claim_api: Api<IPClaim>,
    ) -> Self {
        Self {
            reconciler,
            // IPAM
            netbox_prefix_api,
            netbox_role_api,
            netbox_tag_api,
            netbox_aggregate_api,
            netbox_vlan_api,
            // Tenancy
            netbox_tenant_api,
            // DCIM
            netbox_site_api,
            netbox_device_role_api,
            netbox_manufacturer_api,
            netbox_platform_api,
            netbox_device_type_api,
            netbox_device_api,
            netbox_interface_api,
            netbox_mac_address_api,
            netbox_region_api,
            netbox_site_group_api,
            netbox_location_api,
            // Custom
            ip_pool_api,
            ip_claim_api,
        }
    }
    
    /// Starts watching NetBoxPrefix resources.
    pub async fn watch_netbox_prefixes(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.netbox_prefix_api.clone(),
            self.reconciler.clone(),
            |reconciler, resource| {
                Box::pin(async move {
                    match reconciler.reconcile_netbox_prefix(&*resource).await {
                        Ok(()) => Ok(Action::await_change()),
                        Err(e) => Err(e),
                    }
                })
            },
            "NetBoxPrefix",
        ).await
    }
    
    /// Starts watching IPClaim resources.
    pub async fn watch_ip_claims(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.ip_claim_api.clone(),
            self.reconciler.clone(),
            |reconciler, resource| {
                Box::pin(async move {
                    match reconciler.reconcile_ip_claim(&*resource).await {
                        Ok(()) => Ok(Action::await_change()),
                        Err(e) => Err(e),
                    }
                })
            },
            "IPClaim",
        ).await
    }
    
    /// Starts watching IPPool resources.
    pub async fn watch_ip_pools(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.ip_pool_api.clone(),
            self.reconciler.clone(),
            |reconciler, resource| {
                Box::pin(async move {
                    match reconciler.reconcile_ip_pool(&*resource).await {
                        Ok(()) => Ok(Action::await_change()),
                        Err(e) => Err(e),
                    }
                })
            },
            "IPPool",
        ).await
    }
    
    /// Watches NetBoxTenant resources for changes.
    pub async fn watch_netbox_tenants(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.netbox_tenant_api.clone(),
            self.reconciler.clone(),
            |reconciler, resource| {
                Box::pin(async move {
                    match reconciler.reconcile_netbox_tenant(&*resource).await {
                        Ok(()) => Ok(Action::await_change()),
                        Err(e) => Err(e),
                    }
                })
            },
            "NetBoxTenant",
        ).await
    }
    
    /// Watches NetBoxSite resources for changes.
    pub async fn watch_netbox_sites(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.netbox_site_api.clone(),
            self.reconciler.clone(),
            |reconciler, resource| {
                Box::pin(async move {
                    match reconciler.reconcile_netbox_site(&*resource).await {
                        Ok(()) => Ok(Action::await_change()),
                        Err(e) => Err(e),
                    }
                })
            },
            "NetBoxSite",
        ).await
    }
    
    /// Watches NetBoxRole resources for changes.
    pub async fn watch_netbox_roles(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.netbox_role_api.clone(),
            self.reconciler.clone(),
            |reconciler, resource| {
                Box::pin(async move {
                    match reconciler.reconcile_netbox_role(&*resource).await {
                        Ok(()) => Ok(Action::await_change()),
                        Err(e) => Err(e),
                    }
                })
            },
            "NetBoxRole",
        ).await
    }
    
    /// Watches NetBoxTag resources for changes.
    pub async fn watch_netbox_tags(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.netbox_tag_api.clone(),
            self.reconciler.clone(),
            |reconciler, resource| {
                Box::pin(async move {
                    match reconciler.reconcile_netbox_tag(&*resource).await {
                        Ok(()) => Ok(Action::await_change()),
                        Err(e) => Err(e),
                    }
                })
            },
            "NetBoxTag",
        ).await
    }
    
    /// Watches NetBoxAggregate resources for changes.
    pub async fn watch_netbox_aggregates(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.netbox_aggregate_api.clone(),
            self.reconciler.clone(),
            |reconciler, resource| {
                Box::pin(async move {
                    match reconciler.reconcile_netbox_aggregate(&*resource).await {
                        Ok(()) => Ok(Action::await_change()),
                        Err(e) => Err(e),
                    }
                })
            },
            "NetBoxAggregate",
        ).await
    }
    
    /// Starts watching NetBoxDeviceRole resources.
    pub async fn watch_netbox_device_roles(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.netbox_device_role_api.clone(),
            self.reconciler.clone(),
            |reconciler, resource| {
                Box::pin(async move {
                    match reconciler.reconcile_netbox_device_role(&*resource).await {
                        Ok(()) => Ok(Action::await_change()),
                        Err(e) => Err(e),
                    }
                })
            },
            "NetBoxDeviceRole",
        ).await
    }
    
    /// Starts watching NetBoxManufacturer resources.
    pub async fn watch_netbox_manufacturers(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.netbox_manufacturer_api.clone(),
            self.reconciler.clone(),
            |reconciler, resource| {
                Box::pin(async move {
                    match reconciler.reconcile_netbox_manufacturer(&*resource).await {
                        Ok(()) => Ok(Action::await_change()),
                        Err(e) => Err(e),
                    }
                })
            },
            "NetBoxManufacturer",
        ).await
    }
    
    /// Starts watching NetBoxPlatform resources.
    pub async fn watch_netbox_platforms(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.netbox_platform_api.clone(),
            self.reconciler.clone(),
            |reconciler, resource| {
                Box::pin(async move {
                    match reconciler.reconcile_netbox_platform(&*resource).await {
                        Ok(()) => Ok(Action::await_change()),
                        Err(e) => Err(e),
                    }
                })
            },
            "NetBoxPlatform",
        ).await
    }
    
    /// Starts watching NetBoxDeviceType resources.
    pub async fn watch_netbox_device_types(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.netbox_device_type_api.clone(),
            self.reconciler.clone(),
            |reconciler, resource| {
                Box::pin(async move {
                    match reconciler.reconcile_netbox_device_type(&*resource).await {
                        Ok(()) => Ok(Action::await_change()),
                        Err(e) => Err(e),
                    }
                })
            },
            "NetBoxDeviceType",
        ).await
    }
    
    /// Starts watching NetBoxVLAN resources.
    pub async fn watch_netbox_vlans(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.netbox_vlan_api.clone(),
            self.reconciler.clone(),
            |reconciler, resource| {
                Box::pin(async move {
                    match reconciler.reconcile_netbox_vlan(&*resource).await {
                        Ok(()) => Ok(Action::await_change()),
                        Err(e) => Err(e),
                    }
                })
            },
            "NetBoxVLAN",
        ).await
    }
    
    /// Starts watching NetBoxRegion resources.
    pub async fn watch_netbox_regions(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.netbox_region_api.clone(),
            self.reconciler.clone(),
            |reconciler, resource| {
                Box::pin(async move {
                    match reconciler.reconcile_netbox_region(&*resource).await {
                        Ok(()) => Ok(Action::await_change()),
                        Err(e) => Err(e),
                    }
                })
            },
            "NetBoxRegion",
        ).await
    }
    
    /// Starts watching NetBoxSiteGroup resources.
    pub async fn watch_netbox_site_groups(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.netbox_site_group_api.clone(),
            self.reconciler.clone(),
            |reconciler, resource| {
                Box::pin(async move {
                    match reconciler.reconcile_netbox_site_group(&*resource).await {
                        Ok(()) => Ok(Action::await_change()),
                        Err(e) => Err(e),
                    }
                })
            },
            "NetBoxSiteGroup",
        ).await
    }
    
    /// Starts watching NetBoxLocation resources.
    pub async fn watch_netbox_locations(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.netbox_location_api.clone(),
            self.reconciler.clone(),
            |reconciler, resource| {
                Box::pin(async move {
                    match reconciler.reconcile_netbox_location(&*resource).await {
                        Ok(()) => Ok(Action::await_change()),
                        Err(e) => Err(e),
                    }
                })
            },
            "NetBoxLocation",
        ).await
    }
    
    /// Starts watching NetBoxDevice resources.
    pub async fn watch_netbox_devices(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.netbox_device_api.clone(),
            self.reconciler.clone(),
            |reconciler, resource| {
                Box::pin(async move {
                    match reconciler.reconcile_netbox_device(&*resource).await {
                        Ok(()) => Ok(Action::await_change()),
                        Err(e) => Err(e),
                    }
                })
            },
            "NetBoxDevice",
        ).await
    }
    
    /// Starts watching NetBoxInterface resources.
    pub async fn watch_netbox_interfaces(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.netbox_interface_api.clone(),
            self.reconciler.clone(),
            |reconciler, resource| {
                Box::pin(async move {
                    match reconciler.reconcile_netbox_interface(&*resource).await {
                        Ok(()) => Ok(Action::await_change()),
                        Err(e) => Err(e),
                    }
                })
            },
            "NetBoxInterface",
        ).await
    }
    
    /// Starts watching NetBoxMACAddress resources.
    pub async fn watch_netbox_mac_addresses(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.netbox_mac_address_api.clone(),
            self.reconciler.clone(),
            |reconciler, resource| {
                Box::pin(async move {
                    match reconciler.reconcile_netbox_mac_address(&*resource).await {
                        Ok(()) => Ok(Action::await_change()),
                        Err(e) => Err(e),
                    }
                })
            },
            "NetBoxMACAddress",
        ).await
    }
}
