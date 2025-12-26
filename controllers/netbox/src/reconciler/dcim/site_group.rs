//! NetBoxSiteGroup reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers;
use tracing::{info, error, debug, warn};
use crds::{NetBoxSiteGroup, ResourceState};

impl Reconciler {
    pub async fn reconcile_netbox_site_group(&self, site_group_crd: &NetBoxSiteGroup) -> Result<(), ControllerError> {
        let name = site_group_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxSiteGroup missing name".to_string()))?;
        let namespace = site_group_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxSiteGroup {}/{}", namespace, name);
        
        // Check if already created - use helper for drift detection
        let netbox_site_group = if let Some(status) = &site_group_crd.status {
            if status.state == ResourceState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    // Use simple helper function for drift detection (no update logic)
                    match reconcile_helpers::check_existing(
                        self.netbox_client.as_ref(),
                        netbox_id,
                        &format!("NetBoxSiteGroup {}/{}", namespace, name),
                        self.netbox_client.get_site_group(netbox_id),
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
                                .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
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
                        .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
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
                // Resolve parent site group ID if parent reference provided
                let parent_id = if let Some(parent_ref) = &site_group_crd.spec.parent {
            if parent_ref.kind != "NetBoxSiteGroup" {
                warn!("Invalid kind '{}' for parent site group reference in site group {}, expected 'NetBoxSiteGroup'", parent_ref.kind, name);
                None
            } else {
                match self.netbox_site_group_api.get(&parent_ref.name).await {
                    Ok(parent_crd) => {
                        parent_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                    }
                    Err(_) => {
                        warn!("Parent site group CRD '{}' not found for site group {}", parent_ref.name, name);
                        None
                    }
                }
            }
                } else {
                    None
                };
                
                // Try to find existing site group by name
                let existing_site_group = match self.netbox_client.query_site_groups(
                    &[("name", &site_group_crd.spec.name)],
                    false,
                ).await {
                    Ok(site_groups) => site_groups.first().cloned(),
                    Err(_) => None
                };
                
                let netbox_site_group = if let Some(existing) = existing_site_group {
                    info!("Site group {} already exists in NetBox (ID: {})", site_group_crd.spec.name, existing.id);
                    existing
                } else {
                    let slug = site_group_crd.spec.slug.as_deref().map(|s| s.to_string())
                        .unwrap_or_else(|| site_group_crd.spec.name.to_lowercase().replace(' ', "-"));
                    match self.netbox_client.create_site_group(
                        &site_group_crd.spec.name,
                        &slug,
                        site_group_crd.spec.description.as_deref(),
                    ).await {
                        Ok(created) => {
                            info!("Created site group {} in NetBox (ID: {})", created.name, created.id);
                            created
                        }
                        Err(e) => {
                            // Check if error is "already exists" - if so, try to find it (idempotency)
                            let error_str = format!("{}", e);
                            if error_str.contains("already exists") || error_str.contains("duplicate") || error_str.contains("unique constraint") {
                                warn!("Site group {} already exists in NetBox, attempting to retrieve it (idempotency)", site_group_crd.spec.name);
                                
                                // Try to find the existing site group by name or slug
                                let found_site_group: Option<_> = match self.netbox_client.query_site_groups(
                                    &[("name", &site_group_crd.spec.name)],
                                    false,
                                ).await {
                                    Ok(site_groups) => {
                                        if let Some(found) = site_groups.first() {
                                            info!("Found existing site group {} in NetBox (ID: {}) after create conflict", found.name, found.id);
                                            Some(found.clone())
                                        } else {
                                            // Try by slug
                                            if let Some(slug) = &site_group_crd.spec.slug {
                                                match self.netbox_client.query_site_groups(
                                                    &[("slug", slug)],
                                                    false,
                                                ).await {
                                                    Ok(site_groups_by_slug) => {
                                                        if let Some(found) = site_groups_by_slug.first() {
                                                            info!("Found existing site group {} in NetBox (ID: {}) by slug after create conflict", found.name, found.id);
                                                            Some(found.clone())
                                                        } else {
                                                            None
                                                        }
                                                    }
                                                    Err(_) => None
                                                }
                                            } else {
                                                None
                                            }
                                        }
                                    }
                                    Err(_query_err) => {
                                        // Query failed (likely deserialization issue), try fallback: query all site groups
                                        warn!("Query by name failed for site group {}, trying fallback: query all site groups", site_group_crd.spec.name);
                                        match self.netbox_client.query_site_groups(&[], true).await {
                                            Ok(all_site_groups) => {
                                                // Try to match by name first, then by slug
                                                let found = all_site_groups.iter().find(|sg| {
                                                    sg.name == site_group_crd.spec.name || 
                                                    (site_group_crd.spec.slug.is_some() && sg.slug.as_str() == site_group_crd.spec.slug.as_ref().unwrap().as_str())
                                                });
                                                if let Some(found) = found {
                                                    info!("Found existing site group {} in NetBox (ID: {}) via fallback query", found.name, found.id);
                                                    Some(found.clone())
                                                } else {
                                                    warn!("Fallback query returned {} site groups but none matched name '{}' or slug '{:?}'", all_site_groups.len(), site_group_crd.spec.name, site_group_crd.spec.slug);
                                                    None
                                                }
                                            }
                                            Err(e) => {
                                                warn!("Fallback query for all site groups failed: {}", e);
                                                None
                                            }
                                        }
                                    }
                                };
                                
                                if let Some(found) = found_site_group {
                                    found
                                } else {
                                    let error_msg = format!("Site group {} already exists in NetBox but could not retrieve it: {}", site_group_crd.spec.name, e);
                                    error!("{}", error_msg);
                                    return Err(ControllerError::NetBox(e));
                                }
                            } else {
                                let error_msg = format!("Failed to create site group in NetBox: {}", e);
                                error!("{}", error_msg);
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    }
                };
                
                netbox_site_group
            }
        };
        
        // Update status (use lowercase state to match CRD validation schema)
        let status_patch = Self::create_resource_status_patch(
            netbox_site_group.id,
            netbox_site_group.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_site_group_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
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
