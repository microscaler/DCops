//! NetBoxManufacturer reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxManufacturer, ResourceState};

impl Reconciler {
    pub async fn reconcile_netbox_manufacturer(&self, mfg_crd: &NetBoxManufacturer) -> Result<(), ControllerError> {
        let name = mfg_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxManufacturer missing name".to_string()))?;
        let namespace = mfg_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxManufacturer {}/{}", namespace, name);
        
        // Check if already created - use helper for drift detection
        let netbox_mfg = if let Some(status) = &mfg_crd.status {
            if status.state == ResourceState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    // Note: Manufacturer doesn't have get_manufacturer(id), so we query by name and check ID
                    match self.netbox_client.query_manufacturers(
                        &[("name", &mfg_crd.spec.name)],
                        false,
                    ).await {
                        Ok(manufacturers) => {
                            if let Some(found) = manufacturers.iter().find(|m| m.id == netbox_id) {
                                // Resource exists and matches ID
                                Some(found.clone())
                            } else {
                                // Resource not found or ID mismatch - drift detected
                                warn!("NetBoxManufacturer {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
                                let status_patch = Self::create_resource_status_patch(
                                    0, // Clear netbox_id
                                    String::new(), // Clear URL
                                    ResourceState::Pending,
                                    Some("Resource was deleted in NetBox, will recreate".to_string()),
                                );
                                let pp = kube::api::PatchParams::default();
                                if let Err(e) = self.netbox_manufacturer_api
                                    .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                                    .await
                                {
                                    warn!("Failed to clear NetBoxManufacturer status after drift detection: {}", e);
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
        
        // Handle existing manufacturer (from helper) or create new
        let netbox_mfg = match netbox_mfg {
            Some(mfg) => {
                // Resource exists and is up-to-date - only update status if it changed
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    mfg_crd.status.as_ref(),
                    mfg.id,
                    &mfg.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_resource_status_patch(
                        mfg.id,
                        mfg.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_manufacturer_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxManufacturer {}/{} status: NetBox ID {}", namespace, name, mfg.id);
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update NetBoxManufacturer status: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxManufacturer {}/{} already has correct status (ID: {}), skipping update", namespace, name, mfg.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create manufacturer - try to find existing by name (idempotency fallback)
                let existing_mfg = match self.netbox_client.query_manufacturers(
                    &[("name", &mfg_crd.spec.name)],
                    false,
                ).await {
                    Ok(manufacturers) => manufacturers.first().cloned(),
                    Err(_) => None
                };
                
                if let Some(existing) = existing_mfg {
                    info!("Manufacturer {} already exists in NetBox (ID: {})", mfg_crd.spec.name, existing.id);
                    existing
                } else {
                    let slug = mfg_crd.spec.slug.as_deref().map(|s| s.to_string())
                        .unwrap_or_else(|| mfg_crd.spec.name.to_lowercase().replace(' ', "-"));
                    match self.netbox_client.create_manufacturer(
                        &mfg_crd.spec.name,
                        &slug,
                        mfg_crd.spec.description.as_deref(),
                    ).await {
                        Ok(created) => {
                            info!("Created manufacturer {} in NetBox (ID: {})", created.name, created.id);
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create manufacturer in NetBox: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                        }
                    }
                }
            }
        };
        
        // Update status (use lowercase state to match CRD validation schema)
        let status_patch = Self::create_resource_status_patch(
            netbox_mfg.id,
            netbox_mfg.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_manufacturer_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxManufacturer {}/{} status: NetBox ID {}", namespace, name, netbox_mfg.id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxManufacturer status: {}", e);
                error!("{}", error_msg);
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}
