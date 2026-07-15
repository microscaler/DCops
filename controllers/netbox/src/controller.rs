//! Main controller implementation.
//!
//! This module contains the `Controller` struct that orchestrates
//! reconciliation and resource watching for the unified NetBox Controller.
//!
//! The controller manages NetBox CRD types for IPAM, Tenancy, DCIM, and Extras.

use crate::reconciler::Reconciler;
use crate::watcher::Watcher;
use crate::error::ControllerError;
use crate::kube_api_trait::KubeApiWrapper;
use crate::token_resolver::TokenResolver;
use crds::{
    IPPool, IPClaim,
    NetBoxAggregate, NetBoxDevice, NetBoxDeviceRole, NetBoxDeviceType,
    NetBoxIPAddress, NetBoxIPRange, NetBoxInterface, NetBoxLocation, NetBoxMACAddress,
    NetBoxManufacturer, NetBoxPlatform, NetBoxPrefix, NetBoxRIR, NetBoxRegion, NetBoxRole,
    NetBoxRouteTarget, NetBoxSite, NetBoxSiteGroup, NetBoxTag, NetBoxTenant, NetBoxTenantGroup,
    NetBoxVLAN, NetBoxVRF,
};
use kube::{Api, Client};
use tokio::task::JoinHandle;
use tracing::{info, warn};
use std::sync::Arc;

/// Main controller for NetBox resource management.
pub struct Controller {
    // IPAM watchers
    netbox_prefix_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_role_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_tag_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_aggregate_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_vlan_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_rir_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_ip_address_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_ip_range_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_vrf_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_route_target_watcher: JoinHandle<Result<(), ControllerError>>,
    // Tenancy watchers
    netbox_tenant_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_tenant_group_watcher: JoinHandle<Result<(), ControllerError>>,
    // DCIM watchers
    netbox_site_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_device_role_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_manufacturer_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_platform_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_device_type_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_device_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_interface_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_mac_address_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_region_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_site_group_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_location_watcher: JoinHandle<Result<(), ControllerError>>,
}

impl Controller {

