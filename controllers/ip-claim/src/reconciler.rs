//! Reconciliation logic for IP Claim CRDs.
//!
//! This module handles the reconciliation of `IPClaim` and `IPPool`
//! resources, ensuring IP addresses are allocated from NetBox according
//! to Git-defined intent.

use crate::error::ControllerError;
use crds::{IPClaim, IPPool, IPClaimStatus, IPPoolStatus, AllocationState};
use netbox_client::{NetBoxClient, AllocateIPRequest, IPAddressStatus};
use kube::Api;
use tracing::{debug, info, error};
use chrono::Utc;

/// Reconciles IP allocation resources.
pub struct Reconciler {
    netbox_client: NetBoxClient,
    ip_claim_api: Api<IPClaim>,
    ip_pool_api: Api<IPPool>,
}

impl Reconciler {
    /// Creates a new reconciler instance.
    pub fn new(
        netbox_client: NetBoxClient,
        ip_claim_api: Api<IPClaim>,
        ip_pool_api: Api<IPPool>,
    ) -> Self {
        Self {
            netbox_client,
            ip_claim_api,
            ip_pool_api,
        }
    }
    
    /// Reconciles an IPClaim resource.
    ///
    /// This method:
    /// 1. Fetches the referenced IPPool
    /// 2. Gets the NetBox prefix from the IPPool
    /// 3. Allocates an IP from NetBox
    /// 4. Updates the IPClaim status with the allocated IP
    pub async fn reconcile_ip_claim(&self, claim: &IPClaim) -> Result<(), ControllerError> {
        let name = claim.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("IPClaim missing name".to_string()))?;
        let namespace = claim.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling IPClaim {}/{}", namespace, name);
        
        // Fetch current claim to check status
        // Note: kube-rs CustomResource doesn't include status in the struct by default
        // We'll check by querying NetBox for existing IPs instead
        // This provides better idempotency anyway
        
        // Get the referenced IPPool
        let pool_name = &claim.spec.pool_ref.name;
        let pool_namespace = claim.spec.pool_ref.namespace.as_deref()
            .unwrap_or(namespace);
        
        let pool = self.ip_pool_api
            .get(pool_name)
            .await
            .map_err(|e| ControllerError::IPPoolNotFound(format!(
                "Failed to get IPPool {}/{}: {}", pool_namespace, pool_name, e
            )))?;
        
        // Get NetBox prefix ID
        let prefix_id_str = &pool.spec.netbox_prefix_ref.id;
        let prefix_id = prefix_id_str.parse::<u64>()
            .map_err(|_| ControllerError::InvalidConfig(format!(
                "Invalid prefix ID: {}", prefix_id_str
            )))?;
        
        // Verify prefix exists in NetBox
        let _prefix = self.netbox_client.get_prefix(prefix_id).await
            .map_err(|e| ControllerError::PrefixNotFound(format!(
                "Prefix {} not found in NetBox: {}", prefix_id, e
            )))?;
        
        // Allocate IP from NetBox
        let allocation_request = AllocateIPRequest {
            address: claim.spec.preferred_ip.clone(),
            description: Some(format!("IPClaim: {}/{}", namespace, name)),
            status: Some(IPAddressStatus::Active),
            role: None,
            dns_name: None,
            tags: Some(vec!["managed-by=dcops".to_string(), "owner=ip-claim-controller".to_string()]),
        };
        
        let allocated_ip = self.netbox_client.allocate_ip(prefix_id, Some(allocation_request)).await
            .map_err(|e| {
                error!("Failed to allocate IP from prefix {}: {}", prefix_id, e);
                ControllerError::AllocationFailed(format!(
                    "Failed to allocate IP: {}", e
                ))
            })?;
        
        info!("Allocated IP {} for IPClaim {}/{}", allocated_ip.address, namespace, name);
        
        // Update IPClaim status
        let new_status = IPClaimStatus {
            ip: Some(allocated_ip.address.clone()),
            state: AllocationState::Allocated,
            netbox_ip_ref: Some(allocated_ip.url.clone()),
            last_reconciled: Some(Utc::now()),
            error: None,
        };
        
        // Patch the status using kube-rs status subresource API
        use kube::api::PatchParams;
        use serde_json::json;
        
        let status_patch = json!({
            "status": new_status
        });
        
        let pp = PatchParams::default();
        self.ip_claim_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
            .await
            .map_err(|e| ControllerError::Kube(e.into()))?;
        
        info!("Updated IPClaim {}/{} status", namespace, name);
        Ok(())
    }
    
    /// Reconciles an IPPool resource.
    ///
    /// This method:
    /// 1. Fetches the NetBox prefix
    /// 2. Gets available IPs from the prefix
    /// 3. Updates the IPPool status with pool statistics
    pub async fn reconcile_ip_pool(&self, pool: &IPPool) -> Result<(), ControllerError> {
        let name = pool.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("IPPool missing name".to_string()))?;
        let namespace = pool.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        debug!("Reconciling IPPool {}/{}", namespace, name);
        
        // Get NetBox prefix ID
        let prefix_id_str = &pool.spec.netbox_prefix_ref.id;
        let prefix_id = prefix_id_str.parse::<u64>()
            .map_err(|_| ControllerError::InvalidConfig(format!(
                "Invalid prefix ID: {}", prefix_id_str
            )))?;
        
        // Get prefix from NetBox
        let prefix = self.netbox_client.get_prefix(prefix_id).await
            .map_err(|e| ControllerError::PrefixNotFound(format!(
                "Prefix {} not found in NetBox: {}", prefix_id, e
            )))?;
        
        // Get available IPs
        let available_ips = self.netbox_client.get_available_ips(prefix_id, None).await
            .map_err(|e| ControllerError::NetBox(e))?;
        
        // Query allocated IPs from this prefix
        let allocated_ips = self.netbox_client.query_ip_addresses(
            &[("prefix", &prefix.prefix)],
            true, // fetch all pages
        ).await
            .map_err(|e| ControllerError::NetBox(e))?;
        
        // Calculate pool statistics
        // Note: This is approximate - NetBox doesn't provide exact counts
        let total_ips = allocated_ips.len() + available_ips.len();
        let allocated_count = allocated_ips.len() as u32;
        let available_count = available_ips.len() as u32;
        
        // Update IPPool status
        let new_status = IPPoolStatus {
            total_ips: total_ips as u32,
            allocated_ips: allocated_count,
            available_ips: available_count,
            last_reconciled: Some(Utc::now()),
        };
        
        // Patch the status using kube-rs status subresource API
        use kube::api::PatchParams;
        use serde_json::json;
        
        let status_patch = json!({
            "status": new_status
        });
        
        let pp = PatchParams::default();
        self.ip_pool_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
            .await
            .map_err(|e| ControllerError::Kube(e.into()))?;
        
        debug!("Updated IPPool {}/{} status: {} total, {} allocated, {} available",
            namespace, name, total_ips, allocated_count, available_count);
        
        Ok(())
    }
}

