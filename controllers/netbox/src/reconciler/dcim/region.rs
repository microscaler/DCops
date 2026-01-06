//! NetBoxRegion reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxRegion, ResourceState};
use netbox_client::RegionId;

impl Reconciler {
    fn region_needs_update(
        spec: &crds::NetBoxRegionSpec,
        existing: &netbox_client::Region,
        desired_parent_id: Option<u64>,
    ) -> bool {
        use crate::reconcile_helpers::{
            compare_string_field,
            compare_slug_field,
            compare_optional_string_field,
            compare_optional_dependency_id,
        };
        
        let auto_generated_slug = spec.name.to_lowercase().replace(' ', "-");
        let existing_parent_id = existing.parent.as_ref().map(|p| p.id);
        
        // Evaluate all comparisons to log all field differences (no short-circuit)
        let name_diff = compare_string_field(&spec.name, &existing.name);
        let slug_diff = compare_slug_field(&spec.slug, &existing.slug, auto_generated_slug);
        let parent_diff = compare_optional_dependency_id(desired_parent_id, existing_parent_id);
        let description_diff = compare_optional_string_field(&spec.description, &existing.description);
        let comments_diff = compare_optional_string_field(&spec.comments, &existing.comments);
        // Tags are handled separately
        
        name_diff || slug_diff || parent_diff || description_diff || comments_diff
    }

    pub async fn reconcile_netbox_region(&self, region_crd: &NetBoxRegion) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::{extract_name_and_namespace, resolve_optional_dependency_id};
        let (name, namespace) = extract_name_and_namespace(region_crd, "NetBoxRegion")?;
        
        info!("Reconciling NetBoxRegion {}/{}", namespace, name);
        
        // Get client for shared resource (finds tenant from referencing Sites)
        let netbox_client = self.token_resolver
            .create_client_for_shared_resource(namespace, "NetBoxRegion", name)
            .await
            .map_err(|e| ControllerError::TokenResolution(e))?;
        
