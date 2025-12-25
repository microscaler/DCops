//! NetBoxPlatform reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxPlatform, ResourceState};

impl Reconciler {
    pub async fn reconcile_netbox_platform(&self, platform_crd: &NetBoxPlatform) -> Result<(), ControllerError> {
        let name = platform_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxPlatform missing name".to_string()))?;
        let namespace = platform_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxPlatform {}/{}", namespace, name);
        
        // Check if already created - use helper for drift detection
        let netbox_platform = if let Some(status) = &platform_crd.status {
            if status.state == ResourceState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    // Note: Platform doesn't have get_platform(id), so we query by name and check ID
                    match self.netbox_client.query_platforms(
                        &[("name", &platform_crd.spec.name)],
                        false,
                    ).await {
                        Ok(platforms) => {
                            if let Some(found) = platforms.iter().find(|p| p.id == netbox_id) {
                                // Resource exists and matches ID
                                Some(found.clone())
                            } else {
                                // Resource not found or ID mismatch - drift detected
                                warn!("NetBoxPlatform {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
                                let status_patch = Self::create_resource_status_patch(
                                    0, // Clear netbox_id
                                    String::new(), // Clear URL
                                    ResourceState::Pending,
                                    Some("Resource was deleted in NetBox, will recreate".to_string()),
                                );
                                let pp = kube::api::PatchParams::default();
                                if let Err(e) = self.netbox_platform_api
                                    .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                                    .await
                                {
                                    warn!("Failed to clear NetBoxPlatform status after drift detection: {}", e);
                                }
                                // Fall through to creation
                                None
                            }
                        }
                        Err(e) => {
                            // Query error - return to retry
                            return Err(ControllerError::NetBox(e));
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
        
        // Handle existing platform (from helper) or create new
        let netbox_platform = match netbox_platform {
            Some(platform) => {
                // Resource exists and is up-to-date - only update status if it changed
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    platform_crd.status.as_ref(),
                    platform.id,
                    &platform.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_resource_status_patch(
                        platform.id,
                        platform.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_platform_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxPlatform {}/{} status: NetBox ID {}", namespace, name, platform.id);
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update NetBoxPlatform status: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxPlatform {}/{} already has correct status (ID: {}), skipping update", namespace, name, platform.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create platform - resolve dependencies first
                // Resolve manufacturer ID if manufacturer reference provided
                let manufacturer_id = if let Some(mfg_ref) = &platform_crd.spec.manufacturer {
                    if mfg_ref.kind != "NetBoxManufacturer" {
                        warn!("Invalid kind '{}' for manufacturer reference in platform {}, expected 'NetBoxManufacturer'", mfg_ref.kind, name);
                        None
                    } else {
                        match self.netbox_manufacturer_api.get(&mfg_ref.name).await {
                            Ok(mfg_crd) => {
                                mfg_crd.status
                                    .as_ref()
                                    .and_then(|s| s.netbox_id)
                            }
                            Err(_) => {
                                warn!("Manufacturer CRD '{}' not found for platform {}", mfg_ref.name, name);
                                None
                            }
                        }
                    }
                } else {
                    None
                };
                
                // Try to find existing platform by name
                let existing_platform = match self.netbox_client.query_platforms(
                    &[("name", &platform_crd.spec.name)],
                    false,
                ).await {
                    Ok(platforms) => platforms.first().cloned(),
                    Err(_) => None
                };
                
                if let Some(existing) = existing_platform {
                    info!("Platform {} already exists in NetBox (ID: {})", platform_crd.spec.name, existing.id);
                    existing
                } else {
                    match self.netbox_client.create_platform(
                        &platform_crd.spec.name,
                        platform_crd.spec.slug.as_deref(),
                        manufacturer_id,
                        platform_crd.spec.napalm_driver.as_deref(),
                        platform_crd.spec.napalm_args.as_deref(),
                        platform_crd.spec.description.clone(),
                        platform_crd.spec.comments.clone(),
                    ).await {
                        Ok(created) => {
                            info!("Created platform {} in NetBox (ID: {})", created.name, created.id);
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create platform in NetBox: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                        }
                    }
                }
            }
        };
        
        // Update status (use lowercase state to match CRD validation schema)
        let status_patch = Self::create_resource_status_patch(
            netbox_platform.id,
            netbox_platform.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_platform_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxPlatform {}/{} status: NetBox ID {}", namespace, name, netbox_platform.id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxPlatform status: {}", e);
                error!("{}", error_msg);
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}