    /// Performs startup tasks like reconciliation and cleanup.
    async fn perform_startup_tasks(
        reconciler: &Reconciler,
        token_resolver: &std::sync::Arc<TokenResolver>,
    ) {
        // Perform startup reconciliation to map existing NetBox resources back to CRs
        info!("Performing startup reconciliation to map existing NetBox resources...");
        if let Err(e) = reconciler.startup_reconciliation().await {
            warn!("Startup reconciliation failed (will continue): {}", e);
        } else {
            info!("Startup reconciliation completed");
        }

        // Perform global duplicate IP address cleanup on startup
        // Try to get a NetBox client for the default tenant to run cleanup
        info!("Performing global duplicate IP address cleanup...");
        use crds::NetBoxResourceReference;
        let default_tenant_ref = NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTenant".to_string(),
            name: "datacenter-tenant".to_string(),
            namespace: None,
        };
        if let Ok(netbox_client) = token_resolver.create_client_for_tenant("default", &default_tenant_ref).await {
            match reconciler.cleanup_all_duplicate_ips(&netbox_client).await {
                Ok((total, deleted, errors)) => {
                    info!("Global duplicate IP cleanup completed: {} duplicates found, {} deleted, {} errors", total, deleted, errors);
                }
                Err(e) => {
                    warn!("Global duplicate IP cleanup failed (will continue): {}", e);
                }
            }
        } else {
            warn!("Could not get NetBox client for global duplicate cleanup (will continue)");
        }
    }

    /// Creates a new controller instance.
    pub async fn new(
        netbox_url: String,
        namespace: Option<String>,
    ) -> Result<Self, ControllerError> {
        info!("Initializing NetBox Controller (Multi-Tenant Mode)");

        // Create Kubernetes client
        let kube_client = Client::try_default().await
            .map_err(|e| ControllerError::Kube(e.into()))?;

        // Create TokenResolver (single point of dependency injection)
        let token_resolver = Arc::new(TokenResolver::new(kube_client.clone(), netbox_url.clone()));
        info!("✅ TokenResolver initialized - tokens will be resolved from Tenant CRDs");

        // Create API clients for all CRD types
        // NOTE: These are REAL kube::Api<T> instances that connect to the actual Kubernetes cluster
        // The KubeApiWrapper is a thin delegation layer that forwards all calls to these real APIs
        let ns = namespace.as_deref().unwrap_or("default");
        // IPAM APIs
        let netbox_prefix_api: Api<NetBoxPrefix> = Api::namespaced(kube_client.clone(), ns);
        let netbox_role_api: Api<NetBoxRole> = Api::namespaced(kube_client.clone(), ns);
        let netbox_tag_api: Api<NetBoxTag> = Api::namespaced(kube_client.clone(), ns);
        let netbox_aggregate_api: Api<NetBoxAggregate> = Api::namespaced(kube_client.clone(), ns);
        let netbox_vlan_api: Api<NetBoxVLAN> = Api::namespaced(kube_client.clone(), ns);
        let netbox_rir_api: Api<NetBoxRIR> = Api::namespaced(kube_client.clone(), ns);
        let netbox_ip_address_api: Api<NetBoxIPAddress> = Api::namespaced(kube_client.clone(), ns);
        let netbox_ip_range_api: Api<NetBoxIPRange> = Api::namespaced(kube_client.clone(), ns);
        let netbox_vrf_api: Api<NetBoxVRF> = Api::namespaced(kube_client.clone(), ns);
        let netbox_route_target_api: Api<NetBoxRouteTarget> = Api::namespaced(kube_client.clone(), ns);
        let netbox_ip_pool_api: Api<IPPool> = Api::namespaced(kube_client.clone(), ns);
        let netbox_ip_claim_api: Api<IPClaim> = Api::namespaced(kube_client.clone(), ns);
        // Tenancy APIs
        let netbox_tenant_api: Api<NetBoxTenant> = Api::namespaced(kube_client.clone(), ns);
        let netbox_tenant_group_api: Api<NetBoxTenantGroup> = Api::namespaced(kube_client.clone(), ns);
        // DCIM APIs
        let netbox_site_api: Api<NetBoxSite> = Api::namespaced(kube_client.clone(), ns);
        let netbox_device_role_api: Api<NetBoxDeviceRole> = Api::namespaced(kube_client.clone(), ns);
        let netbox_manufacturer_api: Api<NetBoxManufacturer> = Api::namespaced(kube_client.clone(), ns);
        let netbox_platform_api: Api<NetBoxPlatform> = Api::namespaced(kube_client.clone(), ns);
        let netbox_device_type_api: Api<NetBoxDeviceType> = Api::namespaced(kube_client.clone(), ns);
        let netbox_device_api: Api<NetBoxDevice> = Api::namespaced(kube_client.clone(), ns);
        let netbox_interface_api: Api<NetBoxInterface> = Api::namespaced(kube_client.clone(), ns);
        let netbox_mac_address_api: Api<NetBoxMACAddress> = Api::namespaced(kube_client.clone(), ns);
        let netbox_region_api: Api<NetBoxRegion> = Api::namespaced(kube_client.clone(), ns);
        let netbox_site_group_api: Api<NetBoxSiteGroup> = Api::namespaced(kube_client.clone(), ns);
        let netbox_location_api: Api<NetBoxLocation> = Api::namespaced(kube_client.clone(), ns);

        // Create reconciler with wrapped APIs
        // NOTE: KubeApiWrapper is a thin delegation layer - all calls forward to real Api<T>
        // This preserves 100% real cluster operation while enabling unit testing with mocks
        // Create RealSecretFetcher for production use
        use crate::secret_fetcher::RealSecretFetcher;
        let secret_fetcher = Arc::new(RealSecretFetcher::new(kube_client.clone()));

        // Create EventRecorder for emitting Kubernetes events
        use kube::runtime::events::{Reporter, Recorder};
        use crate::events::RecorderWrapper;
        let reporter = Reporter {
            controller: "netbox-controller".to_string(),
            instance: Some("netbox-controller".to_string()),
        };
        let recorder = Recorder::new(kube_client.clone(), reporter);
        let event_recorder: Option<Arc<dyn crate::events::EventRecorderTrait>> = Some(Arc::new(RecorderWrapper::new(recorder)));

        let reconciler = Reconciler::new(
            token_resolver.clone(),
            Some(secret_fetcher), // Use RealSecretFetcher for production
            event_recorder, // Use EventRecorder for production
            // IPAM
            KubeApiWrapper::new(netbox_prefix_api.clone()), // Wraps REAL Api<T> - zero overhead
            KubeApiWrapper::new(netbox_role_api.clone()),
            KubeApiWrapper::new(netbox_tag_api.clone()),
            KubeApiWrapper::new(netbox_aggregate_api.clone()),
            KubeApiWrapper::new(netbox_vlan_api.clone()),
            KubeApiWrapper::new(netbox_rir_api.clone()),
            KubeApiWrapper::new(netbox_ip_address_api.clone()),
            KubeApiWrapper::new(netbox_ip_range_api.clone()),
            KubeApiWrapper::new(netbox_vrf_api.clone()),
            KubeApiWrapper::new(netbox_route_target_api.clone()),
            KubeApiWrapper::new(netbox_ip_pool_api.clone()),
            KubeApiWrapper::new(netbox_ip_claim_api.clone()),
            // Tenancy
            KubeApiWrapper::new(netbox_tenant_api.clone()),
            KubeApiWrapper::new(netbox_tenant_group_api.clone()),
            // DCIM
            KubeApiWrapper::new(netbox_site_api.clone()),
            KubeApiWrapper::new(netbox_device_role_api.clone()),
            KubeApiWrapper::new(netbox_manufacturer_api.clone()),
            KubeApiWrapper::new(netbox_platform_api.clone()),
            KubeApiWrapper::new(netbox_device_type_api.clone()),
            KubeApiWrapper::new(netbox_device_api.clone()),
            KubeApiWrapper::new(netbox_interface_api.clone()),
            KubeApiWrapper::new(netbox_mac_address_api.clone()),
            KubeApiWrapper::new(netbox_region_api.clone()),
            KubeApiWrapper::new(netbox_site_group_api.clone()),
            KubeApiWrapper::new(netbox_location_api.clone()),
        );

        // Perform startup tasks (reconciliation, cleanup, etc.)
        Self::perform_startup_tasks(&reconciler, &token_resolver).await;

        // Create watchers - use Arc to share reconciler
        let reconciler_arc = Arc::new(reconciler);

        // Create a single watcher instance that handles all CRD types
        let watcher_instance = Arc::new(Watcher::new(
            reconciler_arc.clone(),
            // IPAM
            netbox_prefix_api.clone(),
            netbox_role_api.clone(),
            netbox_tag_api.clone(),
            netbox_aggregate_api.clone(),
            netbox_vlan_api.clone(),
            netbox_rir_api.clone(),
            netbox_ip_address_api.clone(),
            netbox_ip_range_api.clone(),
            netbox_vrf_api.clone(),
            netbox_route_target_api.clone(),
            netbox_ip_pool_api.clone(),
            netbox_ip_claim_api.clone(),
            // Tenancy
            netbox_tenant_api.clone(),
            netbox_tenant_group_api.clone(),
            // DCIM
            netbox_site_api.clone(),
            netbox_device_role_api.clone(),
            netbox_manufacturer_api.clone(),
            netbox_platform_api.clone(),
            netbox_device_type_api.clone(),
            netbox_device_api.clone(),
            netbox_interface_api.clone(),
            netbox_mac_address_api.clone(),
            netbox_region_api.clone(),
            netbox_site_group_api.clone(),
            netbox_location_api.clone(),
        ));

        // Start all watchers in background tasks
        let netbox_prefix_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_prefixes().await
            })
        };

        let netbox_tenant_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_tenants().await
            })
        };

        let netbox_tenant_group_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_tenant_groups().await
            })
        };

        let netbox_site_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_sites().await
            })
        };

        let netbox_role_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_roles().await
            })
        };

        let netbox_tag_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_tags().await
            })
        };

        let netbox_aggregate_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_aggregates().await
            })
        };

        let netbox_vlan_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_vlans().await
            })
        };

        let netbox_rir_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_rirs().await
            })
        };

        let netbox_ip_address_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_ip_addresses().await
            })
        };

        let netbox_ip_range_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_ip_ranges().await
            })
        };

        let netbox_vrf_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_vrfs().await
            })
        };

        let netbox_route_target_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_route_targets().await
            })
        };

        let netbox_device_role_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_device_roles().await
            })
        };

        let netbox_manufacturer_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_manufacturers().await
            })
        };

        let netbox_platform_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_platforms().await
            })
        };

        let netbox_device_type_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_device_types().await
            })
        };

        let netbox_device_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_devices().await
            })
        };

        let netbox_interface_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_interfaces().await
            })
        };

        let netbox_mac_address_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_mac_addresses().await
            })
        };

        let netbox_region_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_regions().await
            })
        };

        let netbox_site_group_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_site_groups().await
            })
        };

        let netbox_location_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_netbox_locations().await
            })
        };

        Ok(Self {
            // IPAM watchers
            netbox_prefix_watcher,
            netbox_role_watcher,
            netbox_tag_watcher,
            netbox_aggregate_watcher,
            netbox_vlan_watcher,
            netbox_rir_watcher,
            netbox_ip_address_watcher,
            netbox_ip_range_watcher,
            netbox_vrf_watcher,
            netbox_route_target_watcher,
            // Tenancy watchers
            netbox_tenant_watcher,
            netbox_tenant_group_watcher,
            // DCIM watchers
            netbox_site_watcher,
            netbox_device_role_watcher,
            netbox_manufacturer_watcher,
            netbox_platform_watcher,
            netbox_device_type_watcher,
            netbox_device_watcher,
            netbox_interface_watcher,
            netbox_mac_address_watcher,
            netbox_region_watcher,
            netbox_site_group_watcher,
            netbox_location_watcher,
        })
    }

    /// Runs the controller until shutdown.
    pub async fn run(mut self) -> Result<(), ControllerError> {
        info!("NetBox Controller running");

        // Wait for any watcher to exit (they should run forever)
        tokio::select! {
            result = &mut self.netbox_prefix_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxPrefix watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxPrefix watcher error: {}", e)))?;
            }
            result = &mut self.netbox_tenant_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxTenant watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxTenant watcher error: {}", e)))?;
            }
            result = &mut self.netbox_tenant_group_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxTenantGroup watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxTenantGroup watcher error: {}", e)))?;
            }
            result = &mut self.netbox_site_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxSite watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxSite watcher error: {}", e)))?;
            }
            result = &mut self.netbox_role_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxRole watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxRole watcher error: {}", e)))?;
            }
            result = &mut self.netbox_tag_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxTag watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxTag watcher error: {}", e)))?;
            }
            result = &mut self.netbox_aggregate_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxAggregate watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxAggregate watcher error: {}", e)))?;
            }
            result = &mut self.netbox_vlan_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxVLAN watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxVLAN watcher error: {}", e)))?;
            }
            result = &mut self.netbox_rir_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxRIR watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxRIR watcher error: {}", e)))?;
            }
            result = &mut self.netbox_ip_address_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxIPAddress watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxIPAddress watcher error: {}", e)))?;
            }
            result = &mut self.netbox_ip_range_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxIPRange watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxIPRange watcher error: {}", e)))?;
            }
            result = &mut self.netbox_vrf_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxVRF watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxVRF watcher error: {}", e)))?;
            }
            result = &mut self.netbox_route_target_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxRouteTarget watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxRouteTarget watcher error: {}", e)))?;
            }
            result = &mut self.netbox_device_role_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxDeviceRole watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxDeviceRole watcher error: {}", e)))?;
            }
            result = &mut self.netbox_manufacturer_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxManufacturer watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxManufacturer watcher error: {}", e)))?;
            }
            result = &mut self.netbox_platform_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxPlatform watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxPlatform watcher error: {}", e)))?;
            }
            result = &mut self.netbox_device_type_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxDeviceType watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxDeviceType watcher error: {}", e)))?;
            }
            result = &mut self.netbox_device_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxDevice watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxDevice watcher error: {}", e)))?;
            }
            result = &mut self.netbox_interface_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxInterface watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxInterface watcher error: {}", e)))?;
            }
            result = &mut self.netbox_mac_address_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxMACAddress watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxMACAddress watcher error: {}", e)))?;
            }
            result = &mut self.netbox_region_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxRegion watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxRegion watcher error: {}", e)))?;
            }
            result = &mut self.netbox_site_group_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxSiteGroup watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxSiteGroup watcher error: {}", e)))?;
            }
            result = &mut self.netbox_location_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("NetBoxLocation watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("NetBoxLocation watcher error: {}", e)))?;
            }
        }

        Ok(())
    }
}
