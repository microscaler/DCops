//! NetBoxRIR reconciler
//!
//! Handles reconciliation of NetBox RIR (Regional Internet Registry) resources.

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers::{extract_name_and_namespace, validate_status_and_drift, DriftCheckResult, status_needs_update, update_resource_status, is_conflict_error};
use tracing::{info, error, debug, warn};
use crds::{NetBoxRIR, ResourceState};

impl Reconciler {
    fn rir_needs_update(
        spec: &crds::NetBoxRIRSpec,
        existing: &netbox_client::Rir,
    ) -> bool {
        use crate::reconcile_helpers::{
            compare_string_field,
            compare_slug_field,
            compare_optional_string_field,
        };
        
        let auto_generated_slug = spec.name.to_lowercase().replace(' ', "-");
        let spec_is_private = spec.is_private.unwrap_or(false);
        
        compare_string_field(&spec.name, &existing.name)
            || compare_slug_field(&spec.slug, &existing.slug, auto_generated_slug)
            || compare_optional_string_field(&spec.description, &existing.description)
            || compare_optional_string_field(&spec.comments, &existing.comments)
            || spec_is_private != existing.is_private
        // Tags are handled separately
    }

    /// Reconciles a NetBoxRIR resource.
    pub async fn reconcile_netbox_rir(&self, rir_crd: &NetBoxRIR) -> Result<(), ControllerError> {
        let (name, namespace) = extract_name_and_namespace(rir_crd, "NetBoxRIR")?;
        
        info!("Reconciling NetBoxRIR {}/{}", namespace, name);
        
        // Get client for shared resource (RIRs are shared resources)
        let netbox_client = self.token_resolver
            .create_client_for_shared_resource(namespace, "NetBoxRIR", name)
            .await
            .map_err(|e| ControllerError::TokenResolution(e))?;
        
        // Check if already created - use shared helper for drift detection and status validation
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                rir_crd.status.as_ref(),
                "NetBoxRIR",
                namespace,
                name,
                |netbox_id: u64| async move {
                    let id_str = netbox_id.to_string();
                    netbox_client_ref.query_rirs(&[("id", &id_str)], false)
                        .await
                        .and_then(|mut rirs| {
                            rirs.pop().ok_or_else(|| netbox_client::NetBoxError::NotFound(format!("RIR {} not found", netbox_id)))
                        })
                },
            ).await?
        };
        
        // Check if drift detection is enabled (defaults to true)
        let drift_detection_enabled = rir_crd.spec.drift_detection.unwrap_or(true);
        
        let netbox_rir = match drift_result {
            DriftCheckResult::UseExisting(rir) => {
                // Check for field drift if enabled
                if drift_detection_enabled {
                    if Self::rir_needs_update(&rir_crd.spec, &rir) {
                        // Field drift detected - update NetBox to match CRD (Git is source of truth)
                        warn!("NetBoxRIR {}/{} fields differ from CRD spec, updating to match Git", namespace, name);
                        use crate::events::reasons;
                        self.record_event_warning(
                            reasons::DRIFT_DETECTED,
                            &format!("NetBoxRIR {}/{} fields differ from CRD, updating to match Git", namespace, name),
                            rir_crd,
                        ).await;
                        
                        // Resolve tags for update
                        let resolved_tags_json = self.resolve_tag_references(
                            netbox_client.as_ref(),
                            &rir_crd.spec.tags,
                            namespace,
                            name,
                            Some(rir.id),
                        ).await;
                        let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                        
                        use netbox_client::RirId;
                        match netbox_client.update_rir(
                            RirId(rir.id),
                            Some(&rir_crd.spec.name),
                            rir_crd.spec.slug.as_deref(),
                            rir_crd.spec.description.clone(),
                            rir_crd.spec.comments.clone(),
                            rir_crd.spec.is_private,
                            resolved_tags,
                        ).await {
                            Ok(updated) => {
                                use crate::events::reasons;
                                self.record_event_normal(
                                    reasons::UPDATED,
                                    &format!("Updated NetBoxRIR {}/{} in NetBox to match CRD (ID: {})", namespace, name, updated.id),
                                    rir_crd,
                                ).await;
                                Some(updated)
                            }
                            Err(e) => {
                                error!("Failed to update NetBoxRIR {}/{} in NetBox: {}", namespace, name, e);
                                Some(rir) // Use existing if update fails
                            }
                        }
                    } else {
                        // No drift - use existing
                        Some(rir)
                    }
                } else {
                    // Drift detection disabled - use existing
                    Some(rir)
                }
            }
            DriftCheckResult::StatusCleared { message } => {
                // Emit event for drift detection
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DRIFT_DETECTED,
                    &format!("NetBoxRIR {}/{} drift detected: {}", namespace, name, message),
                    rir_crd,
                ).await;
                
                let status_patch = Self::create_typed_rir_status_patch(
                    0, String::new(), ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_rir_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxRIR status: {}", update_err);
                }
                None
            }
            DriftCheckResult::Recreate => None,
        };
        
        let netbox_rir = match netbox_rir {
            Some(rir) => {
                // Always resolve tags (even if nothing else changed, tags might need updating)
                let resolved_tags_json = self.resolve_tag_references(
                    netbox_client.as_ref(),
                    &rir_crd.spec.tags,
                    namespace,
                    name,
                None,
                ).await;
                
                // Convert resolved tags from Vec<serde_json::Value> to Vec<String>
                let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                
                // Update tags if they differ
                use netbox_client::RirId;
                let rir_id = rir.id;
                let rir_clone = rir.clone();
                let rir = match crate::reconcile_helpers::update_tags_if_differ(
                    rir,
                    &rir_crd.spec.tags,
                    resolved_tags.clone(),
                    |tags| async move {
                        netbox_client.update_rir(
                            RirId(rir_id),
                            Some(&rir_crd.spec.name),
                            rir_crd.spec.slug.as_deref(),
                            rir_crd.spec.description.clone(),
                            rir_crd.spec.comments.clone(),
                            rir_crd.spec.is_private,
                            tags,
                        ).await
                    },
                    &format!("NetBoxRIR {}/{}", namespace, name),
                ).await {
                    Ok(Some(updated)) => {
                        use crate::events::reasons;
                        self.record_event_normal(
                            reasons::UPDATED,
                            &format!("Updated NetBoxRIR {}/{} tags in NetBox", namespace, name),
                            rir_crd,
                        ).await;
                        updated
                    }
                    Ok(None) => rir_clone, // Tags are up-to-date
                    Err(e) => {
                        warn!("Failed to update NetBoxRIR {}/{} tags: {}", namespace, name, e);
                        rir_clone // Use existing if update fails
                    }
                };
                
                // Check if status needs updating
                let needs_status_update = status_needs_update(
                    rir_crd.status.as_ref(),
                    rir.id,
                    &rir.url,
                    "Created",
                    None,
                );
                
                rir // Return existing RIR (status update happens at end)
            }
            None => {
                // Try to find existing RIR by name or slug
                let existing_rir = match netbox_client.get_rir_by_name(&rir_crd.spec.name).await {
                    Ok(Some(rir)) => Some(rir),
                    Ok(None) => None,
                    Err(e) => {
                        warn!("Failed to query RIR by name: {}, will try to create", e);
                        None
                    }
                };
                
                if let Some(r) = existing_rir.as_ref() {
                    info!("RIR {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", rir_crd.spec.name, r.id);
                }
                
                if let Some(existing) = existing_rir {
                    // Resource exists but no status - check if tags need updating
                    let resolved_tags_json = self.resolve_tag_references(
                        netbox_client.as_ref(),
                        &rir_crd.spec.tags,
                        namespace,
                        name,
                    None,
                ).await;
                    let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                    
                    // Update tags if they differ
                    use netbox_client::RirId;
                    let existing_id = existing.id;
                    let existing_clone = existing.clone();
                    match crate::reconcile_helpers::update_tags_if_differ(
                        existing,
                        &rir_crd.spec.tags,
                        resolved_tags,
                        |tags| async move {
                            netbox_client.update_rir(
                                RirId(existing_id),
                                Some(&rir_crd.spec.name),
                                rir_crd.spec.slug.as_deref(),
                                rir_crd.spec.description.clone(),
                                rir_crd.spec.comments.clone(),
                                rir_crd.spec.is_private,
                                tags,
                            ).await
                        },
                        &format!("NetBoxRIR {}/{} (idempotency path)", namespace, name),
                    ).await {
                        Ok(Some(updated)) => {
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::UPDATED,
                                &format!("Updated NetBoxRIR {}/{} tags in NetBox", namespace, name),
                                rir_crd,
                            ).await;
                            updated
                        }
                        Ok(None) => existing_clone, // Tags are up-to-date
                        Err(e) => {
                            warn!("Failed to update NetBoxRIR {}/{} tags: {}", namespace, name, e);
                            existing_clone // Use existing if update fails
                        }
                    }
                } else {
                    // Resolve tags before create
                    let resolved_tags_json = self.resolve_tag_references(
                        netbox_client.as_ref(),
                        &rir_crd.spec.tags,
                        namespace,
                        name,
                    None,
                ).await;
                    let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                    
                    debug!("Attempting to create RIR {} in NetBox", rir_crd.spec.name);
                    match netbox_client.create_rir(
                        &rir_crd.spec.name,
                        rir_crd.spec.slug.as_deref(),
                        rir_crd.spec.description.clone(),
                        rir_crd.spec.comments.clone(),
                        rir_crd.spec.is_private,
                        resolved_tags,
                    ).await {
                        Ok(created) => {
                            info!("Created RIR {} in NetBox (ID: {})", created.name, created.id);
                            // Emit event for successful creation
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::CREATED,
                                &format!("Created RIR {} in NetBox (ID: {})", created.name, created.id),
                                rir_crd,
                            ).await;
                            created
                        }
                        Err(e) => {
                            if is_conflict_error(&e) {
                                warn!("RIR {} creation conflicted, attempting idempotent lookup", rir_crd.spec.name);

                                // Strategy 1: by name
                                let mut found_rir = match netbox_client.get_rir_by_name(&rir_crd.spec.name).await {
                                    Ok(Some(rir)) => Some(rir),
                                    _ => None,
                                };

                                // Strategy 2: by slug if provided
                                if found_rir.is_none() {
                                    if let Some(slug) = &rir_crd.spec.slug {
                                        if let Ok(Some(rir)) = netbox_client.get_rir_by_name(slug).await {
                                            info!("Found existing RIR by slug '{}' in NetBox (ID: {}) after conflict", slug, rir.id);
                                            found_rir = Some(rir);
                                        }
                                    }
                                }

                                // Strategy 3: fallback query all and filter
                                if found_rir.is_none() {
                                    if let Ok(all_rirs) = netbox_client.query_rirs(&[], true).await {
                                        if let Some(rir) = all_rirs.iter().find(|r| {
                                            let slug_match = rir_crd
                                                .spec
                                                .slug
                                                .as_ref()
                                                .map(|spec_slug| r.slug == *spec_slug)
                                                .unwrap_or(false);
                                            r.name == rir_crd.spec.name || slug_match
                                        }) {
                                            info!("Found existing RIR in NetBox (ID: {}) via fallback query", rir.id);
                                            found_rir = Some(rir.clone());
                                        }
                                    }
                                }

                                if let Some(found) = found_rir {
                                    found
                                } else {
                                    let error_msg = format!("RIR {} already exists in NetBox but could not retrieve it: {}", rir_crd.spec.name, e);
                                    error!("{}", error_msg);
                                    return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                                }
                            } else {
                                let error_msg = format!("Failed to create RIR in NetBox: {}", e);
                                error!("{}", error_msg);
                                // Emit event for reconciliation failure
                                use crate::events::reasons;
                                self.record_event_warning(
                                    reasons::RECONCILIATION_FAILED,
                                    &error_msg,
                                    rir_crd,
                                ).await;
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    }
                }
            }
        };
        
        // Update status using helper
        let status_patch = Self::create_typed_rir_status_patch(
            netbox_rir.id,
            netbox_rir.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_rir_api,
            name,
            namespace,
            &status_patch,
            "NetBoxRIR",
            netbox_rir.id,
        ).await?;
        info!("Updated NetBoxRIR {}/{} status: NetBox ID {}", namespace, name, netbox_rir.id);
        Ok(())
    }
}