        // Resolve optional parent region ID using helper
        let parent_id: Option<u64> = resolve_optional_dependency_id(
            &*self.netbox_region_api,
            region_crd.spec.parent.as_ref(),
            "NetBoxRegion",
            "parent",
            name,
            |crd| crd.status.as_ref(),
        ).await;
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                region_crd.status.as_ref(),
                "NetBoxRegion",
                namespace,
                name,
                |netbox_id: u64| async move {
                    netbox_client_ref.get_region(RegionId(netbox_id)).await
                },
            ).await?
        };
        
        // Check if drift detection is enabled (defaults to true)
        let drift_detection_enabled = region_crd.spec.drift_detection.unwrap_or(true);
        
        let netbox_region = match drift_result {
            DriftCheckResult::UseExisting(region) => {
                // Check for field drift if enabled
                if drift_detection_enabled {
                    if Self::region_needs_update(&region_crd.spec, &region, parent_id) {
                        // Field drift detected - update NetBox to match CRD (Git is source of truth)
                        warn!("NetBoxRegion {}/{} fields differ from CRD spec, updating to match Git", namespace, name);
                        use crate::events::reasons;
                        self.record_event_warning(
                            reasons::DRIFT_DETECTED,
                            &format!("NetBoxRegion {}/{} fields differ from CRD, updating to match Git", namespace, name),
                            region_crd,
                        ).await;
                        
                        // Resolve tags for update
                        let resolved_tags_json = self.resolve_tag_references(
                            netbox_client.as_ref(),
                            &region_crd.spec.tags,
                            namespace,
                            name,
                            Some(region.id),
                            "NetBoxRegion",
                        ).await;
                        let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                        
                        match netbox_client.update_region(
                            RegionId(region.id),
                            Some(&region_crd.spec.name),
                            region_crd.spec.slug.as_deref(),
                            parent_id.map(RegionId),
                            region_crd.spec.description.clone(),
                            region_crd.spec.comments.clone(),
                            resolved_tags,
                        ).await {
                            Ok(updated) => {
                                use crate::events::reasons;
                                self.record_event_normal(
                                    reasons::UPDATED,
                                    &format!("Updated NetBoxRegion {}/{} in NetBox to match CRD (ID: {})", namespace, name, updated.id),
                                    region_crd,
                                ).await;
                                Some(updated)
                            }
                            Err(e) => {
                                error!("Failed to update NetBoxRegion {}/{} in NetBox: {}", namespace, name, e);
                                Some(region) // Use existing if update fails
                            }
                        }
                    } else {
                        // No drift - use existing
                        Some(region)
                    }
                } else {
                    // Drift detection disabled - use existing
                    Some(region)
                }
            }
            DriftCheckResult::StatusCleared { message } => {
                // Emit event for drift detection
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DRIFT_DETECTED,
                    &format!("NetBoxRegion {}/{} drift detected: {}", namespace, name, message),
                    region_crd,
                ).await;
                
                // Status was cleared - update it to Pending
                let status_patch = Self::create_typed_region_status_patch(
                    0, // Clear netbox_id
                    String::new(), // Clear URL
                    ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_region_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxRegion status: {}", update_err);
                }
                // Fall through to creation
                None
            }
            DriftCheckResult::Recreate => {
                // Need to create - fall through
                None
            }
        };
        
        // Handle existing region (from helper) or create new
        let netbox_region = match netbox_region {
            Some(region) => {
                // Always resolve tags (even if nothing else changed, tags might need updating)
                let resolved_tags_json = self.resolve_tag_references(
                    netbox_client.as_ref(),
                    &region_crd.spec.tags,
                    namespace,
                    name,
                    None,
                    "NetBoxRegion",
                ).await;
                
                // Convert resolved tags from Vec<serde_json::Value> to Vec<String>
                let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                
                // Update tags if they differ
                let region_id = region.id;
                let region_clone = region.clone();
                let region = match crate::reconcile_helpers::update_tags_if_differ(
                    region,
                    &region_crd.spec.tags,
                    resolved_tags.clone(),
                    |tags| async move {
                        netbox_client.update_region(
                            RegionId(region_id),
                            Some(&region_crd.spec.name),
                            region_crd.spec.slug.as_deref(),
                            parent_id.map(RegionId),
                            region_crd.spec.description.clone(),
                            region_crd.spec.comments.clone(),
                            tags,
                        ).await
                    },
                    &format!("NetBoxRegion {}/{}", namespace, name),
                ).await {
                    Ok(Some(updated)) => {
                        use crate::events::reasons;
                        self.record_event_normal(
                            reasons::UPDATED,
                            &format!("Updated NetBoxRegion {}/{} tags in NetBox", namespace, name),
                            region_crd,
                        ).await;
                        updated
                    }
                    Ok(None) => region_clone, // Tags are up-to-date
                    Err(e) => {
                        warn!("Failed to update NetBoxRegion {}/{} tags: {}", namespace, name, e);
                        region_clone // Use existing if update fails
                    }
                };
                
                // Check if status needs updating
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    region_crd.status.as_ref(),
                    region.id,
                    &region.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    use crate::reconcile_helpers::update_resource_status;
                    let status_patch = Self::create_typed_region_status_patch(
                        region.id,
                        region.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    update_resource_status(
                        &*self.netbox_region_api,
                        name,
                        namespace,
                        &status_patch,
                        "NetBoxRegion",
                        region.id,
                    ).await?;
                    debug!("Updated NetBoxRegion {}/{} status: NetBox ID {}", namespace, name, region.id);
                } else {
                    debug!("NetBoxRegion {}/{} already has correct status (ID: {}), skipping update", namespace, name, region.id);
                }
                region // Return existing region
            }
            None => {
                // Need to create region - try to find existing by name (idempotency fallback)
                let existing_region = match netbox_client.get_region_by_name(&region_crd.spec.name).await {
                    Ok(Some(region)) => {
                        info!("Region {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", region_crd.spec.name, region.id);
                        Some(region)
                    }
                    Ok(None) => None,
                    Err(e) => {
                        warn!("Failed to query region by name: {}, will try to create", e);
                        None
                    }
                };
                
                if let Some(existing) = existing_region {
                    // Resource exists but no status - check if tags need updating
                    let resolved_tags_json = self.resolve_tag_references(
                        netbox_client.as_ref(),
                        &region_crd.spec.tags,
                        namespace,
                        name,
                        None,
                        "NetBoxRegion",
                    ).await;
                    let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                    
                    // Update tags if they differ
                    let existing_id = existing.id;
                    let existing_clone = existing.clone();
                    match crate::reconcile_helpers::update_tags_if_differ(
                        existing,
                        &region_crd.spec.tags,
                        resolved_tags,
                        |tags| async move {
                            netbox_client.update_region(
                                RegionId(existing_id),
                                Some(&region_crd.spec.name),
                                region_crd.spec.slug.as_deref(),
                                parent_id.map(RegionId),
                                region_crd.spec.description.clone(),
                                region_crd.spec.comments.clone(),
                                tags,
                            ).await
                        },
                        &format!("NetBoxRegion {}/{} (idempotency path)", namespace, name),
                    ).await {
                        Ok(Some(updated)) => {
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::UPDATED,
                                &format!("Updated NetBoxRegion {}/{} tags in NetBox", namespace, name),
                                region_crd,
                            ).await;
                            updated
                        }
                        Ok(None) => existing_clone, // Tags are up-to-date
                        Err(e) => {
                            warn!("Failed to update NetBoxRegion {}/{} tags: {}", namespace, name, e);
                            existing_clone // Use existing if update fails
                        }
                    }
                } else {
                    // Resolve tags before creation
                    let resolved_tags_json = self.resolve_tag_references(
                        netbox_client.as_ref(),
                        &region_crd.spec.tags,
                        namespace,
                        name,
                        None,
                        "NetBoxRegion",
                    ).await;
                    
                    let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                    
                    // Create region
                    debug!("Attempting to create region {} in NetBox", region_crd.spec.name);
                    match netbox_client.create_region(
                        &region_crd.spec.name,
                        region_crd.spec.slug.as_deref(),
                        parent_id.map(RegionId),
                        region_crd.spec.description.clone(),
                        region_crd.spec.comments.clone(),
                        resolved_tags,
                    ).await {
                        Ok(created) => {
                            info!("Created region {} in NetBox (ID: {})", created.name, created.id);
                            // Emit event for successful creation
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::CREATED,
                                &format!("Created region {} in NetBox (ID: {})", created.name, created.id),
                                region_crd,
                            ).await;
                            created
                        }
                        Err(e) => {
                            use crate::reconcile_helpers::is_conflict_error;

                            if is_conflict_error(&e) {
                                warn!("Region {} creation conflicted, attempting idempotent lookup", region_crd.spec.name);

                                // Strategy 1: by name
                                let mut found_region = match netbox_client.get_region_by_name(&region_crd.spec.name).await {
                                    Ok(Some(r)) => Some(r),
                                    _ => None,
                                };

                                // Strategy 2: by slug if provided
                                if found_region.is_none() {
                                    if let Some(slug) = &region_crd.spec.slug {
                                        if let Ok(regions) = netbox_client.query_regions(&[("slug", slug)], false).await {
                                            if let Some(r) = regions.first() {
                                                info!("Found existing region by slug '{}' in NetBox (ID: {}) after conflict", slug, r.id);
                                                found_region = Some(r.clone());
                                            }
                                        }
                                    }
                                }

                                // Strategy 3: fallback query all and filter
                                if found_region.is_none() {
                                    if let Ok(all_regions) = netbox_client.query_regions(&[], true).await {
                                        if let Some(r) = all_regions.iter().find(|r| {
                                            let slug_match = region_crd
                                                .spec
                                                .slug
                                                .as_ref()
                                                .map(|spec_slug| r.slug == *spec_slug)
                                                .unwrap_or(false);
                                            r.name == region_crd.spec.name || slug_match
                                        }) {
                                            info!("Found existing region in NetBox (ID: {}) via fallback query", r.id);
                                            found_region = Some(r.clone());
                                        }
                                    }
                                }

                                if let Some(found) = found_region {
                                    found
                                } else {
                                    let error_msg = format!("Region {} already exists in NetBox but could not retrieve it: {}", region_crd.spec.name, e);
                                    error!("{}", error_msg);
                                    return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                                }
                            } else {
                                let error_msg = format!("Failed to create region in NetBox: {}", e);
                                error!("{}", error_msg);
                                // Emit event for reconciliation failure
                                use crate::events::reasons;
                                self.record_event_warning(
                                    reasons::RECONCILIATION_FAILED,
                                    &error_msg,
                                    region_crd,
                                ).await;
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    }
                }
            }
        };
        
        // Update status using helper
        use crate::reconcile_helpers::update_resource_status;
        let status_patch = Self::create_typed_region_status_patch(
            netbox_region.id,
            netbox_region.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_region_api,
            name,
            namespace,
            &status_patch,
            "NetBoxRegion",
            netbox_region.id,
        ).await?;
        info!("Updated NetBoxRegion {}/{} status: NetBox ID {}", namespace, name, netbox_region.id);
        Ok(())
    }
}
