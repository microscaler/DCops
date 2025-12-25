//! NetBoxDeviceRole reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxDeviceRole, ResourceState};

impl Reconciler {
    pub async fn reconcile_netbox_device_role(&self, role_crd: &NetBoxDeviceRole) -> Result<(), ControllerError> {
        let name = role_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxDeviceRole missing name".to_string()))?;
        let namespace = role_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxDeviceRole {}/{}", namespace, name);
        
        // Check if already created - use helper for drift detection
        let netbox_role = if let Some(status) = &role_crd.status {
            if status.state == ResourceState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    // Use simple helper function for drift detection (no update logic)
                    // Note: DeviceRole doesn't have get_device_role(id), so we query by name and check ID
                    match self.netbox_client.query_device_roles(
                        &[("name", &role_crd.spec.name)],
                        false,
                    ).await {
                        Ok(roles) => {
                            if let Some(found) = roles.iter().find(|r| r.id == netbox_id) {
                                // Resource exists and matches ID
                                Some(found.clone())
                            } else {
                                // Resource not found or ID mismatch - drift detected
                                warn!("NetBoxDeviceRole {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
                                let status_patch = Self::create_resource_status_patch(
                                    0, // Clear netbox_id
                                    String::new(), // Clear URL
                                    ResourceState::Pending,
                                    Some("Resource was deleted in NetBox, will recreate".to_string()),
                                );
                                let pp = kube::api::PatchParams::default();
                                if let Err(e) = self.netbox_device_role_api
                                    .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                                    .await
                                {
                                    warn!("Failed to clear NetBoxDeviceRole status after drift detection: {}", e);
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
        
        // Handle existing role (from helper) or create new
        let netbox_role = match netbox_role {
            Some(role) => {
                // Resource exists and is up-to-date - only update status if it changed
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    role_crd.status.as_ref(),
                    role.id,
                    &role.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_resource_status_patch(
                        role.id,
                        role.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_device_role_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxDeviceRole {}/{} status: NetBox ID {}", namespace, name, role.id);
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update NetBoxDeviceRole status: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxDeviceRole {}/{} already has correct status (ID: {}), skipping update", namespace, name, role.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create device role - try to find existing by name (idempotency fallback)
                // Try to find existing device role by name
                let existing_role = match self.netbox_client.query_device_roles(
                    &[("name", &role_crd.spec.name)],
                    false,
                ).await {
                    Ok(roles) => roles.first().cloned(),
                    Err(_) => None
                };
                
                let netbox_role = if let Some(existing) = existing_role {
                    info!("Device role {} already exists in NetBox (ID: {})", role_crd.spec.name, existing.id);
                    existing
                } else {
                    match self.netbox_client.create_device_role(
                        &role_crd.spec.name,
                        role_crd.spec.slug.as_deref(),
                        role_crd.spec.color.as_deref(),
                        Some(role_crd.spec.vm_role),
                        role_crd.spec.description.clone(),
                        role_crd.spec.comments.clone(),
                    ).await {
                        Ok(created) => {
                            info!("Created device role {} in NetBox (ID: {})", created.name, created.id);
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create device role in NetBox: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                        }
                    }
                };
                
                netbox_role
            }
        };
        
        // Update status (use lowercase state to match CRD validation schema)
        let status_patch = Self::create_resource_status_patch(
            netbox_role.id,
            netbox_role.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_device_role_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxDeviceRole {}/{} status: NetBox ID {}", namespace, name, netbox_role.id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxDeviceRole status: {}", e);
                error!("{}", error_msg);
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}
