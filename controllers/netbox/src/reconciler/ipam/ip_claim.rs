//! IPClaim reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use kube::Api;
use tracing::{info, error, debug, warn};
use crds::{IPClaim, IPClaimStatus, AllocationState};
use netbox_client::{AllocateIPRequest, IPAddressStatus};

impl Reconciler {
    pub async fn reconcile_ip_claim(&self, claim: &IPClaim) -> Result<(), ControllerError> {
        let name = claim.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("IPClaim missing name".to_string()))?;
        let namespace = claim.metadata.namespace.as_deref()
            .unwrap_or("default");
        let resource_key = format!("{}/{}", namespace, name);
        
        info!("Reconciling IPClaim {}/{}", namespace, name);
        
        // Helper function to update status with error (only if error changed)
        async fn update_status_error(
            api: &Api<IPClaim>,
            name: &str,
            namespace: &str,
            error_msg: String,
            current_status: Option<&IPClaimStatus>,
        ) {
            // Check if error is already set to avoid unnecessary updates
            if let Some(status) = current_status {
                if status.state == AllocationState::Failed && status.error.as_ref() == Some(&error_msg) {
                    debug!("IPClaim {}/{} already has this error in status, skipping update", namespace, name);
                    return;
                }
            }
            
            // Update status with error (use lowercase state to match CRD validation schema)
            let status_patch = Reconciler::create_ipclaim_status_patch(
                None,
                AllocationState::Failed,
                None,
                Some(error_msg.clone()),
            );
            
            let pp = kube::api::PatchParams::default();
            if let Err(e) = api
                .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                .await
            {
                error!("Failed to update IPClaim {}/{} error status: {}", namespace, name, e);
            } else {
                info!("Updated IPClaim {}/{} status with error", namespace, name);
            }
        }
        
        // Check if already allocated and in good state
        if let Some(status) = &claim.status {
            if status.state == AllocationState::Allocated && status.ip.is_some() {
                info!("IPClaim {}/{} already allocated to {}", namespace, name, status.ip.as_ref().unwrap());
                // TODO: Verify NetBox state matches
                return Ok(());
            }
            
            // If already failed, check if we should skip reconciliation
            // This prevents infinite loops when status updates trigger new Apply events
            if status.state == AllocationState::Failed {
                if let Some(error) = &status.error {
                    // If error is about prefix not found or authentication, don't retry immediately
                    // The status update will happen once, then we'll skip subsequent reconciliations
                    if error.contains("not found") || error.contains("Invalid token") || error.contains("403 Forbidden") || error.contains("Prefix") {
                        debug!("IPClaim {}/{} already marked as failed with error: {}, skipping reconciliation to prevent loop", namespace, name, error);
                        return Ok(()); // Return Ok to prevent retry loop, but don't update status
                    }
                }
            }
        }
        
        // Get the referenced IPPool
        let pool_name = &claim.spec.pool_ref.name;
        let pool_namespace = claim.spec.pool_ref.namespace.as_deref()
            .unwrap_or(namespace);
        
        let pool = match self.ip_pool_api.get(pool_name).await {
            Ok(p) => p,
            Err(e) => {
                let error_msg = format!("Failed to get IPPool {}/{}: {}", pool_namespace, pool_name, e);
                error!("{}", error_msg);
                update_status_error(&self.ip_claim_api, name, namespace, error_msg.clone(), claim.status.as_ref()).await;
                return Err(ControllerError::IPPoolNotFound(error_msg));
            }
        };
        
        // Resolve NetBox prefix ID from IPPool
        // The id field can be either:
        // 1. A direct NetBox prefix ID (numeric string like "1")
        // 2. A NetBoxPrefix CRD name (controller resolves to NetBox ID from status)
        let prefix_id_str = &pool.spec.netbox_prefix_ref.id;
        let prefix_id = if let Ok(id) = prefix_id_str.parse::<u64>() {
            // Direct numeric ID
            id
        } else {
            // Treat as NetBoxPrefix CRD name - resolve to NetBox ID
            let prefix_crd_name = prefix_id_str;
            let prefix_crd_namespace = pool.metadata.namespace.as_deref().unwrap_or("default");
            
            debug!("Resolving NetBoxPrefix CRD {}/{} to NetBox ID for IPClaim", prefix_crd_namespace, prefix_crd_name);
            
            let prefix_crd = match self.netbox_prefix_api.get(prefix_crd_name).await {
                Ok(crd) => crd,
                Err(e) => {
                    let error_msg = format!("NetBoxPrefix CRD {}/{} not found: {}", prefix_crd_namespace, prefix_crd_name, e);
                    error!("{}", error_msg);
                    update_status_error(&self.ip_claim_api, name, namespace, error_msg.clone(), claim.status.as_ref()).await;
                    self.increment_error(&resource_key);
                    return Err(ControllerError::PrefixNotFound(error_msg));
                }
            };
            
            // Get NetBox ID from NetBoxPrefix status
            match prefix_crd.status
                .as_ref()
                .and_then(|s| s.netbox_id)
            {
                Some(netbox_id) => {
                    info!("Resolved NetBoxPrefix CRD {}/{} to NetBox ID {} for IPClaim", prefix_crd_namespace, prefix_crd_name, netbox_id);
                    netbox_id
                }
                None => {
                    let error_msg = format!("NetBoxPrefix {}/{} has not been created in NetBox yet (no netbox_id in status)", prefix_crd_namespace, prefix_crd_name);
                    error!("{}", error_msg);
                    update_status_error(&self.ip_claim_api, name, namespace, error_msg.clone(), claim.status.as_ref()).await;
                    self.increment_error(&resource_key);
                    return Err(ControllerError::PrefixNotFound(error_msg));
                }
            }
        };
        
