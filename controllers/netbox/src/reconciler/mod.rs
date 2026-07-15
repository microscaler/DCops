//! Reconciliation logic for NetBox-related CRDs.
//!
//! This module is organized by NetBox API sections:
//! - `ipam`: IP Address Management (Prefixes, Aggregates, IP Ranges, IP Addresses)
//! - `tenancy`: Tenancy (Tenants)
//! - `dcim`: Data Center Infrastructure Management (Sites, Devices, Interfaces, etc.)
//! - `extras`: Extras (Roles, Tags)

pub mod ipam;
pub mod tenancy;
#[cfg(test)]
pub mod tenancy_test;
#[cfg(test)]
mod mod_test;
pub mod dcim;
pub mod extras;
#[cfg(test)]
pub mod extras_test;
#[cfg(test)]
mod events_integration_test;

use crate::error::ControllerError;
use crate::backoff::FibonacciBackoff;
use crate::kube_api_trait::KubeApiTrait;
use crate::token_resolver::TokenResolverTrait;
use crate::secret_fetcher::SecretFetcher;
use netbox_client::{NetBoxClientTrait, PrefixId};
use crds::{
    NetBoxPrefix, NetBoxTenant, NetBoxTenantGroup, NetBoxSite, NetBoxRole, NetBoxTag, NetBoxAggregate,
    NetBoxDeviceRole, NetBoxManufacturer, NetBoxPlatform, NetBoxDeviceType, NetBoxDevice,
    NetBoxInterface, NetBoxMACAddress, NetBoxVLAN, NetBoxRegion, NetBoxSiteGroup, NetBoxLocation,
    NetBoxRIR, NetBoxIPAddress, NetBoxIPRange, NetBoxVRF, NetBoxRouteTarget, PrefixState, ResourceState,
    IPClaim, IPClaimState, IPClaimStatus, IPPool, IPPoolState, IPPoolStatus,
};
use tracing::{info, error, debug, warn};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

/// Backoff state for a resource
#[derive(Debug, Clone)]
struct BackoffState {
    backoff: FibonacciBackoff,
    error_count: u32,
}

impl BackoffState {
    fn new() -> Self {
        Self {
            backoff: FibonacciBackoff::new(1, 10), // 1 minute min, 10 minutes max
            error_count: 0,
        }
    }

    fn increment_error(&mut self) {
        self.error_count += 1;
    }

    fn reset(&mut self) {
        self.error_count = 0;
        self.backoff.reset();
    }
}

/// Reconciles NetBox-related resources.
pub struct Reconciler {
    pub(crate) token_resolver: Arc<dyn TokenResolverTrait>,
    pub(crate) secret_fetcher: Option<Arc<dyn SecretFetcher>>, // Optional for testing
    pub(crate) event_recorder: Option<Arc<dyn crate::events::EventRecorderTrait>>, // Optional for testing
    // IPAM APIs
    pub(crate) netbox_prefix_api: Box<dyn KubeApiTrait<NetBoxPrefix> + Send + Sync>,
    pub(crate) netbox_role_api: Box<dyn KubeApiTrait<NetBoxRole> + Send + Sync>,
    pub(crate) netbox_tag_api: Box<dyn KubeApiTrait<NetBoxTag> + Send + Sync>,
    pub(crate) netbox_aggregate_api: Box<dyn KubeApiTrait<NetBoxAggregate> + Send + Sync>,
    pub(crate) netbox_vlan_api: Box<dyn KubeApiTrait<NetBoxVLAN> + Send + Sync>,
    pub(crate) netbox_rir_api: Box<dyn KubeApiTrait<NetBoxRIR> + Send + Sync>,
    pub(crate) netbox_ip_address_api: Box<dyn KubeApiTrait<NetBoxIPAddress> + Send + Sync>,
    pub(crate) netbox_ip_range_api: Box<dyn KubeApiTrait<NetBoxIPRange> + Send + Sync>,
    pub(crate) netbox_vrf_api: Box<dyn KubeApiTrait<NetBoxVRF> + Send + Sync>,
    pub(crate) netbox_route_target_api: Box<dyn KubeApiTrait<NetBoxRouteTarget> + Send + Sync>,
    // IPAM: IPPool / IPClaim APIs
    pub(crate) netbox_ip_pool_api: Box<dyn KubeApiTrait<IPPool> + Send + Sync>,
    pub(crate) netbox_ip_claim_api: Box<dyn KubeApiTrait<IPClaim> + Send + Sync>,
    // Tenancy APIs
    pub(crate) netbox_tenant_api: Box<dyn KubeApiTrait<NetBoxTenant> + Send + Sync>,
    pub(crate) netbox_tenant_group_api: Box<dyn KubeApiTrait<NetBoxTenantGroup> + Send + Sync>,
    // DCIM APIs
    pub(crate) netbox_site_api: Box<dyn KubeApiTrait<NetBoxSite> + Send + Sync>,
    pub(crate) netbox_device_role_api: Box<dyn KubeApiTrait<NetBoxDeviceRole> + Send + Sync>,
    pub(crate) netbox_manufacturer_api: Box<dyn KubeApiTrait<NetBoxManufacturer> + Send + Sync>,
    pub(crate) netbox_platform_api: Box<dyn KubeApiTrait<NetBoxPlatform> + Send + Sync>,
    pub(crate) netbox_device_type_api: Box<dyn KubeApiTrait<NetBoxDeviceType> + Send + Sync>,
    pub(crate) netbox_device_api: Box<dyn KubeApiTrait<NetBoxDevice> + Send + Sync>,
    pub(crate) netbox_interface_api: Box<dyn KubeApiTrait<NetBoxInterface> + Send + Sync>,
    pub(crate) netbox_mac_address_api: Box<dyn KubeApiTrait<NetBoxMACAddress> + Send + Sync>,
    pub(crate) netbox_region_api: Box<dyn KubeApiTrait<NetBoxRegion> + Send + Sync>,
    pub(crate) netbox_site_group_api: Box<dyn KubeApiTrait<NetBoxSiteGroup> + Send + Sync>,
    pub(crate) netbox_location_api: Box<dyn KubeApiTrait<NetBoxLocation> + Send + Sync>,
    /// Error count tracking per resource (namespace/name -> BackoffState)
    backoff_states: Arc<Mutex<HashMap<String, BackoffState>>>,
    /// Tag dependency tracking: tag namespace/name -> set of resources waiting for it
    /// Resource format: "kind:namespace/name" (e.g., "NetBoxIPAddress:default/web-server-ip")
    tag_dependencies: Arc<Mutex<HashMap<String, HashSet<String>>>>,
}

