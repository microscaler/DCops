//! NetBoxPrefix reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers;
use kube::Api;
use tracing::{info, error, debug, warn};
use crds::{NetBoxPrefix, NetBoxPrefixStatus, PrefixState};
use netbox_client;

impl Reconciler {
    /// Check if prefix needs updating by comparing spec with existing NetBox resource
    /// 
    /// Note: NetBox Prefix model doesn't have a `site` field in the response,
    /// but site can be set via API. We can't compare site from existing resource,
    /// so we'll update if other fields changed. Site updates will be handled by
    /// always including site_id in update calls when provided.
    fn prefix_needs_update(
        spec: &crds::NetBoxPrefixSpec,
        existing: &netbox_client::Prefix,
        desired_tenant_id: Option<u64>,
        _desired_site_id: Option<u64>, // Prefix model doesn't have site field, can't compare
        desired_vlan_id: Option<u32>,
        desired_role_id: Option<u64>,
        desired_status: &str,
    ) -> bool {
        // Compare tenant
        let existing_tenant_id = existing.tenant.as_ref().map(|t| t.id);
        if desired_tenant_id != existing_tenant_id {
            debug!("Prefix tenant changed: {:?} -> {:?}", existing_tenant_id, desired_tenant_id);
            return true;
        }
        
        // Compare vlan
        let existing_vlan_id = existing.vlan.as_ref().map(|v| v.id as u32);
        if desired_vlan_id != existing_vlan_id {
            debug!("Prefix vlan changed: {:?} -> {:?}", existing_vlan_id, desired_vlan_id);
            return true;
        }
        
        // Compare role
        let existing_role_id = existing.role.as_ref().map(|r| r.id);
        if desired_role_id != existing_role_id {
            debug!("Prefix role changed: {:?} -> {:?}", existing_role_id, desired_role_id);
            return true;
        }
        
        // Compare description - Prefix model has description as String, not Option<String>
        let spec_desc = spec.description.as_deref().unwrap_or("");
        if spec_desc != existing.description {
            debug!("Prefix description changed: '{}' -> '{}'", existing.description, spec_desc);
            return true;
        }
        
        // Compare status
        let existing_status = match existing.status {
            netbox_client::PrefixStatus::Active => "active",
            netbox_client::PrefixStatus::Reserved => "reserved",
            netbox_client::PrefixStatus::Deprecated => "deprecated",
            netbox_client::PrefixStatus::Container => "container",
        };
        if desired_status != existing_status {
            debug!("Prefix status changed: '{}' -> '{}'", existing_status, desired_status);
            return true;
        }
        
        false // No changes needed
    }

