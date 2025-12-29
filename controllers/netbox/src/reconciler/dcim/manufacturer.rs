//! NetBoxManufacturer reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxManufacturer, ResourceState};
use netbox_client::NetBoxClientTrait;

impl Reconciler {
    pub async fn reconcile_netbox_manufacturer(&self, manufacturer_crd: &NetBoxManufacturer) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::extract_name_and_namespace;
        let (name, namespace) = extract_name_and_namespace(manufacturer_crd, "NetBoxManufacturer")?;
        
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
                |netbox_id: u64| async move {
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
                // Emit event for drift detection
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DRIFT_DETECTED,
                    &format!("NetBoxManufacturer {}/{} drift detected: {}", namespace, name, message),
                    manufacturer_crd,
                ).await;
                
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
                    use crate::reconcile_helpers::update_resource_status;
                    let status_patch = Self::create_typed_manufacturer_status_patch(
                        manufacturer.id,
                        manufacturer.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    update_resource_status(
                        &*self.netbox_manufacturer_api,
                        name,
                        namespace,
                        &status_patch,
                        "NetBoxManufacturer",
                        manufacturer.id,
                    ).await?;
                    debug!("Updated NetBoxManufacturer {}/{} status: NetBox ID {}", namespace, name, manufacturer.id);
                    return Ok(());
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
                    debug!("Attempting to create manufacturer {} in NetBox", manufacturer_crd.spec.name);
                    match netbox_client.create_manufacturer(
                        &manufacturer_crd.spec.name,
                        manufacturer_crd.spec.slug.as_deref(),
                        manufacturer_crd.spec.description.clone(),
                    ).await {
                        Ok(created) => {
                            info!("Created manufacturer {} in NetBox (ID: {})", created.name, created.id);
                            // Emit event for successful creation
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::CREATED,
                                &format!("Created manufacturer {} in NetBox (ID: {})", created.name, created.id),
                                manufacturer_crd,
                            ).await;
                            created
                        }
                        Err(e) => {
                            use crate::reconcile_helpers::is_conflict_error;

                            if is_conflict_error(&e) {
                                warn!("Manufacturer {} creation conflicted, attempting idempotent lookup", manufacturer_crd.spec.name);

                                // Strategy 1: by name
                                let mut found_manufacturer = match netbox_client.get_manufacturer_by_name(&manufacturer_crd.spec.name).await {
                                    Ok(Some(m)) => Some(m),
                                    _ => None,
                                };

                                // Strategy 2: by slug if not found
                                if found_manufacturer.is_none() {
                                    if let Some(slug) = &manufacturer_crd.spec.slug {
                                        if let Ok(manufacturers) = netbox_client.query_manufacturers(&[("slug", slug)], false).await {
                                            if let Some(m) = manufacturers.first() {
                                                info!("Found existing manufacturer by slug '{}' in NetBox (ID: {}) after conflict", slug, m.id);
                                                found_manufacturer = Some(m.clone());
                                            }
                                        }
                                    }
                                }

                                // Strategy 3: fallback query all and filter
                                if found_manufacturer.is_none() {
                                    if let Ok(all_manufacturers) = netbox_client.query_manufacturers(&[], true).await {
                                        if let Some(m) = all_manufacturers.iter().find(|m| {
                                            let slug_match = manufacturer_crd
                                                .spec
                                                .slug
                                                .as_ref()
                                                .map(|spec_slug| m.slug == *spec_slug)
                                                .unwrap_or(false);
                                            m.name == manufacturer_crd.spec.name || slug_match
                                        }) {
                                            info!("Found existing manufacturer in NetBox (ID: {}) via fallback query", m.id);
                                            found_manufacturer = Some(m.clone());
                                        }
                                    }
                                }

                                if let Some(found) = found_manufacturer {
                                    found
                                } else {
                                    let error_msg = format!("Manufacturer {} already exists in NetBox but could not retrieve it: {}", manufacturer_crd.spec.name, e);
                                    error!("{}", error_msg);
                                    return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                                }
                            } else {
                                let error_msg = format!("Failed to create manufacturer in NetBox: {}", e);
                                error!("{}", error_msg);
                                // Emit event for reconciliation failure
                                use crate::events::reasons;
                                self.record_event_warning(
                                    reasons::RECONCILIATION_FAILED,
                                    &error_msg,
                                    manufacturer_crd,
                                ).await;
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    }
                }
            }
        };
        
        use crate::reconcile_helpers::update_resource_status;
        let status_patch = Self::create_typed_manufacturer_status_patch(
            netbox_manufacturer.id,
            netbox_manufacturer.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_manufacturer_api,
            name,
            namespace,
            &status_patch,
            "NetBoxManufacturer",
            netbox_manufacturer.id,
        ).await?;
        info!("Updated NetBoxManufacturer {}/{} status: NetBox ID {}", namespace, name, netbox_manufacturer.id);
        Ok(())
    }
}