impl Reconciler {
    /// Helper to create status patch JSON with PascalCase state values
    /// CRD validation schemas expect PascalCase enum values ("Created", "Failed", etc.).
    /// This helper manually constructs the JSON with PascalCase state values to match the CRD schema.
    /// 
    /// NOTE: `lastReconciled` is only included if the state actually changed to prevent
    /// reconciliation loops from non-deterministic status updates.
    pub(crate) fn create_resource_status_patch(
        netbox_id: u64,
        netbox_url: String,
        state: ResourceState,
        error: Option<String>,
    ) -> serde_json::Value {
        let state_str = match state {
            ResourceState::Pending => "Pending",
            ResourceState::Created => "Created",
            ResourceState::Updated => "Updated",
            ResourceState::Failed => "Failed",
        };
        
        // Only include lastReconciled if state changed (not on every reconciliation)
        // This prevents reconciliation loops from non-deterministic status updates
        // The timestamp will only update when the state actually changes
        serde_json::json!({
            "status": {
                "netboxId": netbox_id,
                "netboxUrl": netbox_url,
                "state": state_str,
                "error": error,
                // Removed lastReconciled to prevent reconciliation loops
                // Controller already tracks reconciliation timing internally
            }
        })
    }
    
    /// Helper to create Prefix status patch with PascalCase state
    /// NOTE: `lastReconciled` removed to prevent reconciliation loops
    pub(crate) fn create_prefix_status_patch(
        netbox_id: u64,
        netbox_url: String,
        state: PrefixState,
        error: Option<String>,
    ) -> serde_json::Value {
        let state_str = match state {
            PrefixState::Pending => "Pending",
            PrefixState::Created => "Created",
            PrefixState::Updated => "Updated",
            PrefixState::Failed => "Failed",
        };
        
        serde_json::json!({
            "status": {
                "netboxId": netbox_id,
                "netboxUrl": netbox_url,
                "state": state_str,
                "error": error,
                // Removed lastReconciled to prevent reconciliation loops
            }
        })
    }
    
    // ============================================================================
    // Typed Status Update Helpers
    // ============================================================================
    // These helpers create typed status structs and serialize them to JSON
    // for use with kube-rs patch_status. This provides compile-time type safety
    // while maintaining compatibility with the existing patch mechanism.
    
    /// Create typed NetBoxRegionStatus and serialize to JSON patch
    pub(crate) fn create_typed_region_status_patch(
        netbox_id: u64,
        netbox_url: String,
        state: ResourceState,
        error: Option<String>,
    ) -> serde_json::Value {
        let status = crds::NetBoxRegionStatus {
            netbox_id: Some(netbox_id),
            netbox_url: Some(netbox_url),
            state,
            error,
            last_reconciled: None, // Removed to prevent reconciliation loops
        };
        serde_json::json!({ "status": status })
    }
    
