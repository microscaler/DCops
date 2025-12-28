//! Extras reconcilers (Roles, Tags)

use super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxRole, NetBoxTag, ResourceState};
use netbox_client::NetBoxClientTrait;

impl Reconciler {
    /// Reconciles a NetBoxRole resource (Extras Role, not IPAM Role).
    pub async fn reconcile_netbox_role(&self, role_crd: &NetBoxRole) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::extract_name_and_namespace;
        let (name, namespace) = extract_name_and_namespace(role_crd, "NetBoxRole")?;
        
        info!("Reconciling NetBoxRole {}/{}", namespace, name);
        
        // Get client for shared resource (finds tenant from referencing resources or uses system tenant)
        let netbox_client = self.token_resolver
            .create_client_for_shared_resource(namespace, "NetBoxRole", name)
            .await
            .map_err(|e| ControllerError::TokenResolution(e))?;
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                role_crd.status.as_ref(),
                "NetBoxRole",
                namespace,
                name,
                |netbox_id| async move {
                    let id_str = netbox_id.to_string();
                    netbox_client_ref.query_roles(&[("id", &id_str)], false)
                        .await
                        .and_then(|mut roles| {
                            roles.pop().ok_or_else(|| netbox_client::NetBoxError::NotFound(format!("Role {} not found", netbox_id)))
                        })
                },
            ).await?
        };
        
        let netbox_role = match drift_result {
            DriftCheckResult::UseExisting(role) => Some(role),
            DriftCheckResult::StatusCleared { message } => {
                let status_patch = Self::create_typed_role_status_patch(
                    0, String::new(), ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_role_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxRole status: {}", update_err);
                }
                None
            }
            DriftCheckResult::Recreate => None,
        };
        
        let netbox_role = match netbox_role {
            Some(role) => {
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    role_crd.status.as_ref(),
                    role.id,
                    &role.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_typed_role_status_patch(
                        role.id,
                        role.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_role_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxRole {}/{} status: NetBox ID {}", namespace, name, role.id);
                            return Ok(());
                        }
                        Err(e) => {
                            error!("Failed to update NetBoxRole status: {}", e);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxRole {}/{} already has correct status (ID: {}), skipping update", namespace, name, role.id);
                    return Ok(());
                }
            }
            None => {
                let existing_role = match netbox_client.query_roles(&[("name", &role_crd.spec.name)], false).await {
                    Ok(mut roles) => {
                        roles.pop()
                    }
                    Err(e) => {
                        warn!("Failed to query role by name: {}, will try to create", e);
                        None
                    }
                };
                
                if let Some(r) = existing_role.as_ref() {
                    info!("Role {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", role_crd.spec.name, r.id);
                }
                
                if let Some(existing) = existing_role {
                    existing
                } else {
                    info!("Creating role {} in NetBox", role_crd.spec.name);
                    match netbox_client.create_role(
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
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                }
            }
        };
        
        let status_patch = Self::create_typed_role_status_patch(
            netbox_role.id,
            netbox_role.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_role_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
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
        
        // Get client for shared resource (finds tenant from referencing resources or uses system tenant)
        let netbox_client = self.token_resolver
            .create_client_for_shared_resource(namespace, "NetBoxTag", name)
            .await
            .map_err(|e| ControllerError::TokenResolution(e))?;
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                tag_crd.status.as_ref(),
                "NetBoxTag",
                namespace,
                name,
                |netbox_id| async move {
                    let id_str = netbox_id.to_string();
                    netbox_client_ref.query_tags(&[("id", &id_str)], false)
                        .await
                        .and_then(|mut tags| {
                            tags.pop().ok_or_else(|| netbox_client::NetBoxError::NotFound(format!("Tag {} not found", netbox_id)))
                        })
                },
            ).await?
        };
        
        let netbox_tag = match drift_result {
            DriftCheckResult::UseExisting(tag) => Some(tag),
            DriftCheckResult::StatusCleared { message } => {
                let status_patch = Self::create_typed_tag_status_patch(
                    0, String::new(), ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_tag_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxTag status: {}", update_err);
                }
                None
            }
            DriftCheckResult::Recreate => None,
        };
        
        let netbox_tag = match netbox_tag {
            Some(tag) => {
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    tag_crd.status.as_ref(),
                    tag.id,
                    &tag.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_typed_tag_status_patch(
                        tag.id,
                        tag.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_tag_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxTag {}/{} status: NetBox ID {}", namespace, name, tag.id);
                            return Ok(());
                        }
                        Err(e) => {
                            error!("Failed to update NetBoxTag status: {}", e);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxTag {}/{} already has correct status (ID: {}), skipping update", namespace, name, tag.id);
                    return Ok(());
                }
            }
            None => {
                let existing_tag = match netbox_client.query_tags(&[("name", &tag_crd.spec.name)], false).await {
                    Ok(mut tags) => {
                        tags.pop()
                    }
                    Err(e) => {
                        warn!("Failed to query tag by name: {}, will try to create", e);
                        None
                    }
                };
                
                if let Some(t) = existing_tag.as_ref() {
                    info!("Tag {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", tag_crd.spec.name, t.id);
                }
                
                if let Some(existing) = existing_tag {
                    existing
                } else {
                    info!("Creating tag {} in NetBox", tag_crd.spec.name);
                    match netbox_client.create_tag(
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
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                }
            }
        };
        
        let status_patch = Self::create_typed_tag_status_patch(
            netbox_tag.id,
            netbox_tag.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_tag_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
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