    pub async fn reconcile_netbox_prefix(&self, prefix_crd: &NetBoxPrefix) -> Result<(), ControllerError> {
        let name = prefix_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxPrefix missing name".to_string()))?;
        let namespace = prefix_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxPrefix {}/{}", namespace, name);
        
        // Helper function to update status with error
        async fn update_status_error(
            api: &Api<NetBoxPrefix>,
            name: &str,
            namespace: &str,
            error_msg: String,
            current_status: Option<&NetBoxPrefixStatus>,
        ) {
            // Check if error is already set to avoid unnecessary updates
            if let Some(status) = current_status {
                if status.state == PrefixState::Failed && status.error.as_ref() == Some(&error_msg) {
                    debug!("NetBoxPrefix {}/{} already has this error in status, skipping update", namespace, name);
                    return;
                }
            }
            
            // Update status with error (use lowercase state to match CRD validation schema)
            let status_patch = Reconciler::create_prefix_status_patch(
                0, // No netbox_id on error
                String::new(), // No URL on error
                PrefixState::Failed,
                Some(error_msg.clone()),
            );
            
            let pp = kube::api::PatchParams::default();
            if let Err(e) = api
                .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                .await
            {
                error!("Failed to update NetBoxPrefix {}/{} error status: {}", namespace, name, e);
            } else {
                info!("Updated NetBoxPrefix {}/{} status with error", namespace, name);
            }
        }
        
        // Convert PrefixStatus enum to string for NetBox API
        let status_str = match prefix_crd.spec.status {
            crds::PrefixStatus::Active => "active",
            crds::PrefixStatus::Reserved => "reserved",
            crds::PrefixStatus::Deprecated => "deprecated",
            crds::PrefixStatus::Container => "container",
        };
        
        // Resolve all references first (needed for both update detection and creation)
        // Resolve Site reference if provided
        let site_id = if let Some(site_ref) = &prefix_crd.spec.site {
            if site_ref.kind != "NetBoxSite" {
                warn!("Invalid kind '{}' for site reference in prefix {}, expected 'NetBoxSite'", site_ref.kind, name);
                None
            } else {
                match self.netbox_site_api.get(&site_ref.name).await {
                    Ok(site_crd) => {
                        site_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                    }
                    Err(_) => {
                        warn!("Site CRD '{}' not found for prefix {}, skipping site reference", site_ref.name, name);
                        None
                    }
                }
            }
        } else {
            None
        };
        
        // Resolve VLAN reference if provided
        let vlan_id = if let Some(vlan_ref) = &prefix_crd.spec.vlan {
            if vlan_ref.kind != "NetBoxVLAN" {
                warn!("Invalid kind '{}' for VLAN reference in prefix {}, expected 'NetBoxVLAN'", vlan_ref.kind, name);
                None
            } else {
                match self.netbox_vlan_api.get(&vlan_ref.name).await {
                    Ok(vlan_crd) => {
                        vlan_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                            .map(|id| id as u32)
                    }
                    Err(_) => {
                        warn!("VLAN CRD '{}' not found for prefix {}, skipping VLAN reference", vlan_ref.name, name);
                        None
                    }
                }
            }
        } else {
            None
        };
        
        // Resolve Tenant reference if provided
        let tenant_id = if let Some(tenant_ref) = &prefix_crd.spec.tenant {
            if tenant_ref.kind != "NetBoxTenant" {
                warn!("Invalid kind '{}' for tenant reference in prefix {}, expected 'NetBoxTenant'", tenant_ref.kind, name);
                None
            } else {
                match self.netbox_tenant_api.get(&tenant_ref.name).await {
                    Ok(tenant_crd) => {
                        tenant_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                    }
                    Err(_) => {
                        warn!("Tenant CRD '{}' not found for prefix {}, skipping tenant reference", tenant_ref.name, name);
                        None
                    }
                }
            }
        } else {
            None
        };
        
        // Resolve Role reference if provided
        let role_id = if let Some(role_ref) = &prefix_crd.spec.role {
            if role_ref.kind != "NetBoxRole" {
                warn!("Invalid kind '{}' for role reference in prefix {}, expected 'NetBoxRole'", role_ref.kind, name);
                None
            } else {
                match self.netbox_role_api.get(&role_ref.name).await {
                    Ok(role_crd) => {
                        role_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                    }
                    Err(_) => {
                        warn!("Role CRD '{}' not found for prefix {}, skipping role reference", role_ref.name, name);
                        None
                    }
                }
            }
        } else {
            None
        };
        
        // Check if already created - use helper for drift detection and updates
        let netbox_prefix = if let Some(status) = &prefix_crd.status {
            // Skip if it's a permanent auth error
            if status.state == PrefixState::Failed {
                if let Some(error) = &status.error {
                    if error.contains("Invalid token") || error.contains("403 Forbidden") {
                        debug!("NetBoxPrefix {}/{} already marked as failed with authentication error, skipping reconciliation", namespace, name);
                        return Ok(());
                    }
                }
            }
            
            if status.state == PrefixState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    // Use helper function for drift detection, diffing, and updating
                    match reconcile_helpers::check_and_update_existing(
                        &self.netbox_client,
                        netbox_id,
                        &format!("NetBoxPrefix {}/{}", namespace, name),
                        self.netbox_client.get_prefix(netbox_id),
                        |existing| Self::prefix_needs_update(
                            &prefix_crd.spec,
                            existing,
                            tenant_id,
                            site_id,
                            vlan_id,
                            role_id,
                            &status_str,
                        ),
                        self.netbox_client.update_prefix(
                            netbox_id,
                            None, // Don't change prefix CIDR
                            prefix_crd.spec.description.clone(),
                            Some(status_str),
                            None, // role - needs role_id but update_prefix expects Option<String>, not Option<u64>
                            tenant_id,
                            site_id, // Include site if resolved
                            vlan_id, // Include vlan if resolved
                            None, // tags - omit for now
                        ),
                    ).await {
                        Ok(Some(resource)) => {
                            // Resource exists and is up-to-date (or was updated)
                            Some(resource)
                        }
                        Ok(None) => {
                            // Drift detected - resource was deleted, clear status and recreate
                            warn!("NetBoxPrefix {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
                            let status_patch = Self::create_prefix_status_patch(
                                0, // Clear netbox_id
                                String::new(), // Clear URL
                                PrefixState::Pending,
                                Some("Resource was deleted in NetBox, will recreate".to_string()),
                            );
                            let pp = kube::api::PatchParams::default();
                            if let Err(e) = self.netbox_prefix_api
                                .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                                .await
                            {
                                warn!("Failed to clear NetBoxPrefix status after drift detection: {}", e);
                            }
                            // Fall through to creation
                            None
                        }
                        Err(e) => {
                            // Error during drift detection/update - return to retry
                            return Err(e);
                        }
                    }
                } else {
                    None // No netbox_id, need to create
                }
            } else {
                // Check if resource exists even if status is Failed (idempotency)
                if status.state == PrefixState::Failed && status.netbox_id.is_some() {
                    if let Some(netbox_id) = status.netbox_id {
                        info!("NetBoxPrefix {}/{} has Failed status, checking if resource exists in NetBox for idempotency", namespace, name);
                        // Try to get the resource - if it exists, we'll update status to Created
                        match self.netbox_client.get_prefix(netbox_id).await {
                            Ok(existing) => {
                                info!("NetBoxPrefix {}/{} exists in NetBox (ID: {}), updating status from Failed to Created", namespace, name, netbox_id);
                                Some(existing)
                            }
                            Err(_) => None // Resource doesn't exist, need to create
                        }
                    } else {
                        None
                    }
                } else {
                    None // Not in Created state, need to create
                }
            }
        } else {
            None // No status, need to create
        };
        
