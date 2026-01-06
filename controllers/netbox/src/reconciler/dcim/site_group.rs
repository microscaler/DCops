//! NetBoxSiteGroup reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxSiteGroup, ResourceState};
use netbox_client::SiteGroupId;

impl Reconciler {
    fn site_group_needs_update(
        spec: &crds::NetBoxSiteGroupSpec,
        existing: &netbox_client::SiteGroup,
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

    pub async fn reconcile_netbox_site_group(&self, site_group_crd: &NetBoxSiteGroup) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::{extract_name_and_namespace, resolve_optional_dependency_id};
        let (name, namespace) = extract_name_and_namespace(site_group_crd, "NetBoxSiteGroup")?;
        
        info!("Reconciling NetBoxSiteGroup {}/{}", namespace, name);
        
        // Get client for shared resource (finds tenant from referencing Sites)
        let netbox_client = self.token_resolver
            .create_client_for_shared_resource(namespace, "NetBoxSiteGroup", name)
            .await
            .map_err(|e| ControllerError::TokenResolution(e))?;
        
        // Resolve optional parent site group ID using helper
        let parent_id: Option<u64> = resolve_optional_dependency_id(
            &*self.netbox_site_group_api,
            site_group_crd.spec.parent.as_ref(),
            "NetBoxSiteGroup",
            "parent",
            name,
            |crd| crd.status.as_ref(),
        ).await;
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                site_group_crd.status.as_ref(),
                "NetBoxSiteGroup",
                namespace,
                name,
                |netbox_id: u64| async move {
                    netbox_client_ref.get_site_group(SiteGroupId(netbox_id)).await
                },
            ).await?
        };
        
        // Check if drift detection is enabled (defaults to true)
        let drift_detection_enabled = site_group_crd.spec.drift_detection.unwrap_or(true);
        
        let netbox_site_group = match drift_result {
            DriftCheckResult::UseExisting(site_group) => {
                // Check for field drift if enabled
                if drift_detection_enabled {
                    if Self::site_group_needs_update(&site_group_crd.spec, &site_group, parent_id) {
                        // Field drift detected - update NetBox to match CRD (Git is source of truth)
                        warn!("NetBoxSiteGroup {}/{} fields differ from CRD spec, updating to match Git", namespace, name);
                        use crate::events::reasons;
                        self.record_event_warning(
                            reasons::DRIFT_DETECTED,
                            &format!("NetBoxSiteGroup {}/{} fields differ from CRD, updating to match Git", namespace, name),
                            site_group_crd,
                        ).await;
                        
                        // Resolve tags for update
                        let resolved_tags_json = self.resolve_tag_references(
                            netbox_client.as_ref(),
                            &site_group_crd.spec.tags,
                            namespace,
                            name,
                            Some(site_group.id),
                            "NetBoxSiteGroup",
                        ).await;
                        let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                        
                        match netbox_client.update_site_group(
                            SiteGroupId(site_group.id),
                            Some(&site_group_crd.spec.name),
                            site_group_crd.spec.slug.as_deref(),
                            parent_id.map(SiteGroupId),
                            site_group_crd.spec.description.clone(),
                            site_group_crd.spec.comments.clone(),
                            resolved_tags,
                        ).await {
                            Ok(updated) => {
                                use crate::events::reasons;
                                self.record_event_normal(
                                    reasons::UPDATED,
                                    &format!("Updated NetBoxSiteGroup {}/{} in NetBox to match CRD (ID: {})", namespace, name, updated.id),
                                    site_group_crd,
                                ).await;
                                Some(updated)
                            }
                            Err(e) => {
                                error!("Failed to update NetBoxSiteGroup {}/{} in NetBox: {}", namespace, name, e);
                                Some(site_group) // Use existing if update fails
                            }
                        }
                    } else {
                        // No drift - use existing
                        Some(site_group)
                    }
                } else {
                    // Drift detection disabled - use existing
                    Some(site_group)
                }
            }
            DriftCheckResult::StatusCleared { message } => {
                // Emit event for drift detection
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DRIFT_DETECTED,
                    &format!("NetBoxSiteGroup {}/{} drift detected: {}", namespace, name, message),
                    site_group_crd,
                ).await;
                
                // Status was cleared - update it to Pending
                let status_patch = Self::create_typed_site_group_status_patch(
                    0, // Clear netbox_id
                    String::new(), // Clear URL
                    ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_site_group_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxSiteGroup status: {}", update_err);
                }
                // Fall through to creation
                None
            }
            DriftCheckResult::Recreate => {
                // Need to create - fall through
                None
            }
        };
        
        // Handle existing site group (from helper) or create new
        let netbox_site_group = match netbox_site_group {
            Some(site_group) => {
                // Always resolve tags (even if nothing else changed, tags might need updating)
                let resolved_tags_json = self.resolve_tag_references(
                    netbox_client.as_ref(),
                    &site_group_crd.spec.tags,
                    namespace,
                    name,
                    None,
                    "NetBoxSiteGroup",
                ).await;
                
                let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                
                // Update tags if they differ
                let site_group_id = site_group.id;
                let site_group_clone = site_group.clone();
                let site_group = match crate::reconcile_helpers::update_tags_if_differ(
                    site_group,
                    &site_group_crd.spec.tags,
                    resolved_tags.clone(),
                    |tags| async move {
                        netbox_client.update_site_group(
                            SiteGroupId(site_group_id),
                            Some(&site_group_crd.spec.name),
                            site_group_crd.spec.slug.as_deref(),
                            parent_id.map(SiteGroupId),
                            site_group_crd.spec.description.clone(),
                            site_group_crd.spec.comments.clone(),
                            tags,
                        ).await
                    },
                    &format!("NetBoxSiteGroup {}/{}", namespace, name),
                ).await {
                    Ok(Some(updated)) => {
                        use crate::events::reasons;
                        self.record_event_normal(
                            reasons::UPDATED,
                            &format!("Updated NetBoxSiteGroup {}/{} tags in NetBox", namespace, name),
                            site_group_crd,
                        ).await;
                        updated
                    }
                    Ok(None) => site_group_clone, // Tags are up-to-date
                    Err(e) => {
                        warn!("Failed to update NetBoxSiteGroup {}/{} tags: {}", namespace, name, e);
                        site_group_clone // Use existing if update fails
                    }
                };
                
                // Check if status needs updating
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    site_group_crd.status.as_ref(),
                    site_group.id,
                    &site_group.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    use crate::reconcile_helpers::update_resource_status;
                    let status_patch = Self::create_typed_site_group_status_patch(
                        site_group.id,
                        site_group.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    update_resource_status(
                        &*self.netbox_site_group_api,
                        name,
                        namespace,
                        &status_patch,
                        "NetBoxSiteGroup",
                        site_group.id,
                    ).await?;
                    debug!("Updated NetBoxSiteGroup {}/{} status: NetBox ID {}", namespace, name, site_group.id);
                } else {
                    debug!("NetBoxSiteGroup {}/{} already has correct status (ID: {}), skipping update", namespace, name, site_group.id);
                }
                site_group // Return existing site group
            }
            None => {
                // Need to create site group - try to find existing by name (idempotency fallback)
                let existing_site_group = match netbox_client.get_site_group_by_name(&site_group_crd.spec.name).await {
                    Ok(Some(sg)) => {
                        info!("SiteGroup {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", site_group_crd.spec.name, sg.id);
                        Some(sg)
                    }
                    Ok(None) => None,
                    Err(e) => {
                        warn!("Failed to query site group by name: {}, will try to create", e);
                        None
                    }
                };
                
                if let Some(existing) = existing_site_group {
                    // Resource exists but no status - check if tags need updating
                    let resolved_tags_json = self.resolve_tag_references(
                        netbox_client.as_ref(),
                        &site_group_crd.spec.tags,
                        namespace,
                        name,
                        None,
                        "NetBoxSiteGroup",
                    ).await;
                    let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                    
                    // Update tags if they differ
                    let existing_id = existing.id;
                    let existing_clone = existing.clone();
                    match crate::reconcile_helpers::update_tags_if_differ(
                        existing,
                        &site_group_crd.spec.tags,
                        resolved_tags,
                        |tags| async move {
                            netbox_client.update_site_group(
                                SiteGroupId(existing_id),
                                Some(&site_group_crd.spec.name),
                                site_group_crd.spec.slug.as_deref(),
                                parent_id.map(SiteGroupId),
                                site_group_crd.spec.description.clone(),
                                site_group_crd.spec.comments.clone(),
                                tags,
                            ).await
                        },
                        &format!("NetBoxSiteGroup {}/{} (idempotency path)", namespace, name),
                    ).await {
                        Ok(Some(updated)) => {
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::UPDATED,
                                &format!("Updated NetBoxSiteGroup {}/{} tags in NetBox", namespace, name),
                                site_group_crd,
                            ).await;
                            updated
                        }
                        Ok(None) => existing_clone, // Tags are up-to-date
                        Err(e) => {
                            warn!("Failed to update NetBoxSiteGroup {}/{} tags: {}", namespace, name, e);
                            existing_clone // Use existing if update fails
                        }
                    }
                } else {
                    // Resolve tags before creation
                    let resolved_tags_json = self.resolve_tag_references(
                        netbox_client.as_ref(),
                        &site_group_crd.spec.tags,
                        namespace,
                        name,
                        None,
                        "NetBoxSiteGroup",
                    ).await;
                    
                    let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                    
                    // Create site group
                    debug!("Attempting to create site group {} in NetBox", site_group_crd.spec.name);
                    match netbox_client.create_site_group(
                        &site_group_crd.spec.name,
                        site_group_crd.spec.slug.as_deref(),
                        parent_id.map(SiteGroupId),
                        site_group_crd.spec.description.clone(),
                        site_group_crd.spec.comments.clone(),
                        resolved_tags,
                    ).await {
                        Ok(created) => {
                            info!("Created site group {} in NetBox (ID: {})", created.name, created.id);
                            // Emit event for successful creation
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::CREATED,
                                &format!("Created site group {} in NetBox (ID: {})", created.name, created.id),
                                site_group_crd,
                            ).await;
                            created
                        }
                        Err(e) => {
                            use crate::reconcile_helpers::is_conflict_error;

                            if is_conflict_error(&e) {
                                warn!("SiteGroup {} creation conflicted, attempting idempotent lookup", site_group_crd.spec.name);

                                // Strategy 1: by name
                                let mut found_site_group = match netbox_client.get_site_group_by_name(&site_group_crd.spec.name).await {
                                    Ok(Some(sg)) => Some(sg),
                                    _ => None,
                                };

                                // Strategy 2: by slug if provided
                                if found_site_group.is_none() {
                                    if let Some(slug) = &site_group_crd.spec.slug {
                                        if let Ok(site_groups) = netbox_client.query_site_groups(&[("slug", slug)], false).await {
                                            if let Some(sg) = site_groups.first() {
                                                info!("Found existing site group by slug '{}' in NetBox (ID: {}) after conflict", slug, sg.id);
                                                found_site_group = Some(sg.clone());
                                            }
                                        }
                                    }
                                }

                                // Strategy 3: fallback query all and filter
                                if found_site_group.is_none() {
                                    if let Ok(all_site_groups) = netbox_client.query_site_groups(&[], true).await {
                                        if let Some(sg) = all_site_groups.iter().find(|sg| {
                                            let slug_match = site_group_crd
                                                .spec
                                                .slug
                                                .as_ref()
                                                .map(|spec_slug| sg.slug == *spec_slug)
                                                .unwrap_or(false);
                                            sg.name == site_group_crd.spec.name || slug_match
                                        }) {
                                            info!("Found existing site group in NetBox (ID: {}) via fallback query", sg.id);
                                            found_site_group = Some(sg.clone());
                                        }
                                    }
                                }

                                if let Some(found) = found_site_group {
                                    found
                                } else {
                                    let error_msg = format!("SiteGroup {} already exists in NetBox but could not retrieve it: {}", site_group_crd.spec.name, e);
                                    error!("{}", error_msg);
                                    return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                                }
                            } else {
                                let error_msg = format!("Failed to create site group in NetBox: {}", e);
                                error!("{}", error_msg);
                                // Emit event for reconciliation failure
                                use crate::events::reasons;
                                self.record_event_warning(
                                    reasons::RECONCILIATION_FAILED,
                                    &error_msg,
                                    site_group_crd,
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
        let status_patch = Self::create_typed_site_group_status_patch(
            netbox_site_group.id,
            netbox_site_group.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_site_group_api,
            name,
            namespace,
            &status_patch,
            "NetBoxSiteGroup",
            netbox_site_group.id,
        ).await?;
        info!("Updated NetBoxSiteGroup {}/{} status: NetBox ID {}", namespace, name, netbox_site_group.id);
        Ok(())
    }
}