    /// Create typed NetBoxSiteGroupStatus and serialize to JSON patch
    pub(crate) fn create_typed_site_group_status_patch(
        netbox_id: u64,
        netbox_url: String,
        state: ResourceState,
        error: Option<String>,
    ) -> serde_json::Value {
        let status = crds::NetBoxSiteGroupStatus {
            netbox_id: Some(netbox_id),
            netbox_url: Some(netbox_url),
            state,
            error,
            last_reconciled: None, // Removed to prevent reconciliation loops
        };
        serde_json::json!({ "status": status })
    }
    
    /// Create typed NetBoxDeviceRoleStatus and serialize to JSON patch
    pub(crate) fn create_typed_device_role_status_patch(
        netbox_id: u64,
        netbox_url: String,
        state: ResourceState,
        error: Option<String>,
    ) -> serde_json::Value {
        let status = crds::NetBoxDeviceRoleStatus {
            netbox_id: Some(netbox_id),
            netbox_url: Some(netbox_url),
            state,
            error,
            last_reconciled: None, // Removed to prevent reconciliation loops
        };
        serde_json::json!({ "status": status })
    }
    
    /// Create typed NetBoxManufacturerStatus and serialize to JSON patch
    pub(crate) fn create_typed_manufacturer_status_patch(
        netbox_id: u64,
        netbox_url: String,
        state: ResourceState,
        error: Option<String>,
    ) -> serde_json::Value {
        let status = crds::NetBoxManufacturerStatus {
            netbox_id: Some(netbox_id),
            netbox_url: Some(netbox_url),
            state,
            error,
            last_reconciled: None, // Removed to prevent reconciliation loops
        };
        serde_json::json!({ "status": status })
    }
    
    /// Create typed NetBoxPlatformStatus and serialize to JSON patch
    pub(crate) fn create_typed_platform_status_patch(
        netbox_id: u64,
        netbox_url: String,
        state: ResourceState,
        error: Option<String>,
    ) -> serde_json::Value {
        let status = crds::NetBoxPlatformStatus {
            netbox_id: Some(netbox_id),
            netbox_url: Some(netbox_url),
            state,
            error,
            last_reconciled: None, // Removed to prevent reconciliation loops
        };
        serde_json::json!({ "status": status })
    }
    
    /// Create typed NetBoxDeviceTypeStatus and serialize to JSON patch
    pub(crate) fn create_typed_device_type_status_patch(
        netbox_id: u64,
        netbox_url: String,
        state: ResourceState,
        error: Option<String>,
    ) -> serde_json::Value {
        let status = crds::NetBoxDeviceTypeStatus {
            netbox_id: Some(netbox_id),
            netbox_url: Some(netbox_url),
            state,
            error,
            last_reconciled: None, // Removed to prevent reconciliation loops
        };
        serde_json::json!({ "status": status })
    }
    
    /// Create typed NetBoxInterfaceStatus and serialize to JSON patch
    pub(crate) fn create_typed_interface_status_patch(
        netbox_id: u64,
        netbox_url: String,
        state: ResourceState,
        error: Option<String>,
    ) -> serde_json::Value {
        let status = crds::NetBoxInterfaceStatus {
            netbox_id: Some(netbox_id),
            netbox_url: Some(netbox_url),
            state,
            error,
            last_reconciled: None, // Removed to prevent reconciliation loops
        };
        serde_json::json!({ "status": status })
    }
    
    /// Create typed NetBoxMACAddressStatus and serialize to JSON patch
    pub(crate) fn create_typed_mac_address_status_patch(
        netbox_id: u64,
        netbox_url: String,
        state: ResourceState,
        error: Option<String>,
    ) -> serde_json::Value {
        let status = crds::NetBoxMACAddressStatus {
            netbox_id: Some(netbox_id),
            netbox_url: Some(netbox_url),
            state,
            error,
            last_reconciled: None, // Removed to prevent reconciliation loops
        };
        serde_json::json!({ "status": status })
    }
    
    /// Create typed NetBoxRoleStatus and serialize to JSON patch
    pub(crate) fn create_typed_role_status_patch(
        netbox_id: u64,
        netbox_url: String,
        state: ResourceState,
        error: Option<String>,
    ) -> serde_json::Value {
        let status = crds::NetBoxRoleStatus {
            netbox_id: Some(netbox_id),
            netbox_url: Some(netbox_url),
            state,
            error,
            last_reconciled: None, // Removed to prevent reconciliation loops
        };
        serde_json::json!({ "status": status })
    }
    
    /// Create typed NetBoxTagStatus and serialize to JSON patch
    pub(crate) fn create_typed_tag_status_patch(
        netbox_id: u64,
        netbox_url: String,
        state: ResourceState,
        error: Option<String>,
    ) -> serde_json::Value {
        let status = crds::NetBoxTagStatus {
            netbox_id: Some(netbox_id),
            netbox_url: Some(netbox_url),
            state,
            error,
            last_reconciled: None, // Removed to prevent reconciliation loops
        };
        serde_json::json!({ "status": status })
    }
    
