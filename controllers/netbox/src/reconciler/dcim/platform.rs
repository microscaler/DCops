//! NetBoxPlatform reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxPlatform, ResourceState};
use netbox_client::{NetBoxClientTrait, ManufacturerId};

impl Reconciler {
    pub async fn reconcile_netbox_platform(&self, platform_crd: &NetBoxPlatform) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::{extract_name_and_namespace, resolve_optional_dependency_id};
        let (name, namespace) = extract_name_and_namespace(platform_crd, "NetBoxPlatform")?;
        
        info!("Reconciling NetBoxPlatform {}/{}", namespace, name);
        
        // Get client for shared resource (finds tenant from referencing Devices)
        let netbox_client = self.token_resolver
            .create_client_for_shared_resource(namespace, "NetBoxPlatform", name)
            .await
            .map_err(|e| ControllerError::TokenResolution(e))?;
        
        // Resolve optional manufacturer ID using helper
        let manufacturer_id: Option<u64> = resolve_optional_dependency_id(
            &*self.netbox_manufacturer_api,
            platform_crd.spec.manufacturer.as_ref(),
            "NetBoxManufacturer",
            "manufacturer",
            name,
            |crd| crd.status.as_ref(),
        ).await;
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                platform_crd.status.as_ref(),
                "NetBoxPlatform",
                namespace,
                name,
                |netbox_id: u64| async move {
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
                // Emit event for drift detection
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DRIFT_DETECTED,
                    &format!("NetBoxPlatform {}/{} drift detected: {}", namespace, name, message),
                    platform_crd,
                ).await;
                
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
                    use crate::reconcile_helpers::update_resource_status;
                    let status_patch = Self::create_typed_platform_status_patch(
                        platform.id,
                        platform.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    update_resource_status(
                        &*self.netbox_platform_api,
                        name,
                        namespace,
                        &status_patch,
                        "NetBoxPlatform",
                        platform.id,
                    ).await?;
                    debug!("Updated NetBoxPlatform {}/{} status: NetBox ID {}", namespace, name, platform.id);
                    return Ok(());
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
                    debug!("Attempting to create platform {} in NetBox", platform_crd.spec.name);
                    match netbox_client.create_platform(
                        &platform_crd.spec.name,
                        platform_crd.spec.slug.as_deref(),
                        manufacturer_id.map(ManufacturerId),
                        platform_crd.spec.napalm_driver.as_deref(),
                        platform_crd.spec.napalm_args.as_deref(),
                        platform_crd.spec.description.clone(),
                        platform_crd.spec.comments.clone(),
                        None, // tags - not yet implemented in reconciler
                    ).await {
                        Ok(created) => {
                            info!("Created platform {} in NetBox (ID: {})", created.name, created.id);
                            // Emit event for successful creation
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::CREATED,
                                &format!("Created platform {} in NetBox (ID: {})", created.name, created.id),
                                platform_crd,
                            ).await;
                            created
                        }
                        Err(e) => {
                            use crate::reconcile_helpers::is_conflict_error;

                            if is_conflict_error(&e) {
                                warn!("Platform {} creation conflicted, attempting idempotent lookup", platform_crd.spec.name);

                                // Strategy 1: by name
                                let mut found_platform = match netbox_client.get_platform_by_name(&platform_crd.spec.name).await {
                                    Ok(Some(p)) => Some(p),
                                    _ => None,
                                };

                                // Strategy 2: by slug if not found
                                if found_platform.is_none() {
                                    if let Some(slug) = &platform_crd.spec.slug {
                                        if let Ok(platforms) = netbox_client.query_platforms(&[("slug", slug)], false).await {
                                            if let Some(p) = platforms.first() {
                                                info!("Found existing platform by slug '{}' in NetBox (ID: {}) after conflict", slug, p.id);
                                                found_platform = Some(p.clone());
                                            }
                                        }
                                    }
                                }

                                // Strategy 3: fallback query all and filter
                                if found_platform.is_none() {
                                    if let Ok(all_platforms) = netbox_client.query_platforms(&[], true).await {
                                        if let Some(p) = all_platforms.iter().find(|p| {
                                            let slug_match = platform_crd
                                                .spec
                                                .slug
                                                .as_ref()
                                                .map(|spec_slug| p.slug == *spec_slug)
                                                .unwrap_or(false);
                                            p.name == platform_crd.spec.name || slug_match
                                        }) {
                                            info!("Found existing platform in NetBox (ID: {}) via fallback query", p.id);
                                            found_platform = Some(p.clone());
                                        }
                                    }
                                }

                                if let Some(found) = found_platform {
                                    found
                                } else {
                                    let error_msg = format!("Platform {} already exists in NetBox but could not retrieve it: {}", platform_crd.spec.name, e);
                                    error!("{}", error_msg);
                                    return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                                }
                            } else {
                                let error_msg = format!("Failed to create platform in NetBox: {}", e);
                                error!("{}", error_msg);
                                // Emit event for reconciliation failure
                                use crate::events::reasons;
                                self.record_event_warning(
                                    reasons::RECONCILIATION_FAILED,
                                    &error_msg,
                                    platform_crd,
                                ).await;
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    }
                }
            }
        };
        
        use crate::reconcile_helpers::update_resource_status;
        let status_patch = Self::create_typed_platform_status_patch(
            netbox_platform.id,
            netbox_platform.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_platform_api,
            name,
            namespace,
            &status_patch,
            "NetBoxPlatform",
            netbox_platform.id,
        ).await?;
        info!("Updated NetBoxPlatform {}/{} status: NetBox ID {}", namespace, name, netbox_platform.id);
        Ok(())
    }
}
