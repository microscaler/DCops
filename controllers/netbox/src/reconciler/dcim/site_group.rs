//! NetBoxSiteGroup reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers;
use crate::kube_api_trait::KubeApiTrait;
use tracing::{info, error, debug, warn};
use crds::{NetBoxSiteGroup, NetBoxSiteGroupStatus, ResourceState};
use netbox_client::NetBoxClientTrait;

impl Reconciler {
    pub async fn reconcile_netbox_site_group(&self, site_group_crd: &NetBoxSiteGroup) -> Result<(), ControllerError> {
        let name = site_group_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxSiteGroup missing name".to_string()))?;
        let namespace = site_group_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxSiteGroup {}/{}", namespace, name);
        
        // Get client for shared resource (finds tenant from referencing Sites)
        let netbox_client = self.token_resolver
            .create_client_for_shared_resource(namespace, "NetBoxSiteGroup", name)
            .await
            .map_err(|e| ControllerError::TokenResolution(e))?;
        
        // Resolve parent site group ID if parent reference provided
        let parent_id = if let Some(parent_ref) = &site_group_crd.spec.parent {
            if parent_ref.kind != "NetBoxSiteGroup" {
                warn!("Invalid kind '{}' for parent reference in site group {}, expected 'NetBoxSiteGroup'", parent_ref.kind, name);
                None
            } else {
                match self.netbox_site_group_api.get(&parent_ref.name).await {
                    Ok(parent_crd) => {
                        parent_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                    }
                    Err(_) => {
                        warn!("Parent SiteGroup CRD '{}' not found for site group {}, skipping parent reference", parent_ref.name, name);
                        None
                    }
                }
            }
        } else {
            None
        };
        
        // Check if already created - use helper for drift detection
        let netbox_site_group = if let Some(status) = &site_group_crd.status {
            if status.state == ResourceState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    // Use simple helper function for drift detection (no update logic yet)
                    match reconcile_helpers::check_existing(
                        &netbox_client,
                        netbox_id,
                        &format!("NetBoxSiteGroup {}/{}", namespace, name),
                        netbox_client.get_site_group(netbox_id),
                    ).await {
                        Ok(Some(resource)) => {
                            // Resource exists and is up-to-date
                            Some(resource)
                        }
                        Ok(None) => {
                            // Drift detected - resource was deleted, clear status and recreate
                            warn!("NetBoxSiteGroup {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
                            let status_patch = Self::create_resource_status_patch(
                                0, // Clear netbox_id
                                String::new(), // Clear URL
                                ResourceState::Pending,
                                Some("Resource was deleted in NetBox, will recreate".to_string()),
                            );
                            let pp = kube::api::PatchParams::default();
                            if let Err(e) = self.netbox_site_group_api
                                .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                                .await
                            {
                                warn!("Failed to clear NetBoxSiteGroup status after drift detection: {}", e);
                            }
                            // Fall through to creation
                            None
                        }
                        Err(e) => {
                            // Error during drift detection - return to retry
                            return Err(e);
                        }
                    }
                } else {
                    None // No netbox_id, need to create
                }
            } else {
                None // Not in Created state, need to create
            }
        } else {
            None // No status, need to create
        };
        
        // Handle existing site group (from helper) or create new
        let netbox_site_group = match netbox_site_group {
            Some(site_group) => {
                // Resource exists and is up-to-date - only update status if it changed
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    site_group_crd.status.as_ref(),
                    site_group.id,
                    &site_group.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_resource_status_patch(
                        site_group.id,
                        site_group.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_site_group_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxSiteGroup {}/{} status: NetBox ID {}", namespace, name, site_group.id);
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update NetBoxSiteGroup status: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxSiteGroup {}/{} already has correct status (ID: {}), skipping update", namespace, name, site_group.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create site group - try to find existing by name (idempotency fallback)
                let existing_site_group = match netbox_client.get_site_group_by_name(&site_group_crd.spec.name).await {
                    Ok(Some(sg)) => {
                        info!("SiteGroup {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", site_group_crd.spec.name, sg.id);
                        Some(sg)
                    }
                    Ok(None) => None,
                    Err(e) => {
                        warn!("Failed to query site group by name: {}, will try to create", e);
                        None
                    }
                };
                
                if let Some(existing) = existing_site_group {
                    existing
                } else {
                    // Create site group
                    info!("Creating site group {} in NetBox", site_group_crd.spec.name);
                    match netbox_client.create_site_group(
                        &site_group_crd.spec.name,
                        site_group_crd.spec.slug.as_deref(),
                        parent_id,
                        site_group_crd.spec.description.clone(),
                        None, // comments - not in CRD spec yet
                    ).await {
                        Ok(created) => {
                            info!("Created site group {} in NetBox (ID: {})", created.name, created.id);
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create site group in NetBox: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                }
            }
        };
        
        // Update status
        let status_patch = Self::create_resource_status_patch(
            netbox_site_group.id,
            netbox_site_group.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_site_group_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxSiteGroup {}/{} status: NetBox ID {}", namespace, name, netbox_site_group.id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxSiteGroup status: {}", e);
                error!("{}", error_msg);
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}
