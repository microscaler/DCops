//! Reconciliation logic for NetBox-related CRDs.
//!
//! This module is organized by NetBox API sections:
//! - `ipam`: IP Address Management (Prefixes, Aggregates, IPClaims, IPPools)
//! - `tenancy`: Tenancy (Tenants)
//! - `dcim`: Data Center Infrastructure Management (Sites, Devices, Interfaces, etc.)
//! - `extras`: Extras (Roles, Tags)

pub mod ipam;
pub mod tenancy;
#[cfg(test)]
pub mod tenancy_test;
pub mod dcim;
pub mod extras;

use crate::error::ControllerError;
use crate::backoff::FibonacciBackoff;
use crate::kube_api_trait::KubeApiTrait;
use crate::token_resolver::TokenResolverTrait;
use netbox_client::{NetBoxClientTrait, PrefixId};
use crds::{
    IPClaim, IPPool, NetBoxPrefix, NetBoxTenant, NetBoxSite, NetBoxRole, NetBoxTag, NetBoxAggregate,
    NetBoxDeviceRole, NetBoxManufacturer, NetBoxPlatform, NetBoxDeviceType, NetBoxDevice,
    NetBoxInterface, NetBoxMACAddress, NetBoxVLAN, NetBoxRegion, NetBoxSiteGroup, NetBoxLocation,
    NetBoxRIR, PrefixState, ResourceState,
};
use tracing::{info, error, debug, warn};
use std::collections::HashMap;
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
    // IPAM APIs
    pub(crate) netbox_prefix_api: Box<dyn KubeApiTrait<NetBoxPrefix> + Send + Sync>,
    pub(crate) netbox_role_api: Box<dyn KubeApiTrait<NetBoxRole> + Send + Sync>,
    pub(crate) netbox_tag_api: Box<dyn KubeApiTrait<NetBoxTag> + Send + Sync>,
    pub(crate) netbox_aggregate_api: Box<dyn KubeApiTrait<NetBoxAggregate> + Send + Sync>,
    pub(crate) netbox_vlan_api: Box<dyn KubeApiTrait<NetBoxVLAN> + Send + Sync>,
    pub(crate) netbox_rir_api: Box<dyn KubeApiTrait<NetBoxRIR> + Send + Sync>,
    // Tenancy APIs
    pub(crate) netbox_tenant_api: Box<dyn KubeApiTrait<NetBoxTenant> + Send + Sync>,
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
    // Custom CRDs
    pub(crate) ip_pool_api: Box<dyn KubeApiTrait<IPPool> + Send + Sync>,
    pub(crate) ip_claim_api: Box<dyn KubeApiTrait<IPClaim> + Send + Sync>,
    /// Error count tracking per resource (namespace/name -> BackoffState)
    backoff_states: Arc<Mutex<HashMap<String, BackoffState>>>,
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
    
    /// Helper to create IPClaim status patch with PascalCase state
    /// NOTE: `lastReconciled` removed to prevent reconciliation loops
    pub(crate) fn create_ipclaim_status_patch(
        ip: Option<String>,
        state: crds::AllocationState,
        netbox_ip_ref: Option<String>,
        error: Option<String>,
    ) -> serde_json::Value {
        let state_str = match state {
            crds::AllocationState::Pending => "Pending",
            crds::AllocationState::Allocated => "Allocated",
            crds::AllocationState::Failed => "Failed",
        };
        
        serde_json::json!({
            "status": {
                "ip": ip,
                "state": state_str,
                "netboxIpRef": netbox_ip_ref,
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
    
    /// Creates a new reconciler instance.
    pub fn new(
        token_resolver: Arc<dyn TokenResolverTrait>,
        // IPAM APIs
        netbox_prefix_api: impl KubeApiTrait<NetBoxPrefix> + Send + Sync + 'static,
        netbox_role_api: impl KubeApiTrait<NetBoxRole> + Send + Sync + 'static,
        netbox_tag_api: impl KubeApiTrait<NetBoxTag> + Send + Sync + 'static,
        netbox_aggregate_api: impl KubeApiTrait<NetBoxAggregate> + Send + Sync + 'static,
        netbox_vlan_api: impl KubeApiTrait<NetBoxVLAN> + Send + Sync + 'static,
        netbox_rir_api: impl KubeApiTrait<NetBoxRIR> + Send + Sync + 'static,
        // Tenancy APIs
        netbox_tenant_api: impl KubeApiTrait<NetBoxTenant> + Send + Sync + 'static,
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
        // Custom CRDs
        ip_pool_api: impl KubeApiTrait<IPPool> + Send + Sync + 'static,
        ip_claim_api: impl KubeApiTrait<IPClaim> + Send + Sync + 'static,
    ) -> Self {
        Self {
            token_resolver,
            // IPAM
            netbox_prefix_api: Box::new(netbox_prefix_api),
            netbox_role_api: Box::new(netbox_role_api),
            netbox_tag_api: Box::new(netbox_tag_api),
            netbox_aggregate_api: Box::new(netbox_aggregate_api),
            netbox_vlan_api: Box::new(netbox_vlan_api),
            netbox_rir_api: Box::new(netbox_rir_api),
            // Tenancy
            netbox_tenant_api: Box::new(netbox_tenant_api),
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
            // Custom
            ip_pool_api: Box::new(ip_pool_api),
            ip_claim_api: Box::new(ip_claim_api),
            backoff_states: Arc::new(Mutex::new(HashMap::new())),
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
            info!("Mapping NetBoxPrefix {}/{} (prefix: {}) to NetBox resource...", namespace, name, prefix_cidr);
            
            // Try multiple methods to find the prefix:
            // 1. Direct get by ID (if we have a hint)
            // 2. Query by prefix CIDR (if deserialization works)
            // 3. List all prefixes and match by CIDR (fallback)
            
            let netbox_prefix = if let Ok(prefixes) = netbox_client.query_prefixes(
                &[("prefix", prefix_cidr)],
                false,
            ).await {
                // Query succeeded, check if we found a match
                if let Some(found) = prefixes.iter().find(|p| p.prefix == *prefix_cidr) {
                    Some(found.clone())
                } else {
                    None
                }
            } else {
                // Query failed (deserialization issue), try fallback: get by ID 1 and check
                warn!("Query failed for prefix {}, trying fallback method", prefix_cidr);
                match netbox_client.get_prefix(PrefixId(1)).await {
                    Ok(prefix) if prefix.prefix == *prefix_cidr => {
                        info!("Found prefix {} via fallback method (ID: 1)", prefix_cidr);
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

