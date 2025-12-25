//! Extras reconcilers (Roles, Tags)

use super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers;
use crds::{NetBoxRole, NetBoxTag, ResourceState};
use tracing::{info, error, warn, debug};

impl Reconciler {
    /// Reconciles a NetBoxRole resource.
    pub async fn reconcile_netbox_role(&self, role_crd: &NetBoxRole) -> Result<(), ControllerError> {
        let name = role_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxRole missing name".to_string()))?;
        let namespace = role_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxRole {}/{}", namespace, name);
        
        // Check if already created - use helper for drift detection
        // Note: Roles don't have update logic yet, so we use the simple check_existing helper
        let netbox_role = if let Some(status) = &role_crd.status {
            if status.state == ResourceState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    // Use simple helper function for drift detection (no update logic)
                    match reconcile_helpers::check_existing(
                        &self.netbox_client,
                        netbox_id,
                        &format!("NetBoxRole {}/{}", namespace, name),
                        self.netbox_client.get_role(netbox_id),
                    ).await {
                        Ok(Some(resource)) => {
                            // Resource exists and is up-to-date
                            Some(resource)
                        }
                        Ok(None) => {
                            // Drift detected - resource was deleted, clear status and recreate
                            warn!("NetBoxRole {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
                            let status_patch = Self::create_resource_status_patch(
                                0, // Clear netbox_id
                                String::new(), // Clear URL
                                ResourceState::Pending,
                                Some("Resource was deleted in NetBox, will recreate".to_string()),
                            );
                            let pp = kube::api::PatchParams::default();
                            if let Err(e) = self.netbox_role_api
                                .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                                .await
                            {
                                warn!("Failed to clear NetBoxRole status after drift detection: {}", e);
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
                    match self.netbox_role_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxRole {}/{} status: NetBox ID {}", namespace, name, role.id);
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update NetBoxRole status: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxRole {}/{} already has correct status (ID: {}), skipping update", namespace, name, role.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create role - try to find existing by name (idempotency fallback)
                let existing_role = match self.netbox_client.query_roles(
                    &[("name", &role_crd.spec.name)],
                    false,
                ).await {
                    Ok(roles) => roles.first().cloned(),
                    Err(_) => None
                };
                
                if let Some(existing) = existing_role {
                    info!("Role {} already exists in NetBox (ID: {})", role_crd.spec.name, existing.id);
                    existing
                } else {
                    // Create new role
                    match self.netbox_client.create_role(
                        &role_crd.spec.name,
                        role_crd.spec.slug.as_deref(),
                        role_crd.spec.description.clone(),
                        role_crd.spec.weight,
                        role_crd.spec.comments.clone(),
                    ).await {
                        Ok(created) => {
                            info!("Created role {} in NetBox (ID: {})", created.name, created.id);
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create role in NetBox: {}", e);
                            error!("{}", error_msg);
                            // Update status with error
                            let status_patch = Self::create_resource_status_patch(
                                0,
                                String::new(),
                                ResourceState::Failed,
                                Some(error_msg.clone()),
                            );
                            let pp = kube::api::PatchParams::default();
                            if let Err(status_err) = self.netbox_role_api
                                .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                                .await
                            {
                                error!("Failed to update NetBoxRole error status: {}", status_err);
                            }
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                }
            }
        };
        
        // Update status
        let status_patch = Self::create_resource_status_patch(
            netbox_role.id,
            netbox_role.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_role_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxRole {}/{} status: NetBox ID {}", namespace, name, netbox_role.id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxRole status: {}", e);
                error!("{}", error_msg);
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
    
    /// Reconciles a NetBoxTag resource.
    pub async fn reconcile_netbox_tag(&self, tag_crd: &NetBoxTag) -> Result<(), ControllerError> {
        let name = tag_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxTag missing name".to_string()))?;
        let namespace = tag_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxTag {}/{}", namespace, name);
        
        // Check if already created - use helper for drift detection
        // Note: Tags don't have update logic yet, so we use the simple check_existing helper
        let netbox_tag = if let Some(status) = &tag_crd.status {
            if status.state == ResourceState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    // Use simple helper function for drift detection (no update logic)
                    match reconcile_helpers::check_existing(
                        &self.netbox_client,
                        netbox_id,
                        &format!("NetBoxTag {}/{}", namespace, name),
                        self.netbox_client.get_tag(netbox_id),
                    ).await {
                        Ok(Some(resource)) => {
                            // Resource exists and is up-to-date
                            Some(resource)
                        }
                        Ok(None) => {
                            // Drift detected - resource was deleted, clear status and recreate
                            warn!("NetBoxTag {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
                            let status_patch = Self::create_resource_status_patch(
                                0, // Clear netbox_id
                                String::new(), // Clear URL
                                ResourceState::Pending,
                                Some("Resource was deleted in NetBox, will recreate".to_string()),
                            );
                            let pp = kube::api::PatchParams::default();
                            if let Err(e) = self.netbox_tag_api
                                .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                                .await
                            {
                                warn!("Failed to clear NetBoxTag status after drift detection: {}", e);
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
        
        // Handle existing tag (from helper) or create new
        let netbox_tag = match netbox_tag {
            Some(tag) => {
                // Resource exists and is up-to-date - only update status if it changed
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    tag_crd.status.as_ref(),
                    tag.id,
                    &tag.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_resource_status_patch(
                        tag.id,
                        tag.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_tag_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxTag {}/{} status: NetBox ID {}", namespace, name, tag.id);
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update NetBoxTag status: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxTag {}/{} already has correct status (ID: {}), skipping update", namespace, name, tag.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create tag - try to find existing by name (idempotency fallback)
                let existing_tag = match self.netbox_client.query_tags(
                    &[("name", &tag_crd.spec.name)],
                    false,
                ).await {
                    Ok(tags) => tags.first().cloned(),
                    Err(_) => None
                };
                
                if let Some(existing) = existing_tag {
                    info!("Tag {} already exists in NetBox (ID: {})", tag_crd.spec.name, existing.id);
                    existing
                } else {
                    // Create new tag
                    match self.netbox_client.create_tag(
                        &tag_crd.spec.name,
                        tag_crd.spec.slug.as_deref(),
                        tag_crd.spec.color.as_deref(),
                        tag_crd.spec.description.clone(),
                        tag_crd.spec.comments.clone(),
                    ).await {
                        Ok(created) => {
                            info!("Created tag {} in NetBox (ID: {})", created.name, created.id);
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create tag in NetBox: {}", e);
                            error!("{}", error_msg);
                            // Update status with error
                            let status_patch = Self::create_resource_status_patch(
                                0,
                                String::new(),
                                ResourceState::Failed,
                                Some(error_msg.clone()),
                            );
                            let pp = kube::api::PatchParams::default();
                            if let Err(status_err) = self.netbox_tag_api
                                .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                                .await
                            {
                                error!("Failed to update NetBoxTag error status: {}", status_err);
                            }
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                }
            }
        };
        
        // Update status
        let status_patch = Self::create_resource_status_patch(
            netbox_tag.id,
            netbox_tag.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_tag_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxTag {}/{} status: NetBox ID {}", namespace, name, netbox_tag.id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxTag status: {}", e);
                error!("{}", error_msg);
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}