    pub(crate) fn create_typed_rir_status_patch(
        netbox_id: u64,
        netbox_url: String,
        state: ResourceState,
        error: Option<String>,
    ) -> serde_json::Value {
        let status = crds::NetBoxRIRStatus {
            netbox_id: Some(netbox_id),
            netbox_url: Some(netbox_url),
            state,
            error,
            last_reconciled: None,
        };
        serde_json::json!({ "status": status })
    }

    pub(crate) fn create_typed_ip_address_status_patch(
        netbox_id: u64,
        netbox_url: String,
        address: Option<String>,
        state: ResourceState,
        error: Option<String>,
    ) -> serde_json::Value {
        let status = crds::NetBoxIPAddressStatus {
            netbox_id: Some(netbox_id),
            netbox_url: Some(netbox_url),
            address,
            state,
            error,
            last_reconciled: None,
        };
        serde_json::json!({ "status": status })
    }
    
    /// Create IPPool status patch with PascalCase state values
    pub(crate) fn create_ip_pool_status_patch(
        netbox_id: u64,
        netbox_url: String,
        state: IPPoolState,
        error: Option<String>,
    ) -> serde_json::Value {
        let state_str = match state {
            IPPoolState::Pending => "Pending",
            IPPoolState::Created => "Created",
            IPPoolState::Failed => "Failed",
        };
        serde_json::json!({
            "status": {
                "netboxId": netbox_id,
                "netboxUrl": netbox_url,
                "state": state_str,
                "error": error,
            }
        })
    }
    
    /// Create IPClaim status patch with PascalCase state values
    pub(crate) fn create_ip_claim_status_patch(
        netbox_id: u64,
        netbox_url: String,
        ip: Option<String>,
        state: IPClaimState,
        error: Option<String>,
    ) -> serde_json::Value {
        let state_str = match state {
            IPClaimState::Pending => "Pending",
            IPClaimState::Created => "Created",
            IPClaimState::Failed => "Failed",
        };
        serde_json::json!({
            "status": {
                "netboxId": netbox_id,
                "netboxUrl": netbox_url,
                "ip": ip,
                "state": state_str,
                "error": error,
            }
        })
    }
    
    /// Create typed NetBoxTenantGroupStatus and serialize to JSON patch
    pub(crate) fn create_typed_tenant_group_status_patch(
        netbox_id: u64,
        netbox_url: String,
        state: ResourceState,
        error: Option<String>,
    ) -> serde_json::Value {
        let status = crds::NetBoxTenantGroupStatus {
            netbox_id: Some(netbox_id),
            netbox_url: Some(netbox_url),
            state,
            error,
            last_reconciled: None, // Removed to prevent reconciliation loops
        };
        serde_json::json!({ "status": status })
    }
    
