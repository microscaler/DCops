//! NetBoxDeviceType reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxDeviceType, ResourceState};
use netbox_client::{NetBoxClientTrait, ManufacturerId};

impl Reconciler {
    pub async fn reconcile_netbox_device_type(&self, device_type_crd: &NetBoxDeviceType) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::{extract_name_and_namespace, validate_reference_kind, resolve_required_dependency_id};
        let (name, namespace) = extract_name_and_namespace(device_type_crd, "NetBoxDeviceType")?;
        
        info!("Reconciling NetBoxDeviceType {}/{}", namespace, name);
        
        // Get client for shared resource (finds tenant from referencing Devices)
        let netbox_client = self.token_resolver
            .create_client_for_shared_resource(namespace, "NetBoxDeviceType", name)
            .await
            .map_err(|e| ControllerError::TokenResolution(e))?;
        
        // Validate and resolve manufacturer ID (required) using helper
        validate_reference_kind(&device_type_crd.spec.manufacturer, "NetBoxManufacturer", "manufacturer", name)?;
        let manufacturer_id = resolve_required_dependency_id(
            &*self.netbox_manufacturer_api,
            &device_type_crd.spec.manufacturer.name,
            "Manufacturer",
            name,
            |crd| crd.status.as_ref(),
        ).await?;
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                device_type_crd.status.as_ref(),
                "NetBoxDeviceType",
                namespace,
                name,
                |netbox_id| async move {
                    let id_str = netbox_id.to_string();
                    netbox_client_ref.query_device_types(&[("id", &id_str)], false)
                        .await
                        .and_then(|mut device_types| {
                            device_types.pop().ok_or_else(|| netbox_client::NetBoxError::NotFound(format!("DeviceType {} not found", netbox_id)))
                        })
                },
            ).await?
        };
        
        let netbox_device_type = match drift_result {
            DriftCheckResult::UseExisting(device_type) => Some(device_type),
            DriftCheckResult::StatusCleared { message } => {
                let status_patch = Self::create_typed_device_type_status_patch(
                    0, String::new(), ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_device_type_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxDeviceType status: {}", update_err);
                }
                None
            }
            DriftCheckResult::Recreate => None,
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
                    let status_patch = Self::create_typed_device_type_status_patch(
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
                let existing_device_type = match netbox_client.get_device_type_by_model(ManufacturerId(manufacturer_id), &device_type_crd.spec.model).await {
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
                        ManufacturerId(manufacturer_id),
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
        
        use crate::reconcile_helpers::update_resource_status;
        let status_patch = Self::create_typed_device_type_status_patch(
            netbox_device_type.id,
            netbox_device_type.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_device_type_api,
            name,
            namespace,
            &status_patch,
            "NetBoxDeviceType",
            netbox_device_type.id,
        ).await?;
        info!("Updated NetBoxDeviceType {}/{} status: NetBox ID {}", namespace, name, netbox_device_type.id);
        Ok(())
    }
}
