//! IPPool reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug};
use crds::{IPPool, IPPoolStatus};

impl Reconciler {
    pub async fn reconcile_ip_pool(&self, pool: &IPPool) -> Result<(), ControllerError> {
        let name = pool.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("IPPool missing name".to_string()))?;
        let namespace = pool.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling IPPool {}/{}", namespace, name);
        
        // Resolve NetBox prefix ID from NetBoxPrefix CRD reference
        // The spec.netbox_prefix_ref is a NetBoxResourceReference pointing to a NetBoxPrefix CRD
        let prefix_ref = &pool.spec.netbox_prefix_ref;
        
        // Validate that the reference is to a NetBoxPrefix CRD
        if prefix_ref.kind != "NetBoxPrefix" {
            let error_msg = format!(
                "Invalid kind '{}' for netbox_prefix_ref in IPPool {}, expected 'NetBoxPrefix'",
                prefix_ref.kind, name
            );
            error!("{}", error_msg);
            return Err(ControllerError::InvalidConfig(error_msg));
        }
        
        // Resolve the NetBoxPrefix CRD to get the NetBox prefix ID
        let prefix_crd_name = &prefix_ref.name;
        let prefix_crd_namespace = prefix_ref.namespace.as_deref()
            .unwrap_or_else(|| pool.metadata.namespace.as_deref().unwrap_or("default"));
        
        debug!("Resolving NetBoxPrefix CRD {}/{} to NetBox ID for IPPool {}", prefix_crd_namespace, prefix_crd_name, name);
        
        let prefix_crd = match self.netbox_prefix_api.get(prefix_crd_name).await {
            Ok(crd) => crd,
            Err(e) => {
                let error_msg = format!(
                    "NetBoxPrefix CRD {}/{} not found for IPPool {}: {}",
                    prefix_crd_namespace, prefix_crd_name, name, e
                );
                error!("{}", error_msg);
                return Err(ControllerError::PrefixNotFound(error_msg));
            }
        };
        
        // Get NetBox ID from NetBoxPrefix status
        let prefix_id = prefix_crd.status
            .as_ref()
            .and_then(|s| s.netbox_id)
            .ok_or_else(|| {
                let error_msg = format!(
                    "NetBoxPrefix {}/{} has not been created in NetBox yet (no netbox_id in status). Ensure the NetBoxPrefix CRD is reconciled first.",
                    prefix_crd_namespace, prefix_crd_name
                );
                error!("{}", error_msg);
                ControllerError::PrefixNotFound(error_msg)
            })?;
        
        // Get NetBox prefix URL from NetBoxPrefix status
        let prefix_url = prefix_crd.status
            .as_ref()
            .and_then(|s| s.netbox_url.clone());
        
        info!("Resolved NetBoxPrefix CRD {}/{} to NetBox ID {} for IPPool {}", 
            prefix_crd_namespace, prefix_crd_name, prefix_id, name);
        
        // Get prefix from NetBox
        let prefix = match self.netbox_client.get_prefix(prefix_id).await {
            Ok(p) => p,
            Err(netbox_client::NetBoxError::NotFound(_)) => {
                // Prefix not found - this indicates drift (prefix was deleted in NetBox)
                let error_msg = format!(
                    "Prefix ID {} (resolved from NetBoxPrefix CRD {}/{}) not found in NetBox. This may indicate the prefix was deleted. Ensure the NetBoxPrefix CRD is reconciled and the prefix exists in NetBox.",
                    prefix_id, prefix_crd_namespace, prefix_crd_name
                );
                error!("{}", error_msg);
                return Err(ControllerError::PrefixNotFound(error_msg));
            }
            Err(e) => {
                let error_msg = format!("Failed to get prefix {} from NetBox: {}", prefix_id, e);
                error!("{}", error_msg);
                return Err(ControllerError::PrefixNotFound(error_msg));
            }
        };
        
        // Get available IPs
        let available_ips = match self.netbox_client.get_available_ips(prefix_id, None).await {
            Ok(ips) => ips,
            Err(e) => {
                let error_msg = format!("Failed to get available IPs: {}", e);
                error!("{}", error_msg);
                return Err(ControllerError::NetBox(e));
            }
        };
        
        // Query allocated IPs from this prefix
        let allocated_ips = match self.netbox_client.query_ip_addresses(
            &[("prefix", &prefix.prefix)],
            true, // fetch all pages
        ).await {
            Ok(ips) => ips,
            Err(e) => {
                let error_msg = format!("Failed to query allocated IPs: {}", e);
                error!("{}", error_msg);
                return Err(ControllerError::NetBox(e));
            }
        };
        
        // Calculate pool statistics
        // Note: This is approximate - NetBox doesn't provide exact counts
        let total_ips = allocated_ips.len() + available_ips.len();
        let allocated_count = allocated_ips.len() as u32;
        let available_count = available_ips.len() as u32;
        
        // Check if status needs update (only update if netbox_prefix_id or statistics changed)
        let current_status = pool.status.as_ref();
        let needs_update = match current_status {
            Some(status) => {
                // Check if netbox_prefix_id changed
                let id_changed = status.netbox_prefix_id != Some(prefix_id);
                // Check if URL changed
                let url_changed = status.netbox_prefix_url != prefix_url;
                // Check if statistics changed
                let stats_changed = status.total_ips != total_ips as u32
                    || status.allocated_ips != allocated_count
                    || status.available_ips != available_count;
                
                id_changed || url_changed || stats_changed
            }
            None => true, // No status, need to create it
        };
        
        if !needs_update {
            debug!("IPPool {}/{} status is up-to-date, skipping update", namespace, name);
            return Ok(());
        }
        
        // Update IPPool status with resolved NetBox prefix ID and statistics
        // NOTE: last_reconciled removed to prevent reconciliation loops
        // The timestamp changes on every reconciliation, causing status updates
        // which trigger watch events, potentially causing loops
        let new_status = IPPoolStatus {
            netbox_prefix_id: Some(prefix_id),
            netbox_prefix_url: prefix_url,
            total_ips: total_ips as u32,
            allocated_ips: allocated_count,
            available_ips: available_count,
            last_reconciled: None,  // Removed to prevent reconciliation loops
        };
        
        // Patch the status using kube-rs status subresource API
        use kube::api::PatchParams;
        use serde_json::json;
        
        let status_patch = json!({
            "status": new_status
        });
        
        let pp = PatchParams::default();
        match self.ip_pool_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
            .await
        {
            Ok(_) => {
                info!("Updated IPPool {}/{} status: NetBox prefix ID {}, {} total, {} allocated, {} available",
                    namespace, name, prefix_id, total_ips, allocated_count, available_count);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update IPPool status: {}", e);
                error!("{}", error_msg);
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}
