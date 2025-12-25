//! Main controller implementation.
//!
//! This module contains the `Controller` struct that orchestrates
//! reconciliation and resource watching for the unified NetBox Controller.
//!
//! The controller manages three CRD types:
//! - NetBoxPrefix: Creates and manages prefixes in NetBox
//! - IPPool: Manages IP address pools (references NetBoxPrefix)
//! - IPClaim: Allocates IP addresses from IPPools via NetBox

use crate::reconciler::Reconciler;
use crate::watcher::Watcher;
use crate::error::ControllerError;
use crds::{
    IPClaim, IPPool, NetBoxPrefix, NetBoxTenant, NetBoxSite, NetBoxRole, NetBoxTag, NetBoxAggregate,
    NetBoxVLAN, NetBoxDeviceRole, NetBoxManufacturer, NetBoxPlatform, NetBoxDeviceType,
    NetBoxDevice, NetBoxInterface, NetBoxMACAddress, NetBoxRegion, NetBoxSiteGroup, NetBoxLocation,
};
use kube::{Api, Client};
use netbox_client::NetBoxClient;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{info, warn, error};

/// Main controller for NetBox resource management.
pub struct Controller {
    // IPAM watchers
    netbox_prefix_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_role_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_tag_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_aggregate_watcher: JoinHandle<Result<(), ControllerError>>,
    netbox_vlan_watcher: JoinHandle<Result<(), ControllerError>>,
    // Tenancy watchers
    netbox_tenant_watcher: JoinHandle<Result<(), ControllerError>>,
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
    // Custom CRD watchers
    ip_pool_watcher: JoinHandle<Result<(), ControllerError>>,
    ip_claim_watcher: JoinHandle<Result<(), ControllerError>>,
}

impl Controller {
    /// Creates a new controller instance.
    pub async fn new(
        netbox_url: String,
        netbox_token: String,
        namespace: Option<String>,
    ) -> Result<Self, ControllerError> {
        info!("Initializing NetBox Controller");
        
        // Create Kubernetes client
        let kube_client = Client::try_default().await
            .map_err(|e| ControllerError::Kube(e.into()))?;
        
        // Create NetBox client
        let netbox_client = NetBoxClient::new(netbox_url.clone(), netbox_token.clone())
            .map_err(|e| ControllerError::NetBox(e))?;
        
        // Validate token and connectivity before proceeding
        info!("Validating NetBox token and connectivity...");
        netbox_client.validate_token().await
            .map_err(|e| {
                error!("Failed to validate NetBox token: {}", e);
                error!("Please ensure:");
                error!("  1. NETBOX_TOKEN environment variable is set correctly");
                error!("  2. The token is valid in NetBox");
                error!("  3. NetBox is reachable at {}", netbox_url);
                ControllerError::NetBox(e)
            })?;
        info!("✅ NetBox token validated and connectivity established");
        
        // Create API clients for all CRD types
        let ns = namespace.as_deref().unwrap_or("default");
        // IPAM APIs
        let netbox_prefix_api: Api<NetBoxPrefix> = Api::namespaced(kube_client.clone(), ns);
        let netbox_role_api: Api<NetBoxRole> = Api::namespaced(kube_client.clone(), ns);
        let netbox_tag_api: Api<NetBoxTag> = Api::namespaced(kube_client.clone(), ns);
        let netbox_aggregate_api: Api<NetBoxAggregate> = Api::namespaced(kube_client.clone(), ns);
        let netbox_vlan_api: Api<NetBoxVLAN> = Api::namespaced(kube_client.clone(), ns);
        // Tenancy APIs
        let netbox_tenant_api: Api<NetBoxTenant> = Api::namespaced(kube_client.clone(), ns);
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
        // Custom CRDs
        let ip_pool_api: Api<IPPool> = Api::namespaced(kube_client.clone(), ns);
        let ip_claim_api: Api<IPClaim> = Api::namespaced(kube_client.clone(), ns);
        
        // Create reconciler
        let reconciler = Reconciler::new(
            netbox_client,
            // IPAM
            netbox_prefix_api.clone(),
            netbox_role_api.clone(),
            netbox_tag_api.clone(),
            netbox_aggregate_api.clone(),
            netbox_vlan_api.clone(),
            // Tenancy
            netbox_tenant_api.clone(),
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
            // Custom
            ip_pool_api.clone(),
            ip_claim_api.clone(),
        );
        
        // Perform startup reconciliation to map existing NetBox resources back to CRs
        info!("Performing startup reconciliation to map existing NetBox resources...");
        if let Err(e) = reconciler.startup_reconciliation().await {
            warn!("Startup reconciliation failed (will continue): {}", e);
        } else {
            info!("Startup reconciliation completed");
        }
        
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
            // Tenancy
            netbox_tenant_api.clone(),
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
            // Custom
            ip_pool_api.clone(),
            ip_claim_api.clone(),
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
        
        let ip_pool_watcher = {
            let watcher = watcher_instance.clone();
            tokio::spawn(async move {
                watcher.watch_ip_pools().await
            })
        };
        
        let ip_claim_watcher = {
            let watcher = watcher_instance;
            tokio::spawn(async move {
                watcher.watch_ip_claims().await
            })
        };
        
        Ok(Self {
            // IPAM watchers
            netbox_prefix_watcher,
            netbox_role_watcher,
            netbox_tag_watcher,
            netbox_aggregate_watcher,
            netbox_vlan_watcher,
            // Tenancy watchers
            netbox_tenant_watcher,
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
            // Custom CRD watchers
            ip_pool_watcher,
            ip_claim_watcher,
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
            result = &mut self.ip_pool_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("IPPool watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("IPPool watcher error: {}", e)))?;
            }
            result = &mut self.ip_claim_watcher => {
                result.map_err(|e| ControllerError::Watch(format!("IPClaim watcher panicked: {}", e)))?
                    .map_err(|e| ControllerError::Watch(format!("IPClaim watcher error: {}", e)))?;
            }
        }
        
        Ok(())
    }
}