        // Verify prefix exists in NetBox
        let _prefix = match self.netbox_client.get_prefix(prefix_id).await {
            Ok(p) => p,
            Err(e) => {
                let error_msg = format!("Prefix {} not found in NetBox: {}", prefix_id, e);
                error!("{}", error_msg);
                update_status_error(&self.ip_claim_api, name, namespace, error_msg.clone(), claim.status.as_ref()).await;
                return Err(ControllerError::PrefixNotFound(error_msg));
            }
        };
        
        // Resolve tag names to tag IDs or dictionaries for NetBox API
        // NetBox expects tags as either numeric IDs or dictionaries with "name" or "slug" keys
        let tag_refs = {
            let mut refs = Vec::new();
            let tag_names = vec!["managed-by-dcops", "owner-ip-claim-controller"];
            
            for tag_name in tag_names {
                match self.netbox_client.query_tags(&[("name", tag_name)], false).await {
                    Ok(tags) => {
                        if let Some(tag) = tags.first() {
                            // Use numeric ID (tags always have IDs in NetBox)
                            refs.push(serde_json::json!(tag.id));
                            debug!("Resolved tag '{}' to ID {}", tag_name, tag.id);
                        } else {
                            // Tag doesn't exist, try using name as slug in dictionary format
                            warn!("Tag '{}' not found, using as slug in dictionary format", tag_name);
                            refs.push(serde_json::json!({"slug": tag_name}));
                        }
                    }
                    Err(e) => {
                        warn!("Failed to query tag '{}': {}, using as slug in dictionary format", tag_name, e);
                        refs.push(serde_json::json!({"slug": tag_name}));
                    }
                }
            }
            Some(refs)
        };
        
        // Allocate IP from NetBox
        let allocation_request = AllocateIPRequest {
            address: claim.spec.preferred_ip.clone(),
            description: Some(format!("IPClaim: {}/{}", namespace, name)),
            status: Some(IPAddressStatus::Active),
            role: None,
            dns_name: None,
            tags: tag_refs,
        };
        
