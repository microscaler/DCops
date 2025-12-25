//! NetBoxLocation reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers;
use tracing::{info, error, debug, warn};
use crds::{NetBoxLocation, ResourceState};

impl Reconciler {
    pub async fn reconcile_netbox_location(&self, location_crd: &NetBoxLocation) -> Result<(), ControllerError> {
        let name = location_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxLocation missing name".to_string()))?;
        let namespace = location_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxLocation {}/{}", namespace, name);
        
        // Check if already created - use helper for drift detection
        let netbox_location = if let Some(status) = &location_crd.status {
            if status.state == ResourceState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    // Use simple helper function for drift detection (no update logic)
                    match reconcile_helpers::check_existing(
                        &self.netbox_client,
                        netbox_id,
                        &format!("NetBoxLocation {}/{}", namespace, name),
                        self.netbox_client.get_location(netbox_id),
                    ).await {
                        Ok(Some(resource)) => {
                            // Resource exists and is up-to-date
                            Some(resource)
                        }
                        Ok(None) => {
                            // Drift detected - resource was deleted, clear status and recreate
                            warn!("NetBoxLocation {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
                            let status_patch = Self::create_resource_status_patch(
                                0, // Clear netbox_id
                                String::new(), // Clear URL
                                ResourceState::Pending,
                                Some("Resource was deleted in NetBox, will recreate".to_string()),
                            );
                            let pp = kube::api::PatchParams::default();
                            if let Err(e) = self.netbox_location_api
                                .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                                .await
                            {
                                warn!("Failed to clear NetBoxLocation status after drift detection: {}", e);
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
        
        // Handle existing location (from helper) or create new
        let netbox_location = match netbox_location {
            Some(location) => {
                // Resource exists and is up-to-date - only update status if it changed
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    location_crd.status.as_ref(),
                    location.id,
                    &location.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_resource_status_patch(
                        location.id,
                        location.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_location_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxLocation {}/{} status: NetBox ID {}", namespace, name, location.id);
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update NetBoxLocation status: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxLocation {}/{} already has correct status (ID: {}), skipping update", namespace, name, location.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create location - resolve dependencies first
                // Resolve site ID (required)
                if location_crd.spec.site.kind != "NetBoxSite" {
                    return Err(ControllerError::InvalidConfig(
                        format!("Invalid kind '{}' for site reference in location {}, expected 'NetBoxSite'", location_crd.spec.site.kind, name)
                    ));
                }
                let site_id = match self.netbox_site_api.get(&location_crd.spec.site.name).await {
                    Ok(site_crd) => {
                        site_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                            .ok_or_else(|| ControllerError::InvalidConfig(
                                format!("Site '{}' has not been created in NetBox yet (no netbox_id in status)", location_crd.spec.site.name)
                            ))?
                    }
                    Err(_) => {
                        return Err(ControllerError::InvalidConfig(
                            format!("Site CRD '{}' not found for location {}", location_crd.spec.site.name, name)
                        ));
                    }
                };
                
                // Resolve parent location ID if parent reference provided
                let parent_id = if let Some(parent_ref) = &location_crd.spec.parent {
            if parent_ref.kind != "NetBoxLocation" {
                warn!("Invalid kind '{}' for parent location reference in location {}, expected 'NetBoxLocation'", parent_ref.kind, name);
                None
            } else {
                match self.netbox_location_api.get(&parent_ref.name).await {
                    Ok(parent_crd) => {
                        parent_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                    }
                    Err(_) => {
                        warn!("Parent location CRD '{}' not found for location {}", parent_ref.name, name);
                        None
                    }
                }
            }
                } else {
                    None
                };
                
                // Try to find existing location by name and site
                let existing_location = match self.netbox_client.query_locations(
                    &[("site_id", &site_id.to_string()), ("name", &location_crd.spec.name)],
                    false,
                ).await {
                    Ok(locations) => locations.first().cloned(),
                    Err(_) => None
                };
                
                let netbox_location = if let Some(existing) = existing_location {
                    info!("Location {} already exists in NetBox (ID: {})", location_crd.spec.name, existing.id);
                    existing
                } else {
                    match self.netbox_client.create_location(
                        site_id,
                        &location_crd.spec.name,
                        location_crd.spec.slug.as_deref(),
                        parent_id,
                        location_crd.spec.description.clone(),
                        None, // comments not in spec
                    ).await {
                        Ok(created) => {
                            info!("Created location {} in NetBox (ID: {})", created.name, created.id);
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create location in NetBox: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                };
                
                netbox_location
            }
        };
        
        // Update status (use lowercase state to match CRD validation schema)
        let status_patch = Self::create_resource_status_patch(
            netbox_location.id,
            netbox_location.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_location_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxLocation {}/{} status: NetBox ID {}", namespace, name, netbox_location.id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxLocation status: {}", e);
                error!("{}", error_msg);
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}
