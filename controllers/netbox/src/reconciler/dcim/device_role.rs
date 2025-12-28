//! NetBoxDeviceRole reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers;
use tracing::{info, error, debug, warn};
use crds::{NetBoxDeviceRole, ResourceState};
use netbox_client::NetBoxClientTrait;

impl Reconciler {
    pub async fn reconcile_netbox_device_role(&self, device_role_crd: &NetBoxDeviceRole) -> Result<(), ControllerError> {
        let name = device_role_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxDeviceRole missing name".to_string()))?;
        let namespace = device_role_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxDeviceRole {}/{}", namespace, name);
        
        // Get client for shared resource (finds tenant from referencing Devices)
        let netbox_client = self.token_resolver
            .create_client_for_shared_resource(namespace, "NetBoxDeviceRole", name)
            .await
            .map_err(|e| ControllerError::TokenResolution(e))?;
        
        // Check if already created - use helper for drift detection
        let netbox_device_role = if let Some(status) = &device_role_crd.status {
            if status.state == ResourceState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    match reconcile_helpers::check_existing(
                        &netbox_client,
                        netbox_id,
                        &format!("NetBoxDeviceRole {}/{}", namespace, name),
                        async {
                            let id_str = netbox_id.to_string();
                            netbox_client.query_device_roles(&[("id", &id_str)], false)
                                .await
                                .and_then(|mut device_roles| {
                                    device_roles.pop().ok_or_else(|| netbox_client::NetBoxError::NotFound(format!("DeviceRole {} not found", netbox_id)))
                                })
                        },
                    ).await {
                        Ok(Some(resource)) => Some(resource),
                        Ok(None) => {
                            warn!("NetBoxDeviceRole {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
                            let status_patch = Self::create_typed_device_role_status_patch(
                                0, String::new(), ResourceState::Pending,
                                Some("Resource was deleted in NetBox, will recreate".to_string()),
                            );
                            let pp = kube::api::PatchParams::default();
                            if let Err(e) = self.netbox_device_role_api
                                .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                                .await
                            {
                                warn!("Failed to clear NetBoxDeviceRole status after drift detection: {}", e);
                            }
                            None
                        }
                        Err(e) => return Err(e),
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        let netbox_device_role = match netbox_device_role {
            Some(device_role) => {
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    device_role_crd.status.as_ref(),
                    device_role.id,
                    &device_role.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_typed_device_role_status_patch(
                        device_role.id,
                        device_role.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_device_role_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxDeviceRole {}/{} status: NetBox ID {}", namespace, name, device_role.id);
                            return Ok(());
                        }
                        Err(e) => {
                            error!("Failed to update NetBoxDeviceRole status: {}", e);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxDeviceRole {}/{} already has correct status (ID: {}), skipping update", namespace, name, device_role.id);
                    return Ok(());
                }
            }
            None => {
                let existing_device_role = match netbox_client.get_device_role_by_name(&device_role_crd.spec.name).await {
                    Ok(Some(dr)) => {
                        info!("DeviceRole {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", device_role_crd.spec.name, dr.id);
                        Some(dr)
                    }
                    Ok(None) => None,
                    Err(e) => {
                        warn!("Failed to query device role by name: {}, will try to create", e);
                        None
                    }
                };
                
                if let Some(existing) = existing_device_role {
                    existing
                } else {
                    info!("Creating device role {} in NetBox", device_role_crd.spec.name);
                    match netbox_client.create_device_role(
                        &device_role_crd.spec.name,
                        device_role_crd.spec.slug.as_deref(),
                        device_role_crd.spec.color.as_deref(),
                        Some(device_role_crd.spec.vm_role),
                        device_role_crd.spec.description.clone(),
                        device_role_crd.spec.comments.clone(),
                    ).await {
                        Ok(created) => {
                            info!("Created device role {} in NetBox (ID: {})", created.name, created.id);
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create device role in NetBox: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                }
            }
        };
        
        let status_patch = Self::create_typed_device_role_status_patch(
            netbox_device_role.id,
            netbox_device_role.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_device_role_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxDeviceRole {}/{} status: NetBox ID {}", namespace, name, netbox_device_role.id);
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