        let allocated_ip = match self.netbox_client.allocate_ip(prefix_id, Some(allocation_request)).await {
            Ok(ip) => ip,
            Err(e) => {
                // Check if error is "already exists" - if so, try to find it (idempotency)
                let error_str = format!("{}", e);
                if error_str.contains("already exists") || error_str.contains("duplicate") || error_str.contains("unique constraint") {
                    warn!("IP allocation failed with 'already exists', attempting to retrieve existing IP (idempotency)");
                    
                    // Try to find the existing IP address
                    if let Some(preferred_ip) = &claim.spec.preferred_ip {
                        // Query by the preferred IP address
                        match self.netbox_client.query_ip_addresses(
                            &[("address", preferred_ip)],
                            false,
                        ).await {
                            Ok(ips) => {
                                if let Some(existing_ip) = ips.first() {
                                    info!("Found existing IP address {} in NetBox (ID: {}) after allocation conflict", existing_ip.address, existing_ip.id);
                                    // Use the existing IP - only update status if it changed
                                    use crate::reconcile_helpers::ipclaim_status_needs_update;
                                    let needs_status_update = ipclaim_status_needs_update(
                                        claim.status.as_ref(),
                                        Some(&existing_ip.address),
                                        "Allocated",
                                        Some(&existing_ip.url),
                                        None,
                                    );
                                    
                                    if needs_status_update {
                                        let status_patch = Self::create_ipclaim_status_patch(
                                            Some(existing_ip.address.clone()),
                                            AllocationState::Allocated,
                                            Some(existing_ip.url.clone()),
                                            None,
                                        );
                                        let pp = kube::api::PatchParams::default();
                                        if let Err(update_err) = self.ip_claim_api
                                            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                                            .await
                                        {
                                            error!("Failed to update IPClaim status with existing IP: {}", update_err);
                                            return Err(ControllerError::AllocationFailed(format!("Found existing IP but failed to update status: {}", update_err)));
                                        }
                                        debug!("Updated IPClaim {}/{} status with existing IP {}", namespace, name, existing_ip.address);
                                    } else {
                                        debug!("IPClaim {}/{} already has correct status (IP: {}), skipping update", namespace, name, existing_ip.address);
                                    }
                                    self.reset_error(&resource_key);
                                    return Ok(());
                                }
                            }
                            Err(query_err) => {
                                warn!("Failed to query for existing IP address: {}", query_err);
                            }
                        }
                    }
                    
                    // If we couldn't find it by preferred IP, try querying all IPs in the prefix
                    // This is a fallback - less efficient but more reliable
                    warn!("Could not find IP by preferred address, querying all IPs in prefix {} (idempotency)", prefix_id);
                    match self.netbox_client.query_ip_addresses(
                        &[("prefix_id", &prefix_id.to_string())],
                        true, // fetch_all
                    ).await {
                        Ok(all_ips) => {
                            // Try to match by preferred IP if we have one
                            if let Some(preferred_ip) = &claim.spec.preferred_ip {
                                if let Some(found) = all_ips.iter().find(|ip| ip.address == *preferred_ip) {
                                    info!("Found existing IP address {} in NetBox (ID: {}) after querying prefix", found.address, found.id);
                                    // Use the existing IP - only update status if it changed
                                    use crate::reconcile_helpers::ipclaim_status_needs_update;
                                    let needs_status_update = ipclaim_status_needs_update(
                                        claim.status.as_ref(),
                                        Some(&found.address),
                                        "Allocated",
                                        Some(&found.url),
                                        None,
                                    );
                                    
                                    if needs_status_update {
                                        let status_patch = Self::create_ipclaim_status_patch(
                                            Some(found.address.clone()),
                                            AllocationState::Allocated,
                                            Some(found.url.clone()),
                                            None,
                                        );
                                        let pp = kube::api::PatchParams::default();
                                        if let Err(update_err) = self.ip_claim_api
                                            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                                            .await
                                        {
                                            error!("Failed to update IPClaim status with existing IP: {}", update_err);
                                            return Err(ControllerError::AllocationFailed(format!("Found existing IP but failed to update status: {}", update_err)));
                                        }
                                        debug!("Updated IPClaim {}/{} status with existing IP {}", namespace, name, found.address);
                                    } else {
                                        debug!("IPClaim {}/{} already has correct status (IP: {}), skipping update", namespace, name, found.address);
                                    }
                                    self.reset_error(&resource_key);
                                    return Ok(());
                                }
                            }
                        }
                        Err(query_err) => {
                            warn!("Failed to query all IPs in prefix: {}", query_err);
                        }
                    }
                }
                
                let error_msg = format!("Failed to allocate IP from prefix {}: {}", prefix_id, e);
                error!("{}", error_msg);
                update_status_error(&self.ip_claim_api, name, namespace, error_msg.clone(), claim.status.as_ref()).await;
                return Err(ControllerError::AllocationFailed(error_msg));
            }
        };
        
        info!("Allocated IP {} for IPClaim {}/{}", allocated_ip.address, namespace, name);
        
        // Update IPClaim status with success - only if status changed
        // IPClaim status must show the allocated IP address
        use crate::reconcile_helpers::ipclaim_status_needs_update;
        let needs_status_update = ipclaim_status_needs_update(
            claim.status.as_ref(),
            Some(&allocated_ip.address),
            "Allocated",
            Some(&allocated_ip.url),
            None,
        );
        
        if needs_status_update {
            let status_patch = Self::create_ipclaim_status_patch(
                Some(allocated_ip.address.clone()),
                AllocationState::Allocated,
                Some(allocated_ip.url.clone()),
                None,
            );
            
            // Patch the status using kube-rs status subresource API
            use kube::api::PatchParams;
            let pp = PatchParams::default();
            match self.ip_claim_api
                .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                .await
            {
                Ok(_) => {
                    debug!("Updated IPClaim {}/{} status with IP {}", namespace, name, allocated_ip.address);
                    // Reset error count on success
                    let resource_key = format!("{}/{}", namespace, name);
                    self.reset_error(&resource_key);
                    Ok(())
                }
                Err(e) => {
                    let error_msg = format!("Failed to update IPClaim status: {}", e);
                    error!("{}", error_msg);
                    update_status_error(&self.ip_claim_api, name, namespace, error_msg.clone(), claim.status.as_ref()).await;
                    Err(ControllerError::Kube(e.into()))
                }
            }
        } else {
            debug!("IPClaim {}/{} already has correct status (IP: {}), skipping update", namespace, name, allocated_ip.address);
            // Reset error count on success (even if status unchanged)
            let resource_key = format!("{}/{}", namespace, name);
            self.reset_error(&resource_key);
            Ok(())
        }
    }
}