    /// Creates a new reconciler instance.
    pub fn new(
        token_resolver: Arc<dyn TokenResolverTrait>,
        secret_fetcher: Option<Arc<dyn SecretFetcher>>,
        event_recorder: Option<Arc<dyn crate::events::EventRecorderTrait>>,
        // IPAM APIs
        netbox_prefix_api: impl KubeApiTrait<NetBoxPrefix> + Send + Sync + 'static,
        netbox_role_api: impl KubeApiTrait<NetBoxRole> + Send + Sync + 'static,
        netbox_tag_api: impl KubeApiTrait<NetBoxTag> + Send + Sync + 'static,
        netbox_aggregate_api: impl KubeApiTrait<NetBoxAggregate> + Send + Sync + 'static,
        netbox_vlan_api: impl KubeApiTrait<NetBoxVLAN> + Send + Sync + 'static,
        netbox_rir_api: impl KubeApiTrait<NetBoxRIR> + Send + Sync + 'static,
        netbox_ip_address_api: impl KubeApiTrait<NetBoxIPAddress> + Send + Sync + 'static,
        netbox_ip_range_api: impl KubeApiTrait<NetBoxIPRange> + Send + Sync + 'static,
        netbox_vrf_api: impl KubeApiTrait<NetBoxVRF> + Send + Sync + 'static,
        netbox_route_target_api: impl KubeApiTrait<NetBoxRouteTarget> + Send + Sync + 'static,
        // IPAM: IPPool / IPClaim APIs
        netbox_ip_pool_api: impl KubeApiTrait<IPPool> + Send + Sync + 'static,
        netbox_ip_claim_api: impl KubeApiTrait<IPClaim> + Send + Sync + 'static,
        // Tenancy APIs
        netbox_tenant_api: impl KubeApiTrait<NetBoxTenant> + Send + Sync + 'static,
        netbox_tenant_group_api: impl KubeApiTrait<NetBoxTenantGroup> + Send + Sync + 'static,
        // DCIM APIs
        netbox_site_api: impl KubeApiTrait<NetBoxSite> + Send + Sync + 'static,
        netbox_device_role_api: impl KubeApiTrait<NetBoxDeviceRole> + Send + Sync + 'static,
        netbox_manufacturer_api: impl KubeApiTrait<NetBoxManufacturer> + Send + Sync + 'static,
        netbox_platform_api: impl KubeApiTrait<NetBoxPlatform> + Send + Sync + 'static,
        netbox_device_type_api: impl KubeApiTrait<NetBoxDeviceType> + Send + Sync + 'static,
        netbox_device_api: impl KubeApiTrait<NetBoxDevice> + Send + Sync + 'static,
        netbox_interface_api: impl KubeApiTrait<NetBoxInterface> + Send + Sync + 'static,
        netbox_mac_address_api: impl KubeApiTrait<NetBoxMACAddress> + Send + Sync + 'static,
        netbox_region_api: impl KubeApiTrait<NetBoxRegion> + Send + Sync + 'static,
        netbox_site_group_api: impl KubeApiTrait<NetBoxSiteGroup> + Send + Sync + 'static,
        netbox_location_api: impl KubeApiTrait<NetBoxLocation> + Send + Sync + 'static,
    ) -> Self {
        Self {
            token_resolver,
            secret_fetcher,
            event_recorder,
            // IPAM
            netbox_prefix_api: Box::new(netbox_prefix_api),
            netbox_role_api: Box::new(netbox_role_api),
            netbox_tag_api: Box::new(netbox_tag_api),
            netbox_aggregate_api: Box::new(netbox_aggregate_api),
            netbox_vlan_api: Box::new(netbox_vlan_api),
            netbox_rir_api: Box::new(netbox_rir_api),
            netbox_ip_address_api: Box::new(netbox_ip_address_api),
            netbox_ip_range_api: Box::new(netbox_ip_range_api),
            netbox_vrf_api: Box::new(netbox_vrf_api),
            netbox_route_target_api: Box::new(netbox_route_target_api),
            // IPAM: IPPool / IPClaim
            netbox_ip_pool_api: Box::new(netbox_ip_pool_api),
            netbox_ip_claim_api: Box::new(netbox_ip_claim_api),
            // Tenancy
            netbox_tenant_api: Box::new(netbox_tenant_api),
            netbox_tenant_group_api: Box::new(netbox_tenant_group_api),
            // DCIM
            netbox_site_api: Box::new(netbox_site_api),
            netbox_device_role_api: Box::new(netbox_device_role_api),
            netbox_manufacturer_api: Box::new(netbox_manufacturer_api),
            netbox_platform_api: Box::new(netbox_platform_api),
            netbox_device_type_api: Box::new(netbox_device_type_api),
            netbox_device_api: Box::new(netbox_device_api),
            netbox_interface_api: Box::new(netbox_interface_api),
            netbox_mac_address_api: Box::new(netbox_mac_address_api),
            netbox_region_api: Box::new(netbox_region_api),
            netbox_site_group_api: Box::new(netbox_site_group_api),
            netbox_location_api: Box::new(netbox_location_api),
            backoff_states: Arc::new(Mutex::new(HashMap::new())),
            tag_dependencies: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Register a resource as waiting for a tag
    /// 
    /// When a resource references a tag that doesn't exist yet, this method
    /// registers the dependency so that when the tag becomes available, the
    /// resource can be automatically requeued for reconciliation.
    /// 
    /// # Arguments
    /// - `tag_namespace`: Namespace of the tag
    /// - `tag_name`: Name of the tag
    /// - `resource_kind`: Kind of the resource waiting for the tag (e.g., "NetBoxIPAddress")
    /// - `resource_namespace`: Namespace of the resource
    /// - `resource_name`: Name of the resource
    pub(crate) fn register_tag_dependency(
        &self,
        tag_namespace: &str,
        tag_name: &str,
        resource_kind: &str,
        resource_namespace: &str,
        resource_name: &str,
    ) {
        let tag_key = format!("{}/{}", tag_namespace, tag_name);
        let resource_key = format!("{}:{}/{}", resource_kind, resource_namespace, resource_name);
        
        let tag_key_clone = tag_key.clone();
        let resource_key_clone = resource_key.clone();
        
        let mut deps = self.tag_dependencies.lock().unwrap();
        deps.entry(tag_key).or_insert_with(HashSet::new).insert(resource_key);
        
        debug!("Registered tag dependency: {} -> {}", tag_key_clone, resource_key_clone);
    }
    
    /// Unregister a resource from waiting for a tag
    /// 
    /// When a resource successfully resolves all its tags, this method
    /// removes it from the dependency tracking to prevent unnecessary requeues.
    /// 
    /// # Arguments
    /// - `tag_namespace`: Namespace of the tag
    /// - `tag_name`: Name of the tag
    /// - `resource_kind`: Kind of the resource
    /// - `resource_namespace`: Namespace of the resource
    /// - `resource_name`: Name of the resource
    pub(crate) fn unregister_tag_dependency(
        &self,
        tag_namespace: &str,
        tag_name: &str,
        resource_kind: &str,
        resource_namespace: &str,
        resource_name: &str,
    ) {
        let tag_key = format!("{}/{}", tag_namespace, tag_name);
        let resource_key = format!("{}:{}/{}", resource_kind, resource_namespace, resource_name);
        
        let tag_key_clone = tag_key.clone();
        let resource_key_clone = resource_key.clone();
        
        let mut deps = self.tag_dependencies.lock().unwrap();
        if let Some(resources) = deps.get_mut(&tag_key) {
            resources.remove(&resource_key);
            if resources.is_empty() {
                deps.remove(&tag_key);
            }
            debug!("Unregistered tag dependency: {} -> {}", tag_key_clone, resource_key_clone);
        }
    }
    
    /// Trigger reconciliation of all resources waiting for a tag
    /// 
    /// When a tag is successfully created or updated, this method finds all
    /// resources that were waiting for it and triggers their reconciliation.
    /// 
    /// # Arguments
    /// - `tag_namespace`: Namespace of the tag that became available
    /// - `tag_name`: Name of the tag that became available
    pub(crate) async fn trigger_dependent_resource_reconciliation(
        &self,
        tag_namespace: &str,
        tag_name: &str,
    ) {
        let tag_key = format!("{}/{}", tag_namespace, tag_name);
        
        let dependent_resources: Vec<String> = {
            let deps = self.tag_dependencies.lock().unwrap();
            deps.get(&tag_key)
                .map(|resources| resources.iter().cloned().collect())
                .unwrap_or_default()
        };
        
        if dependent_resources.is_empty() {
            debug!("No resources waiting for tag {}", tag_key);
            return;
        }
        
        info!("Tag {} became available, triggering reconciliation of {} dependent resource(s)", tag_key, dependent_resources.len());
        
        // Trigger reconciliation for each dependent resource by patching the resource
        // This causes the watcher to detect a change and requeue the resource
        for resource_key in dependent_resources {
            let parts: Vec<&str> = resource_key.split(':').collect();
            if parts.len() != 2 {
                warn!("Invalid resource key format: {}", resource_key);
                continue;
            }
            
            let resource_kind = parts[0];
            let resource_path: Vec<&str> = parts[1].split('/').collect();
            if resource_path.len() != 2 {
                warn!("Invalid resource path format: {}", parts[1]);
                continue;
            }
            
            let resource_namespace = resource_path[0];
            let resource_name = resource_path[1];
            
            info!("Triggering reconciliation of {} {}/{} (was waiting for tag {})", 
                resource_kind, resource_namespace, resource_name, tag_key);
            
            // Trigger requeue by patching the resource with a timestamp-based annotation
            // This causes the watcher to detect a change and requeue immediately
            let annotation_key = "dcops.microscaler.io/tag-triggered-reconcile";
            let annotation_value = format!("{}", chrono::Utc::now().timestamp());
            
            // Trigger requeue by patching the resource with an annotation
            // This causes the watcher to detect a change and requeue immediately
            let patch_result = match resource_kind {
                "NetBoxIPAddress" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_ip_address_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxDevice" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_device_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxInterface" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_interface_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxMACAddress" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_mac_address_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxPrefix" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_prefix_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxIPRange" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_ip_range_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxVRF" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_vrf_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxRouteTarget" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_route_target_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxTenant" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_tenant_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxTenantGroup" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_tenant_group_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxSite" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_site_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxVLAN" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_vlan_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxAggregate" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_aggregate_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxRole" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_role_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxPlatform" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_platform_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxManufacturer" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_manufacturer_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxDeviceType" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_device_type_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxDeviceRole" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_device_role_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxRegion" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_region_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxSiteGroup" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_site_group_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxLocation" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_location_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                "NetBoxRIR" => {
                    self.trigger_resource_requeue_via_annotation_api(
                        &*self.netbox_rir_api,
                        resource_name, resource_namespace, &annotation_key, &annotation_value
                    ).await
                }
                _ => {
                    warn!("Unsupported resource kind for tag-triggered reconciliation: {} (will be picked up by periodic requeue)", resource_kind);
                    Ok(())
                }
            };
            
            if let Err(e) = patch_result {
                warn!("Failed to trigger requeue for {} {}/{}: {} (will be picked up by periodic requeue)", 
                    resource_kind, resource_namespace, resource_name, e);
            }
        }
    }
    
    /// Helper to trigger resource requeue by adding/updating an annotation via KubeApiTrait
    /// 
    /// This patches the resource with a timestamp-based annotation, which causes
    /// the Kubernetes watcher to detect a change and immediately requeue the resource
    /// for reconciliation. This is much faster than waiting for the 10-second periodic requeue.
    async fn trigger_resource_requeue_via_annotation_api<K>(
        &self,
        api: &dyn KubeApiTrait<K>,
        name: &str,
        namespace: &str,
        annotation_key: &str,
        annotation_value: &str,
    ) -> Result<(), ControllerError>
    where
        K: kube::Resource + Clone + Send + Sync + std::fmt::Debug + serde::de::DeserializeOwned + 'static,
        K::DynamicType: Default,
    {
        use kube::api::{Patch, PatchParams};
        
        // Create a patch that adds/updates the annotation
        let patch_body = serde_json::json!({
            "metadata": {
                "annotations": {
                    annotation_key: annotation_value
                }
            }
        });
        
        let patch = Patch::Merge(patch_body);
        let params = PatchParams::default();
        
        // Patch the resource to trigger watcher
        match api.patch(name, &params, &patch).await {
            Ok(_) => {
                debug!("Successfully triggered requeue for {}/{} via annotation {}={}", 
                    namespace, name, annotation_key, annotation_value);
                Ok(())
            }
            Err(e) => {
                // If patching fails, log but don't error - periodic requeue will pick it up
                debug!("Failed to patch annotation for {}/{}: {} (will be picked up by periodic requeue)", 
                    namespace, name, e);
                Ok(()) // Return Ok to continue with other resources
            }
        }
    }
    
    /// Helper method to record a Normal event
    pub(crate) async fn record_event_normal<K: kube::Resource>(&self, reason: &str, message: &str, obj: &K) 
    where
        K::DynamicType: Default,
    {
        if let Some(recorder) = &self.event_recorder {
            use crate::events::{record_event_normal_helper, EventRecorderTrait};
            record_event_normal_helper(recorder.as_ref(), reason, message, obj).await;
        }
    }
    
    /// Helper method to record a Warning event
    pub(crate) async fn record_event_warning<K: kube::Resource>(&self, reason: &str, message: &str, obj: &K)
    where
        K::DynamicType: Default,
    {
        if let Some(recorder) = &self.event_recorder {
            use crate::events::{record_event_warning_helper, EventRecorderTrait};
            record_event_warning_helper(recorder.as_ref(), reason, message, obj).await;
        }
    }
    
    /// Helper method to record a retry attempt event
    /// This is called when a reconciliation fails and will be retried with backoff
    /// Takes a string error message to avoid borrowing issues in closures
    pub(crate) async fn record_event_retry_attempt_str<K: kube::Resource>(
        &self,
        error_str: &str,
        attempt: u32,
        backoff_seconds: u64,
        obj: &K,
    )
    where
        K::DynamicType: Default,
    {
        if let Some(recorder) = &self.event_recorder {
            use crate::events::{record_event_warning_helper, reasons, EventRecorderTrait};
            let message = format!(
                "Retrying reconciliation after error (attempt {}, backoff: {}s): {}",
                attempt, backoff_seconds, error_str
            );
            record_event_warning_helper(recorder.as_ref(), reasons::RETRY_ATTEMPT, &message, obj).await;
        }
    }
    
    /// Performs startup reconciliation to map existing NetBox resources back to Kubernetes CRs.
    ///
    /// This is called when the controller starts up to ensure that:
    /// 1. CRs that were created before controller restart are mapped to their NetBox IDs
    /// 2. Resources that exist in NetBox but don't have status.netbox_id are discovered
    ///
    /// Strategy:
    /// - List all NetBoxPrefix CRs
    /// - For each CR without a netbox_id, query NetBox by prefix CIDR
    /// - If found, update the CR status with the NetBox ID
    pub async fn startup_reconciliation(&self) -> Result<(), ControllerError> {
        info!("Starting startup reconciliation for NetBoxPrefix resources...");
        
        // List all NetBoxPrefix CRs
        let prefixes = match self.netbox_prefix_api.list(&kube::api::ListParams::default()).await {
            Ok(list) => list,
            Err(e) => {
                error!("Failed to list NetBoxPrefix CRs: {}", e);
                return Err(ControllerError::Kube(e.into()));
            }
        };
        
        info!("Found {} NetBoxPrefix CRs to reconcile", prefixes.items.len());
        
        let mut mapped_count = 0;
        let mut not_found_count = 0;
        
        for prefix_crd in prefixes.items {
            let name = prefix_crd.metadata.name.as_ref()
                .ok_or_else(|| ControllerError::InvalidConfig("NetBoxPrefix missing name".to_string()))?;
            let namespace = prefix_crd.metadata.namespace.as_deref()
                .unwrap_or("default");
            
            // Skip if already has netbox_id
            if let Some(status) = &prefix_crd.status {
                if status.netbox_id.is_some() {
                    debug!("NetBoxPrefix {}/{} already has netbox_id, skipping", namespace, name);
                    continue;
                }
            }
            
            // Get tenant-specific client for this prefix
            let tenant_ref = &prefix_crd.spec.tenant;
            let netbox_client = match self.token_resolver
                .create_client_for_tenant(namespace, tenant_ref)
                .await
            {
                Ok(client) => client,
                Err(e) => {
                    warn!("Failed to resolve token for tenant {} in prefix {}/{}: {}", 
                        tenant_ref.name, namespace, name, e);
                    continue; // Skip this prefix, will be reconciled later
                }
            };
            
            // Try to find this prefix in NetBox by CIDR
            let prefix_cidr = &prefix_crd.spec.prefix;
            
            // Convert prefix_cidr string to IpNet for comparison (used in multiple places)
            use std::str::FromStr;
            use ipnet::IpNet;
            let prefix_net = IpNet::from_str(prefix_cidr)
                .map_err(|e| ControllerError::InvalidIPFormat(format!("Invalid prefix format: {} - {}", prefix_cidr, e)))?;
            
            info!("Mapping NetBoxPrefix {}/{} (prefix: {}) to NetBox resource...", namespace, name, prefix_net);
            
            // Try multiple methods to find the prefix:
            // 1. Direct get by ID (if we have a hint)
            // 2. Query by prefix CIDR (if deserialization works)
            // 3. List all prefixes and match by CIDR (fallback)
            
            let netbox_prefix = if let Ok(prefixes) = netbox_client.query_prefixes(
                &[("prefix", prefix_cidr)],
                false,
            ).await {
                // Query succeeded, check if we found a match
                if let Some(found) = prefixes.iter().find(|p| p.prefix == prefix_net) {
                    Some(found.clone())
                } else {
                    None
                }
            } else {
                // Query failed (deserialization issue), try fallback: get by ID 1 and check
                warn!("Query failed for prefix {}, trying fallback method", prefix_net);
                match netbox_client.get_prefix(PrefixId(1)).await {
                    Ok(prefix) if prefix.prefix == prefix_net => {
                        info!("Found prefix {} via fallback method (ID: 1)", prefix_net);
                        Some(prefix)
                    }
                    _ => {
                        // Try to list all prefixes (if NetBox supports it without filters)
                        // For now, we'll just log and continue
                        warn!("Could not map prefix {} to NetBox resource", prefix_cidr);
                        None
                    }
                }
            };
            
            if let Some(netbox_prefix) = netbox_prefix {
                // Update CR status with NetBox ID
                // Update status (use lowercase state to match CRD validation schema)
                let status_patch = Self::create_prefix_status_patch(
                    netbox_prefix.id,
                    netbox_prefix.url.clone(),
                    PrefixState::Created,
                    None,
                );
                
                let pp = kube::api::PatchParams::default();
                match self.netbox_prefix_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    Ok(_) => {
                        info!("✅ Mapped NetBoxPrefix {}/{} to NetBox ID {}", namespace, name, netbox_prefix.id);
                        mapped_count += 1;
                    }
                    Err(e) => {
                        error!("Failed to update NetBoxPrefix {}/{} status: {}", namespace, name, e);
                    }
                }
            } else {
                warn!("⚠️  Could not find NetBox resource for prefix {}", prefix_cidr);
                not_found_count += 1;
            }
        }
        
        info!("Startup reconciliation complete: {} mapped, {} not found", mapped_count, not_found_count);
        Ok(())
    }

    /// Get the Fibonacci backoff duration for a resource based on its error count
    ///
    /// Returns (backoff_seconds, error_count)
    pub fn get_backoff_for_resource(&self, resource_key: &str) -> (u64, u32) {
        match self.backoff_states.lock() {
            Ok(mut states) => {
                let state = states
                    .entry(resource_key.to_string())
                    .or_insert_with(|| BackoffState::new());
                let backoff_seconds = state.backoff.next_backoff_seconds();
                let error_count = state.error_count;
                (backoff_seconds, error_count)
            }
            Err(e) => {
                warn!("Failed to lock backoff_states: {}, using default backoff", e);
                (60, 0) // 60 seconds default
            }
        }
    }

    /// Increment error count for a resource
    pub fn increment_error(&self, resource_key: &str) {
        if let Ok(mut states) = self.backoff_states.lock() {
            let state = states
                .entry(resource_key.to_string())
                .or_insert_with(|| BackoffState::new());
            state.increment_error();
        }
    }

    /// Reset error count for a resource (on successful reconciliation)
    pub fn reset_error(&self, resource_key: &str) {
        if let Ok(mut states) = self.backoff_states.lock() {
            if let Some(state) = states.get_mut(resource_key) {
                state.reset();
            }
        }
    }
}

// Re-exports are not needed - reconciler methods are accessed via impl Reconciler

