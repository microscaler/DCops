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
        
        // Resolve NetBox prefix ID
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
            
            debug!("Resolving NetBoxPrefix CRD {}/{} to NetBox ID", prefix_crd_namespace, prefix_crd_name);
            
            let prefix_crd = match self.netbox_prefix_api.get(prefix_crd_name).await {
                Ok(crd) => crd,
                Err(e) => {
                    let error_msg = format!("NetBoxPrefix CRD {}/{} not found: {}", prefix_crd_namespace, prefix_crd_name, e);
                    error!("{}", error_msg);
                    return Err(ControllerError::PrefixNotFound(error_msg));
                }
            };
            
            // Get NetBox ID from NetBoxPrefix status
            let netbox_id = prefix_crd.status
                .as_ref()
                .and_then(|s| s.netbox_id)
                .ok_or_else(|| {
                    let error_msg = format!("NetBoxPrefix {}/{} has not been created in NetBox yet (no netbox_id in status)", prefix_crd_namespace, prefix_crd_name);
                    error!("{}", error_msg);
                    ControllerError::PrefixNotFound(error_msg)
                })?;
            
            info!("Resolved NetBoxPrefix CRD {}/{} to NetBox ID {}", prefix_crd_namespace, prefix_crd_name, netbox_id);
            netbox_id
        };
        
        // Get prefix from NetBox
        let prefix = match self.netbox_client.get_prefix(prefix_id).await {
            Ok(p) => p,
            Err(netbox_client::NetBoxError::NotFound(_)) => {
                // Prefix not found - provide helpful error message
                let error_msg = if prefix_id_str.parse::<u64>().is_ok() {
                    // Direct numeric ID not found
                    format!("Prefix ID {} not found in NetBox. Ensure the prefix exists or create a NetBoxPrefix CRD and reconcile it first.", prefix_id)
                } else {
                    // CRD name was resolved but prefix not found
                    format!("Prefix {} (resolved from NetBoxPrefix CRD {}) not found in NetBox. Ensure the NetBoxPrefix CRD has been reconciled and the prefix exists in NetBox.", prefix_id, prefix_id_str)
                };
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
        
        // Update IPPool status
        // NOTE: last_reconciled removed to prevent reconciliation loops
        // The timestamp changes on every reconciliation, causing status updates
        // which trigger watch events, potentially causing loops
        let new_status = IPPoolStatus {
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
                info!("Updated IPPool {}/{} status: {} total, {} allocated, {} available",
                    namespace, name, total_ips, allocated_count, available_count);
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
