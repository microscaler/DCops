//! NetBoxPlatform reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxPlatform, ResourceState};
use netbox_client::{NetBoxClientTrait, ManufacturerId};

impl Reconciler {
    pub async fn reconcile_netbox_platform(&self, platform_crd: &NetBoxPlatform) -> Result<(), ControllerError> {
        let name = platform_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxPlatform missing name".to_string()))?;
        let namespace = platform_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxPlatform {}/{}", namespace, name);
        
        // Get client for shared resource (finds tenant from referencing Devices)
        let netbox_client = self.token_resolver
            .create_client_for_shared_resource(namespace, "NetBoxPlatform", name)
            .await
            .map_err(|e| ControllerError::TokenResolution(e))?;
        
        // Resolve manufacturer ID if provided
        let manufacturer_id = if let Some(manufacturer_ref) = &platform_crd.spec.manufacturer {
            if manufacturer_ref.kind != "NetBoxManufacturer" {
                warn!("Invalid kind '{}' for manufacturer reference in platform {}, expected 'NetBoxManufacturer'", manufacturer_ref.kind, name);
                None
            } else {
                match self.netbox_manufacturer_api.get(&manufacturer_ref.name).await {
                    Ok(manufacturer_crd) => {
                        manufacturer_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                    }
                    Err(_) => {
                        warn!("Manufacturer CRD '{}' not found for platform {}, skipping manufacturer reference", manufacturer_ref.name, name);
                        None
                    }
                }
            }
        } else {
            None
        };
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                platform_crd.status.as_ref(),
                "NetBoxPlatform",
                namespace,
                name,
                |netbox_id| async move {
                    let id_str = netbox_id.to_string();
                    netbox_client_ref.query_platforms(&[("id", &id_str)], false)
                        .await
                        .and_then(|mut platforms| {
                            platforms.pop().ok_or_else(|| netbox_client::NetBoxError::NotFound(format!("Platform {} not found", netbox_id)))
                        })
                },
            ).await?
        };
        
        let netbox_platform = match drift_result {
            DriftCheckResult::UseExisting(platform) => Some(platform),
            DriftCheckResult::StatusCleared { message } => {
                let status_patch = Self::create_typed_platform_status_patch(
                    0, String::new(), ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_platform_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxPlatform status: {}", update_err);
                }
                None
            }
            DriftCheckResult::Recreate => None,
        };
        
        let netbox_platform = match netbox_platform {
            Some(platform) => {
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    platform_crd.status.as_ref(),
                    platform.id,
                    &platform.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_typed_platform_status_patch(
                        platform.id,
                        platform.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_platform_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxPlatform {}/{} status: NetBox ID {}", namespace, name, platform.id);
                            return Ok(());
                        }
                        Err(e) => {
                            error!("Failed to update NetBoxPlatform status: {}", e);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxPlatform {}/{} already has correct status (ID: {}), skipping update", namespace, name, platform.id);
                    return Ok(());
                }
            }
            None => {
                let existing_platform = match netbox_client.get_platform_by_name(&platform_crd.spec.name).await {
                    Ok(Some(p)) => {
                        info!("Platform {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", platform_crd.spec.name, p.id);
                        Some(p)
                    }
                    Ok(None) => None,
                    Err(e) => {
                        warn!("Failed to query platform by name: {}, will try to create", e);
                        None
                    }
                };
                
                if let Some(existing) = existing_platform {
                    existing
                } else {
                    info!("Creating platform {} in NetBox", platform_crd.spec.name);
                    match netbox_client.create_platform(
                        &platform_crd.spec.name,
                        platform_crd.spec.slug.as_deref(),
                        manufacturer_id.map(ManufacturerId),
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
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                }
            }
        };
        
        let status_patch = Self::create_typed_platform_status_patch(
            netbox_platform.id,
            netbox_platform.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_platform_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
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
