//! Reconciliation logic for NetBox-related CRDs.
//!
//! This module is organized by NetBox API sections:
//! - `ipam`: IP Address Management (Prefixes, Aggregates, IPClaims, IPPools)
//! - `tenancy`: Tenancy (Tenants)
//! - `dcim`: Data Center Infrastructure Management (Sites, Devices, Interfaces, etc.)
//! - `extras`: Extras (Roles, Tags)

pub mod ipam;
pub mod tenancy;
pub mod dcim;
pub mod extras;

use crate::error::ControllerError;
use crate::backoff::FibonacciBackoff;
use crds::{
    IPClaim, IPPool, NetBoxPrefix, NetBoxTenant, NetBoxSite, NetBoxRole, NetBoxTag, NetBoxAggregate,
    NetBoxDeviceRole, NetBoxManufacturer, NetBoxPlatform, NetBoxDeviceType, NetBoxDevice,
    NetBoxInterface, NetBoxMACAddress, NetBoxVLAN, NetBoxRegion, NetBoxSiteGroup, NetBoxLocation,
    PrefixState, ResourceState,
};
use netbox_client::NetBoxClientTrait;
use kube::Api;
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
    pub(crate) netbox_client: Box<dyn NetBoxClientTrait + Send + Sync>,
    // IPAM APIs
    pub(crate) netbox_prefix_api: Api<NetBoxPrefix>,
    pub(crate) netbox_role_api: Api<NetBoxRole>,
    pub(crate) netbox_tag_api: Api<NetBoxTag>,
    pub(crate) netbox_aggregate_api: Api<NetBoxAggregate>,
    pub(crate) netbox_vlan_api: Api<NetBoxVLAN>,
    // Tenancy APIs
    pub(crate) netbox_tenant_api: Api<NetBoxTenant>,
    // DCIM APIs
    pub(crate) netbox_site_api: Api<NetBoxSite>,
    pub(crate) netbox_device_role_api: Api<NetBoxDeviceRole>,
    pub(crate) netbox_manufacturer_api: Api<NetBoxManufacturer>,
    pub(crate) netbox_platform_api: Api<NetBoxPlatform>,
    pub(crate) netbox_device_type_api: Api<NetBoxDeviceType>,
    pub(crate) netbox_device_api: Api<NetBoxDevice>,
    pub(crate) netbox_interface_api: Api<NetBoxInterface>,
    pub(crate) netbox_mac_address_api: Api<NetBoxMACAddress>,
    pub(crate) netbox_region_api: Api<NetBoxRegion>,
    pub(crate) netbox_site_group_api: Api<NetBoxSiteGroup>,
    pub(crate) netbox_location_api: Api<NetBoxLocation>,
    // Custom CRDs
    pub(crate) ip_pool_api: Api<IPPool>,
    pub(crate) ip_claim_api: Api<IPClaim>,
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
    
    /// Creates a new reconciler instance.
    pub fn new(
        netbox_client: impl NetBoxClientTrait + Send + Sync + 'static,
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
            netbox_client: Box::new(netbox_client),
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
        let prefixes = match self.netbox_prefix_api.list(&Default::default()).await {
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
            
            // Try to find this prefix in NetBox by CIDR
            let prefix_cidr = &prefix_crd.spec.prefix;
            info!("Mapping NetBoxPrefix {}/{} (prefix: {}) to NetBox resource...", namespace, name, prefix_cidr);
            
            // Try multiple methods to find the prefix:
            // 1. Direct get by ID (if we have a hint)
            // 2. Query by prefix CIDR (if deserialization works)
            // 3. List all prefixes and match by CIDR (fallback)
            
            let netbox_prefix = if let Ok(prefixes) = self.netbox_client.query_prefixes(
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
                match self.netbox_client.get_prefix(1).await {
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
                    .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
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

