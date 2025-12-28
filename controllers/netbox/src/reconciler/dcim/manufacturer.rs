//! NetBoxManufacturer reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxManufacturer, ResourceState};
use netbox_client::NetBoxClientTrait;

impl Reconciler {
    pub async fn reconcile_netbox_manufacturer(&self, manufacturer_crd: &NetBoxManufacturer) -> Result<(), ControllerError> {
        let name = manufacturer_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxManufacturer missing name".to_string()))?;
        let namespace = manufacturer_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxManufacturer {}/{}", namespace, name);
        
        // Get client for shared resource (finds tenant from referencing Devices via DeviceType)
        let netbox_client = self.token_resolver
            .create_client_for_shared_resource(namespace, "NetBoxManufacturer", name)
            .await
            .map_err(|e| ControllerError::TokenResolution(e))?;
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                manufacturer_crd.status.as_ref(),
                "NetBoxManufacturer",
                namespace,
                name,
                |netbox_id| async move {
                    let id_str = netbox_id.to_string();
                    netbox_client_ref.query_manufacturers(&[("id", &id_str)], false)
                        .await
                        .and_then(|mut manufacturers| {
                            manufacturers.pop().ok_or_else(|| netbox_client::NetBoxError::NotFound(format!("Manufacturer {} not found", netbox_id)))
                        })
                },
            ).await?
        };
        
        let netbox_manufacturer = match drift_result {
            DriftCheckResult::UseExisting(manufacturer) => Some(manufacturer),
            DriftCheckResult::StatusCleared { message } => {
                let status_patch = Self::create_typed_manufacturer_status_patch(
                    0, String::new(), ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_manufacturer_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxManufacturer status: {}", update_err);
                }
                None
            }
            DriftCheckResult::Recreate => None,
        };
        
        let netbox_manufacturer = match netbox_manufacturer {
            Some(manufacturer) => {
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    manufacturer_crd.status.as_ref(),
                    manufacturer.id,
                    &manufacturer.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_typed_manufacturer_status_patch(
                        manufacturer.id,
                        manufacturer.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_manufacturer_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxManufacturer {}/{} status: NetBox ID {}", namespace, name, manufacturer.id);
                            return Ok(());
                        }
                        Err(e) => {
                            error!("Failed to update NetBoxManufacturer status: {}", e);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxManufacturer {}/{} already has correct status (ID: {}), skipping update", namespace, name, manufacturer.id);
                    return Ok(());
                }
            }
            None => {
                let existing_manufacturer = match netbox_client.get_manufacturer_by_name(&manufacturer_crd.spec.name).await {
                    Ok(Some(m)) => {
                        info!("Manufacturer {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", manufacturer_crd.spec.name, m.id);
                        Some(m)
                    }
                    Ok(None) => None,
                    Err(e) => {
                        warn!("Failed to query manufacturer by name: {}, will try to create", e);
                        None
                    }
                };
                
                if let Some(existing) = existing_manufacturer {
                    existing
                } else {
                    info!("Creating manufacturer {} in NetBox", manufacturer_crd.spec.name);
                    match netbox_client.create_manufacturer(
                        &manufacturer_crd.spec.name,
                        manufacturer_crd.spec.slug.as_deref(),
                        manufacturer_crd.spec.description.clone(),
                    ).await {
                        Ok(created) => {
                            info!("Created manufacturer {} in NetBox (ID: {})", created.name, created.id);
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create manufacturer in NetBox: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                }
            }
        };
        
        let status_patch = Self::create_typed_manufacturer_status_patch(
            netbox_manufacturer.id,
            netbox_manufacturer.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_manufacturer_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxManufacturer {}/{} status: NetBox ID {}", namespace, name, netbox_manufacturer.id);
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