        // Handle existing prefix (from helper) or create new
        let netbox_prefix = match netbox_prefix {
            Some(prefix) => {
                // Resource exists and is up-to-date - only update status if it changed
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    prefix_crd.status.as_ref(),
                    prefix.id,
                    &prefix.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_prefix_status_patch(
                        prefix.id,
                        prefix.url.clone(),
                        PrefixState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_prefix_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxPrefix {}/{} status: NetBox ID {}", namespace, name, prefix.id);
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update NetBoxPrefix status: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxPrefix {}/{} already has correct status (ID: {}), skipping update", namespace, name, prefix.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create prefix - try to find existing by prefix CIDR (idempotency fallback)
                // Convert PrefixStatus enum to string for NetBox API
                let status_str = match prefix_crd.spec.status {
                    crds::PrefixStatus::Active => "active",
                    crds::PrefixStatus::Reserved => "reserved",
                    crds::PrefixStatus::Deprecated => "deprecated",
                    crds::PrefixStatus::Container => "container",
                };
                
                // Resolve VLAN reference if provided
                let vlan_id = if let Some(vlan_ref) = &prefix_crd.spec.vlan {
                    // Validate kind
                    if vlan_ref.kind != "NetBoxVLAN" {
                        warn!("Invalid kind '{}' for VLAN reference in prefix {}, expected 'NetBoxVLAN'", vlan_ref.kind, name);
                        None
                    } else {
                        match self.netbox_vlan_api.get(&vlan_ref.name).await {
                            Ok(vlan_crd) => {
                                vlan_crd.status
                                    .as_ref()
                                    .and_then(|s| s.netbox_id)
                                    .map(|id| id as u32)
                            }
                            Err(_) => {
                                warn!("VLAN CRD '{}' not found for prefix {}, skipping VLAN reference", vlan_ref.name, name);
                                None
                            }
                        }
                    }
                } else {
                    None
                };
                
                // Resolve Site reference if provided - need ID for NetBox API
                let site_id = if let Some(site_ref) = &prefix_crd.spec.site {
                    // Validate kind
                    if site_ref.kind != "NetBoxSite" {
                        warn!("Invalid kind '{}' for site reference in prefix {}, expected 'NetBoxSite'", site_ref.kind, name);
                        None
                    } else {
                        // Resolve to NetBox ID
                        match self.netbox_site_api.get(&site_ref.name).await {
                            Ok(site_crd) => {
                                site_crd.status
                                    .as_ref()
                                    .and_then(|s| s.netbox_id)
                            }
                            Err(_) => {
                                warn!("Site CRD '{}' not found for prefix {}, skipping site reference", site_ref.name, name);
                                None
                            }
                        }
                    }
                } else {
                    None
                };
                
                // Resolve Tenant reference if provided - need ID for NetBox API
                let tenant_id = if let Some(tenant_ref) = &prefix_crd.spec.tenant {
                    // Validate kind
                    if tenant_ref.kind != "NetBoxTenant" {
                        warn!("Invalid kind '{}' for tenant reference in prefix {}, expected 'NetBoxTenant'", tenant_ref.kind, name);
                        None
                    } else {
                        // Resolve to NetBox ID
                        match self.netbox_tenant_api.get(&tenant_ref.name).await {
                            Ok(tenant_crd) => {
                                tenant_crd.status
                                    .as_ref()
                                    .and_then(|s| s.netbox_id)
                            }
                            Err(_) => {
                                warn!("Tenant CRD '{}' not found for prefix {}, skipping tenant reference", tenant_ref.name, name);
                                None
                            }
                        }
                    }
                } else {
                    None
                };
                
                // Resolve Role reference if provided - need ID for NetBox API
                let role_id = if let Some(role_ref) = &prefix_crd.spec.role {
                    // Validate kind
                    if role_ref.kind != "NetBoxRole" {
                        warn!("Invalid kind '{}' for role reference in prefix {}, expected 'NetBoxRole'", role_ref.kind, name);
                        None
                    } else {
                        // Resolve to NetBox ID
                        match self.netbox_role_api.get(&role_ref.name).await {
                            Ok(role_crd) => {
                                role_crd.status
                                    .as_ref()
                                    .and_then(|s| s.netbox_id)
                            }
                            Err(_) => {
                                warn!("Role CRD '{}' not found for prefix {}, skipping role reference", role_ref.name, name);
                                None
                            }
                        }
                    }
                } else {
                    None
                };
        
                // Try to find existing prefix by querying NetBox (idempotency fallback)
                let existing_prefix = match self.netbox_client.query_prefixes(
                    &[("prefix", &prefix_crd.spec.prefix)],
                    false, // Just check first page
                ).await {
                    Ok(prefixes) => {
                        prefixes.iter().find(|p| p.prefix == prefix_crd.spec.prefix).cloned()
                    }
                    Err(e) => {
                        // Query failed - try alternative methods to find existing prefix
                        warn!("Failed to query prefixes in NetBox: {}, trying alternative methods", e);
                        
                        // Try to get all prefixes and search (if fetch_all works)
                        match self.netbox_client.query_prefixes(
                            &[],
                            true, // fetch_all
                        ).await {
                            Ok(all_prefixes) => {
                                all_prefixes.iter().find(|p| p.prefix == prefix_crd.spec.prefix).cloned()
                            }
                            Err(_) => {
                                warn!("Could not query prefixes, will try to create (resource may already exist)");
                                None
                            }
                        }
                    }
                };
                
                let netbox_prefix = if let Some(existing) = existing_prefix {
                    // Prefix exists in NetBox - this is the idempotent case
                    info!("Prefix {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", prefix_crd.spec.prefix, existing.id);
                    
                    // Update prefix if needed (tenant, site, vlan, description, status)
                    // Note: Omitting role and tags for now (requires numeric IDs or string slugs)
                    match self.netbox_client.update_prefix(
                        existing.id,
                        None, // Don't change prefix CIDR
                        prefix_crd.spec.description.clone(),
                        Some(status_str),
                        None, // role - omit for now (requires numeric ID or string slug)
                        tenant_id, // Include tenant if resolved
                        site_id, // Include site if resolved
                        vlan_id, // Include vlan if resolved
                        None, // tags - omit for now (requires numeric IDs or tag slugs)
                    ).await {
                        Ok(updated) => {
                            info!("Updated prefix {} in NetBox (ID: {})", updated.prefix, updated.id);
                            updated
                        }
                        Err(e) => {
                            // If update fails, use the existing prefix we already have
                            // This is still a success case - resource exists, we just couldn't update it
                            warn!("Failed to update prefix in NetBox: {}, but resource exists (ID: {}), using existing data", e, existing.id);
                            existing
                        }
                    }
                } else {
                    // Prefix doesn't exist, create it
                    info!("Creating prefix {} in NetBox", prefix_crd.spec.prefix);
                    
                    // NetBox API requires site and role to be numeric IDs
                    // Tags must be numeric IDs or tag slugs
                    // TODO: Add support for resolving tag names to tag slugs
                    match self.netbox_client.create_prefix(
                        &prefix_crd.spec.prefix,
                        prefix_crd.spec.description.clone(),
                        site_id,
                        vlan_id,
                        Some(status_str),
                        role_id,
                        tenant_id, // Include tenant if resolved
                        None, // tags - omit for now (requires numeric IDs or tag slugs)
                    ).await {
                        Ok(created) => {
                            info!("Created prefix {} in NetBox (ID: {})", created.prefix, created.id);
                            created
                        }
                        Err(e) => {
                            // Check if error is "already exists" - if so, try to find it (idempotency)
                            let error_str = format!("{}", e);
                            if error_str.contains("already exists") || error_str.contains("duplicate") || error_str.contains("unique constraint") {
                                warn!("Prefix {} already exists in NetBox, attempting to retrieve it (idempotency)", prefix_crd.spec.prefix);
                                
                                // Try to find the existing prefix using fetch_all
                                match self.netbox_client.query_prefixes(
                                    &[],
                                    true, // fetch_all
                                ).await {
                                    Ok(all_prefixes) => {
                                        if let Some(found) = all_prefixes.iter().find(|p| p.prefix == prefix_crd.spec.prefix) {
                                            info!("Found existing prefix {} in NetBox (ID: {}) after create conflict", found.prefix, found.id);
                                            found.clone()
                                        } else {
                                            // Prefix exists but we can't find it - this is unusual
                                            let error_msg = format!("Prefix {} already exists in NetBox but could not retrieve it: {}", prefix_crd.spec.prefix, e);
                                            error!("{}", error_msg);
                                            update_status_error(&self.netbox_prefix_api, name, namespace, error_msg.clone(), prefix_crd.status.as_ref()).await;
                                            return Err(ControllerError::NetBox(e));
                                        }
                                    }
                                    Err(query_err) => {
                                        // Couldn't query - this is a real error
                                        let error_msg = format!("Failed to create prefix in NetBox (may already exist, but could not verify): {} (query error: {})", e, query_err);
                                        error!("{}", error_msg);
                                        update_status_error(&self.netbox_prefix_api, name, namespace, error_msg.clone(), prefix_crd.status.as_ref()).await;
                                        return Err(ControllerError::NetBox(e));
                                    }
                                }
                            } else {
                                // Real creation error
                                let error_msg = format!("Failed to create prefix in NetBox: {}", e);
                                error!("{}", error_msg);
                                update_status_error(&self.netbox_prefix_api, name, namespace, error_msg.clone(), prefix_crd.status.as_ref()).await;
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    }
                };
                
                netbox_prefix
            }
        };
        
        // Update NetBoxPrefix status with success
        // Update status (use lowercase state to match CRD validation schema)
        let status_patch = Self::create_prefix_status_patch(
            netbox_prefix.id,
            netbox_prefix.url.clone(),
            PrefixState::Created,
            None,
        );
        
        // Patch the status using kube-rs status subresource API
        use kube::api::PatchParams;
        let pp = PatchParams::default();
        match self.netbox_prefix_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxPrefix {}/{} status (NetBox ID: {})", namespace, name, netbox_prefix.id);
                // Reset error count on success
                let resource_key = format!("{}/{}", namespace, name);
                self.reset_error(&resource_key);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxPrefix status: {}", e);
                error!("{}", error_msg);
                update_status_error(&self.netbox_prefix_api, name, namespace, error_msg.clone(), prefix_crd.status.as_ref()).await;
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}
