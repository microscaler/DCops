//! NetBoxManufacturer reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxManufacturer, ResourceState};
use netbox_client::NetBoxClientTrait;

impl Reconciler {
    /// Check if Manufacturer needs updating by comparing spec with existing NetBox resource
    fn manufacturer_needs_update(
        spec: &crds::NetBoxManufacturerSpec,
        existing: &netbox_client::Manufacturer,
    ) -> bool {
        use crate::reconcile_helpers::{
            compare_string_field,
            compare_slug_field,
            compare_optional_string_field,
        };
        
        let auto_generated_slug = spec.name.to_lowercase().replace(' ', "-");
        
        compare_string_field(&spec.name, &existing.name)
            || compare_slug_field(&spec.slug, &existing.slug, auto_generated_slug)
            || compare_optional_string_field(&spec.description, &existing.description)
            || compare_optional_string_field(&spec.comments, &existing.comments)
        // Tags are handled separately
    }

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
        
        // Check if drift detection is enabled (defaults to true)
        let drift_detection_enabled = manufacturer_crd.spec.drift_detection.unwrap_or(true);
        
        let netbox_manufacturer = match drift_result {
            DriftCheckResult::UseExisting(manufacturer) => {
                // Check for field drift if enabled
                if drift_detection_enabled {
                    if Self::manufacturer_needs_update(&manufacturer_crd.spec, &manufacturer) {
                        // Field drift detected - update NetBox to match CRD (Git is source of truth)
                        warn!("NetBoxManufacturer {}/{} fields differ from CRD spec, updating to match Git", namespace, name);
                        use crate::events::reasons;
                        self.record_event_warning(
                            reasons::DRIFT_DETECTED,
                            &format!("NetBoxManufacturer {}/{} fields differ from CRD, updating to match Git", namespace, name),
                            manufacturer_crd,
                        ).await;
                        
                        // Resolve tags for update
                        let resolved_tags_json = self.resolve_tag_references(
                            netbox_client.as_ref(),
                            &manufacturer_crd.spec.tags,
                            namespace,
                            name,
                            Some(manufacturer.id),
                        ).await;
                        let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                        
                        use netbox_client::ManufacturerId;
                        match netbox_client.update_manufacturer(
                            ManufacturerId(manufacturer.id),
                            Some(&manufacturer_crd.spec.name),
                            manufacturer_crd.spec.slug.as_deref(),
                            manufacturer_crd.spec.description.clone(),
                            manufacturer_crd.spec.comments.clone(),
                            resolved_tags,
                        ).await {
                            Ok(updated) => {
                                info!("Updated NetBoxManufacturer {}/{} in NetBox to match CRD (ID: {})", namespace, name, updated.id);
                                Some(updated)
                            }
                            Err(e) => {
                                error!("Failed to update NetBoxManufacturer {}/{} in NetBox: {}", namespace, name, e);
                                use crate::events::reasons;
                                self.record_event_warning(
                                    reasons::RECONCILIATION_FAILED,
                                    &format!("Failed to update NetBoxManufacturer {}/{} in NetBox: {}", namespace, name, e),
                                    manufacturer_crd,
                                ).await;
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    } else {
                        // No field drift - use existing
                        Some(manufacturer)
                    }
                } else {
                    // Drift detection disabled - use existing without checking
                    debug!("Drift detection disabled for NetBoxManufacturer {}/{}", namespace, name);
                    Some(manufacturer)
                }
            }
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
                // Always resolve tags (even if nothing else changed, tags might need updating)
                let resolved_tags_json = self.resolve_tag_references(
                    netbox_client.as_ref(),
                    &manufacturer_crd.spec.tags,
                    namespace,
                    name,
                None,
                ).await;
                
                // Convert resolved tags from Vec<serde_json::Value> to Vec<String>
                let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                
                // Update tags if they differ
                use netbox_client::ManufacturerId;
                let manufacturer_id = manufacturer.id;
                let manufacturer_clone = manufacturer.clone();
                let manufacturer = match crate::reconcile_helpers::update_tags_if_differ(
                    manufacturer,
                    &manufacturer_crd.spec.tags,
                    resolved_tags.clone(),
                    |tags| async move {
                        netbox_client.update_manufacturer(
                            ManufacturerId(manufacturer_id),
                            Some(&manufacturer_crd.spec.name),
                            manufacturer_crd.spec.slug.as_deref(),
                            manufacturer_crd.spec.description.clone(),
                            manufacturer_crd.spec.comments.clone(),
                            tags,
                        ).await
                    },
                    &format!("NetBoxManufacturer {}/{}", namespace, name),
                ).await {
                    Ok(Some(updated)) => {
                        use crate::events::reasons;
                        self.record_event_normal(
                            reasons::UPDATED,
                            &format!("Updated NetBoxManufacturer {}/{} tags in NetBox", namespace, name),
                            manufacturer_crd,
                        ).await;
                        updated
                    }
                    Ok(None) => manufacturer_clone, // Tags are up-to-date
                    Err(e) => {
                        warn!("Failed to update NetBoxManufacturer {}/{} tags: {}", namespace, name, e);
                        manufacturer_clone // Use existing if update fails
                    }
                };
                
                // Check if status needs updating
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
                } else {
                    debug!("NetBoxManufacturer {}/{} already has correct status (ID: {}), skipping update", namespace, name, manufacturer.id);
                }
                manufacturer // Return existing manufacturer
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
                    // Resource exists but no status - check if tags need updating
                    let resolved_tags_json = self.resolve_tag_references(
                        netbox_client.as_ref(),
                        &manufacturer_crd.spec.tags,
                        namespace,
                        name,
                    None,
                ).await;
                    let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                    
                    // Update tags if they differ
                    use netbox_client::ManufacturerId;
                    let existing_id = existing.id;
                    let existing_clone = existing.clone();
                    match crate::reconcile_helpers::update_tags_if_differ(
                        existing,
                        &manufacturer_crd.spec.tags,
                        resolved_tags,
                        |tags| async move {
                            netbox_client.update_manufacturer(
                                ManufacturerId(existing_id),
                                Some(&manufacturer_crd.spec.name),
                                manufacturer_crd.spec.slug.as_deref(),
                                manufacturer_crd.spec.description.clone(),
                                manufacturer_crd.spec.comments.clone(),
                                tags,
                            ).await
                        },
                        &format!("NetBoxManufacturer {}/{} (idempotency path)", namespace, name),
                    ).await {
                        Ok(Some(updated)) => {
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::UPDATED,
                                &format!("Updated NetBoxManufacturer {}/{} tags in NetBox", namespace, name),
                                manufacturer_crd,
                            ).await;
                            updated
                        }
                        Ok(None) => existing_clone, // Tags are up-to-date
                        Err(e) => {
                            warn!("Failed to update NetBoxManufacturer {}/{} tags: {}", namespace, name, e);
                            existing_clone // Use existing if update fails
                        }
                    }
                } else {
                    // Resolve tags before create
                    let resolved_tags_json = self.resolve_tag_references(
                        netbox_client.as_ref(),
                        &manufacturer_crd.spec.tags,
                        namespace,
                        name,
                    None,
                ).await;
                    let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                    
                    debug!("Attempting to create manufacturer {} in NetBox", manufacturer_crd.spec.name);
                    match netbox_client.create_manufacturer(
                        &manufacturer_crd.spec.name,
                        manufacturer_crd.spec.slug.as_deref(),
                        manufacturer_crd.spec.description.clone(),
                        manufacturer_crd.spec.comments.clone(),
                        resolved_tags,
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
