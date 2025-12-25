//! NetBoxRegion reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers;
use tracing::{info, error, debug, warn};
use crds::{NetBoxRegion, ResourceState};

impl Reconciler {
    pub async fn reconcile_netbox_region(&self, region_crd: &NetBoxRegion) -> Result<(), ControllerError> {
        let name = region_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxRegion missing name".to_string()))?;
        let namespace = region_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxRegion {}/{}", namespace, name);
        
        // Check if already created - use helper for drift detection
        let netbox_region = if let Some(status) = &region_crd.status {
            if status.state == ResourceState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    // Use simple helper function for drift detection (no update logic)
                    match reconcile_helpers::check_existing(
                        &self.netbox_client,
                        netbox_id,
                        &format!("NetBoxRegion {}/{}", namespace, name),
                        self.netbox_client.get_region(netbox_id),
                    ).await {
                        Ok(Some(resource)) => {
                            // Resource exists and is up-to-date
                            Some(resource)
                        }
                        Ok(None) => {
                            // Drift detected - resource was deleted, clear status and recreate
                            warn!("NetBoxRegion {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
                            let status_patch = Self::create_resource_status_patch(
                                0, // Clear netbox_id
                                String::new(), // Clear URL
                                ResourceState::Pending,
                                Some("Resource was deleted in NetBox, will recreate".to_string()),
                            );
                            let pp = kube::api::PatchParams::default();
                            if let Err(e) = self.netbox_region_api
                                .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                                .await
                            {
                                warn!("Failed to clear NetBoxRegion status after drift detection: {}", e);
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
        
        // Handle existing region (from helper) or create new
        let netbox_region = match netbox_region {
            Some(region) => {
                // Resource exists and is up-to-date - only update status if it changed
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    region_crd.status.as_ref(),
                    region.id,
                    &region.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_resource_status_patch(
                        region.id,
                        region.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_region_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxRegion {}/{} status: NetBox ID {}", namespace, name, region.id);
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update NetBoxRegion status: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxRegion {}/{} already has correct status (ID: {}), skipping update", namespace, name, region.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create region - try to find existing by name (idempotency fallback)
                // Resolve parent region ID if parent reference provided
                let parent_id = if let Some(parent_ref) = &region_crd.spec.parent {
            if parent_ref.kind != "NetBoxRegion" {
                warn!("Invalid kind '{}' for parent region reference in region {}, expected 'NetBoxRegion'", parent_ref.kind, name);
                None
            } else {
                match self.netbox_region_api.get(&parent_ref.name).await {
                    Ok(parent_crd) => {
                        parent_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                    }
                    Err(_) => {
                        warn!("Parent region CRD '{}' not found for region {}", parent_ref.name, name);
                        None
                    }
                }
            }
                } else {
                    None
                };
                
                // Try to find existing region by name
                let existing_region = match self.netbox_client.query_regions(
                    &[("name", &region_crd.spec.name)],
                    false,
                ).await {
                    Ok(regions) => regions.first().cloned(),
                    Err(_) => None
                };
                
                let netbox_region = if let Some(existing) = existing_region {
                    info!("Region {} already exists in NetBox (ID: {})", region_crd.spec.name, existing.id);
                    existing
                } else {
                    match self.netbox_client.create_region(
                        &region_crd.spec.name,
                        region_crd.spec.slug.as_deref(),
                        parent_id,
                        region_crd.spec.description.clone(),
                        None, // comments not in spec
                    ).await {
                        Ok(created) => {
                            info!("Created region {} in NetBox (ID: {})", created.name, created.id);
                            created
                        }
                        Err(e) => {
                            // Check if error is "already exists" - if so, try to find it (idempotency)
                            let error_str = format!("{}", e);
                            if error_str.contains("already exists") || error_str.contains("duplicate") || error_str.contains("unique constraint") {
                                warn!("Region {} already exists in NetBox, attempting to retrieve it (idempotency)", region_crd.spec.name);
                                
                                // Try to find the existing region by name or slug
                                let found_region: Option<_> = match self.netbox_client.query_regions(
                                    &[("name", &region_crd.spec.name)],
                                    false,
                                ).await {
                                    Ok(regions) => {
                                        if let Some(found) = regions.first() {
                                            info!("Found existing region {} in NetBox (ID: {}) after create conflict", found.name, found.id);
                                            Some(found.clone())
                                        } else {
                                            // Try by slug
                                            if let Some(slug) = &region_crd.spec.slug {
                                                match self.netbox_client.query_regions(
                                                    &[("slug", slug)],
                                                    false,
                                                ).await {
                                                    Ok(regions_by_slug) => {
                                                        if let Some(found) = regions_by_slug.first() {
                                                            info!("Found existing region {} in NetBox (ID: {}) by slug after create conflict", found.name, found.id);
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
                                        // Query failed (likely deserialization issue), try fallback: query all regions
                                        warn!("Query by name failed for region {}, trying fallback: query all regions", region_crd.spec.name);
                                        match self.netbox_client.query_regions(&[], true).await {
                                            Ok(all_regions) => {
                                                // Try to match by name first, then by slug
                                                let found = all_regions.iter().find(|r| {
                                                    r.name == region_crd.spec.name || 
                                                    (region_crd.spec.slug.is_some() && r.slug.as_str() == region_crd.spec.slug.as_ref().unwrap().as_str())
                                                });
                                                if let Some(found) = found {
                                                    info!("Found existing region {} in NetBox (ID: {}) via fallback query", found.name, found.id);
                                                    Some(found.clone())
                                                } else {
                                                    None
                                                }
                                            }
                                            Err(_) => None
                                        }
                                    }
                                };
                                
                                if let Some(found) = found_region {
                                    found
                                } else {
                                    let error_msg = format!("Region {} already exists in NetBox but could not retrieve it: {}", region_crd.spec.name, e);
                                    error!("{}", error_msg);
                                    return Err(ControllerError::NetBox(e));
                                }
                            } else {
                                let error_msg = format!("Failed to create region in NetBox: {}", e);
                                error!("{}", error_msg);
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    }
                };
                
                netbox_region
            }
        };
        
        // Update status (use lowercase state to match CRD validation schema)
        let status_patch = Self::create_resource_status_patch(
            netbox_region.id,
            netbox_region.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_region_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxRegion {}/{} status: NetBox ID {}", namespace, name, netbox_region.id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxRegion status: {}", e);
                error!("{}", error_msg);
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}
