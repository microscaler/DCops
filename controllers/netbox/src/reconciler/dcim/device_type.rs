//! NetBoxDeviceType reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers;
use tracing::{info, error, debug, warn};
use crds::{NetBoxDeviceType, NetBoxDeviceTypeStatus, ResourceState};
use netbox_client::NetBoxClientTrait;

impl Reconciler {
    pub async fn reconcile_netbox_device_type(&self, device_type_crd: &NetBoxDeviceType) -> Result<(), ControllerError> {
        let name = device_type_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxDeviceType missing name".to_string()))?;
        let namespace = device_type_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxDeviceType {}/{}", namespace, name);
        
        // Get client for shared resource (finds tenant from referencing Devices)
        let netbox_client = self.token_resolver
            .create_client_for_shared_resource(namespace, "NetBoxDeviceType", name)
            .await
            .map_err(|e| ControllerError::TokenResolution(e))?;
        
        // Resolve manufacturer ID (required)
        let manufacturer_id = if device_type_crd.spec.manufacturer.kind != "NetBoxManufacturer" {
            return Err(ControllerError::InvalidConfig(
                format!("Invalid kind '{}' for manufacturer reference in device type {}, expected 'NetBoxManufacturer'", device_type_crd.spec.manufacturer.kind, name)
            ));
        } else {
            match self.netbox_manufacturer_api.get(&device_type_crd.spec.manufacturer.name).await {
                Ok(manufacturer_crd) => {
                    manufacturer_crd.status
                        .as_ref()
                        .and_then(|s| s.netbox_id)
                        .ok_or_else(|| ControllerError::InvalidConfig(
                            format!("Manufacturer '{}' has not been created in NetBox yet (no netbox_id in status)", device_type_crd.spec.manufacturer.name)
                        ))?
                }
                Err(_) => {
                    return Err(ControllerError::InvalidConfig(
                        format!("Manufacturer CRD '{}' not found for device type {}", device_type_crd.spec.manufacturer.name, name)
                    ));
                }
            }
        };
        
        // Check if already created - use helper for drift detection
        let netbox_device_type = if let Some(status) = &device_type_crd.status {
            if status.state == ResourceState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    match reconcile_helpers::check_existing(
                        &netbox_client,
                        netbox_id,
                        &format!("NetBoxDeviceType {}/{}", namespace, name),
                        async {
                            let id_str = netbox_id.to_string();
                            netbox_client.query_device_types(&[("id", &id_str)], false)
                                .await
                                .and_then(|mut device_types| {
                                    device_types.pop().ok_or_else(|| netbox_client::NetBoxError::NotFound(format!("DeviceType {} not found", netbox_id)))
                                })
                        },
                    ).await {
                        Ok(Some(resource)) => Some(resource),
                        Ok(None) => {
                            warn!("NetBoxDeviceType {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
                            let status_patch = Self::create_resource_status_patch(
                                0, String::new(), ResourceState::Pending,
                                Some("Resource was deleted in NetBox, will recreate".to_string()),
                            );
                            let pp = kube::api::PatchParams::default();
                            if let Err(e) = self.netbox_device_type_api
                                .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                                .await
                            {
                                warn!("Failed to clear NetBoxDeviceType status after drift detection: {}", e);
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
        
        let netbox_device_type = match netbox_device_type {
            Some(device_type) => {
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    device_type_crd.status.as_ref(),
                    device_type.id,
                    &device_type.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_resource_status_patch(
                        device_type.id,
                        device_type.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_device_type_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxDeviceType {}/{} status: NetBox ID {}", namespace, name, device_type.id);
                            return Ok(());
                        }
                        Err(e) => {
                            error!("Failed to update NetBoxDeviceType status: {}", e);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxDeviceType {}/{} already has correct status (ID: {}), skipping update", namespace, name, device_type.id);
                    return Ok(());
                }
            }
            None => {
                // Try to find existing by model and manufacturer
                let existing_device_type = match netbox_client.get_device_type_by_model(manufacturer_id, &device_type_crd.spec.model).await {
                    Ok(Some(dt)) => {
                        info!("DeviceType {} (manufacturer ID: {}) already exists in NetBox (ID: {}), acknowledging existence (idempotency)", device_type_crd.spec.model, manufacturer_id, dt.id);
                        Some(dt)
                    }
                    Ok(None) => None,
                    Err(e) => {
                        warn!("Failed to query device type by model: {}, will try to create", e);
                        None
                    }
                };
                
                if let Some(existing) = existing_device_type {
                    existing
                } else {
                    info!("Creating device type {} in NetBox", device_type_crd.spec.model);
                    match netbox_client.create_device_type(
                        manufacturer_id,
                        &device_type_crd.spec.model,
                        device_type_crd.spec.slug.as_deref(),
                        device_type_crd.spec.part_number.as_deref(),
                        Some(device_type_crd.spec.u_height),
                        Some(device_type_crd.spec.is_full_depth),
                        device_type_crd.spec.description.clone(),
                        device_type_crd.spec.comments.clone(),
                    ).await {
                        Ok(created) => {
                            info!("Created device type {} in NetBox (ID: {})", created.model, created.id);
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create device type in NetBox: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                }
            }
        };
        
        let status_patch = Self::create_resource_status_patch(
            netbox_device_type.id,
            netbox_device_type.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_device_type_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxDeviceType {}/{} status: NetBox ID {}", namespace, name, netbox_device_type.id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxDeviceType status: {}", e);
                error!("{}", error_msg);
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}
