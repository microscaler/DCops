//! NetBoxDeviceType reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxDeviceType, ResourceState};

impl Reconciler {
    pub async fn reconcile_netbox_device_type(&self, device_type_crd: &NetBoxDeviceType) -> Result<(), ControllerError> {
        let name = device_type_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxDeviceType missing name".to_string()))?;
        let namespace = device_type_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxDeviceType {}/{}", namespace, name);
        
        // Validate manufacturer reference kind
        if device_type_crd.spec.manufacturer.kind != "NetBoxManufacturer" {
            let error_msg = format!("Invalid kind '{}' for manufacturer reference in device type {}, expected 'NetBoxManufacturer'", device_type_crd.spec.manufacturer.kind, name);
            error!("{}", error_msg);
            return Err(ControllerError::InvalidConfig(error_msg));
        }
        
        // Resolve manufacturer ID first (required for drift detection)
        let manufacturer_id = match self.netbox_manufacturer_api.get(&device_type_crd.spec.manufacturer.name).await {
            Ok(mfg_crd) => {
                match mfg_crd.status
                    .as_ref()
                    .and_then(|s| s.netbox_id) {
                    Some(id) => id,
                    None => {
                        // Manufacturer CR exists but not yet created in NetBox - retry later
                        let error_msg = format!(
                            "Manufacturer '{}' has not been created in NetBox yet (will retry)",
                            device_type_crd.spec.manufacturer.name
                        );
                        warn!("{}", error_msg);
                        return Err(ControllerError::InvalidConfig(error_msg));
                    }
                }
            }
            Err(_) => {
                let error_msg = format!("Manufacturer CRD '{}' not found", device_type_crd.spec.manufacturer.name);
                error!("{}", error_msg);
                return Err(ControllerError::InvalidConfig(error_msg));
            }
        };
        
        // Check if already created - use helper for drift detection
        // Note: DeviceType requires manufacturer_id to query, so we check after resolving it
        let netbox_device_type = if let Some(status) = &device_type_crd.status {
            if status.state == ResourceState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    // Query by manufacturer and model, then check if ID matches
                    match self.netbox_client.get_device_type_by_model(
                        manufacturer_id,
                        &device_type_crd.spec.model,
                    ).await {
                        Ok(Some(dt)) if dt.id == netbox_id => {
                            // Resource exists and matches ID
                            Some(dt)
                        }
                        _ => {
                            // Resource not found or ID mismatch - drift detected
                            warn!("NetBoxDeviceType {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
                            let status_patch = Self::create_resource_status_patch(
                                0, // Clear netbox_id
                                String::new(), // Clear URL
                                ResourceState::Pending,
                                Some("Resource was deleted in NetBox, will recreate".to_string()),
                            );
                            let pp = kube::api::PatchParams::default();
                            if let Err(e) = self.netbox_device_type_api
                                .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                                .await
                            {
                                warn!("Failed to clear NetBoxDeviceType status after drift detection: {}", e);
                            }
                            // Fall through to creation
                            None
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
        
        // Handle existing device type (from helper) or create new
        let netbox_device_type = match netbox_device_type {
            Some(device_type) => {
                // Resource exists and is up-to-date - only update status if it changed
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
                        .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxDeviceType {}/{} status: NetBox ID {}", namespace, name, device_type.id);
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update NetBoxDeviceType status: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxDeviceType {}/{} already has correct status (ID: {}), skipping update", namespace, name, device_type.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create device type - try to find existing by manufacturer and model (idempotency fallback)
                let existing_device_type = match self.netbox_client.get_device_type_by_model(
                    manufacturer_id,
                    &device_type_crd.spec.model,
                ).await {
                    Ok(Some(dt)) => Some(dt),
                    Ok(None) => None,
                    Err(_) => None
                };
                
                if let Some(existing) = existing_device_type {
                    info!("Device type {} already exists in NetBox (ID: {})", device_type_crd.spec.model, existing.id);
                    existing
                } else {
                    match self.netbox_client.create_device_type(
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
                            return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                        }
                    }
                }
            }
        };
        
        // Update status (use lowercase state to match CRD validation schema)
        let status_patch = Self::create_resource_status_patch(
            netbox_device_type.id,
            netbox_device_type.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_device_type_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
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
