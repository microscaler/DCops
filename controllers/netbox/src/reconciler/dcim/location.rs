//! NetBoxLocation reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxLocation, ResourceState};
use netbox_client::{LocationId, SiteId, TenantId};

impl Reconciler {
    fn location_needs_update(
        spec: &crds::NetBoxLocationSpec,
        existing: &netbox_client::Location,
        desired_site_id: u64,
        desired_tenant_id: u64,
        desired_parent_id: Option<u64>,
    ) -> bool {
        use crate::reconcile_helpers::{
            compare_string_field,
            compare_slug_field,
            compare_optional_string_field,
            compare_required_dependency_id,
            compare_optional_dependency_id,
        };
        
        let auto_generated_slug = spec.name.to_lowercase().replace(' ', "-");
        let existing_site_id = existing.site.id;
        let existing_tenant_id = existing.tenant.as_ref().map(|t| t.id);
        let existing_parent_id = existing.parent.as_ref().map(|p| p.id);
        
        compare_string_field(&spec.name, &existing.name)
            || compare_slug_field(&spec.slug, &existing.slug, auto_generated_slug)
            || compare_required_dependency_id(desired_site_id, Some(existing_site_id))
            || compare_optional_dependency_id(Some(desired_tenant_id), existing_tenant_id)
            || compare_optional_dependency_id(desired_parent_id, existing_parent_id)
            || compare_optional_string_field(&spec.facility, &existing.facility)
            || compare_optional_string_field(&spec.description, &existing.description)
            || compare_optional_string_field(&spec.comments, &existing.comments)
        // Tags are handled separately
    }

    pub async fn reconcile_netbox_location(&self, location_crd: &NetBoxLocation) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::extract_name_and_namespace;
        let (name, namespace) = extract_name_and_namespace(location_crd, "NetBoxLocation")?;
        let tenant_ref = &location_crd.spec.tenant;
        
        // SINGLE POINT: Get tenant-specific client
        let netbox_client = self.token_resolver
            .create_client_for_tenant(namespace, tenant_ref)
            .await?;
        
        info!("Reconciling NetBoxLocation {}/{}", namespace, name);
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                location_crd.status.as_ref(),
                "NetBoxLocation",
                namespace,
                name,
                |netbox_id: u64| async move {
                    netbox_client_ref.get_location(LocationId(netbox_id)).await
                },
            ).await?
        };
        
        // Resolve dependencies for drift detection
        use crate::reconcile_helpers::{resolve_required_dependency_id, resolve_optional_dependency_id};
        let site_id = match resolve_required_dependency_id(
            &*self.netbox_site_api,
            &location_crd.spec.site.name,
            "Site",
            name,
            |crd| crd.status.as_ref(),
        ).await {
            Ok(id) => id,
            Err(e) => {
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DEPENDENCY_NOT_FOUND,
                    &format!("Site '{}' not found or not ready: {}", location_crd.spec.site.name, e),
                    location_crd,
                ).await;
                return Err(e);
            }
        };
        
        let tenant_id = match resolve_required_dependency_id(
            &*self.netbox_tenant_api,
            &location_crd.spec.tenant.name,
            "Tenant",
            name,
            |crd| crd.status.as_ref(),
        ).await {
            Ok(id) => id,
            Err(e) => {
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DEPENDENCY_NOT_FOUND,
                    &format!("Tenant '{}' not found or not ready: {}", location_crd.spec.tenant.name, e),
                    location_crd,
                ).await;
                return Err(e);
            }
        };
        
        let parent_id: Option<u64> = resolve_optional_dependency_id(
            &*self.netbox_location_api,
            location_crd.spec.parent.as_ref(),
            "NetBoxLocation",
            "parent",
            name,
            |crd| crd.status.as_ref(),
        ).await;
        
        // Check if drift detection is enabled (defaults to true)
        let drift_detection_enabled = location_crd.spec.drift_detection.unwrap_or(true);
        
        let netbox_location = match drift_result {
            DriftCheckResult::UseExisting(location) => {
                // Check for field drift if enabled
                if drift_detection_enabled {
                    if Self::location_needs_update(&location_crd.spec, &location, site_id, tenant_id, parent_id) {
                        // Field drift detected - update NetBox to match CRD (Git is source of truth)
                        warn!("NetBoxLocation {}/{} fields differ from CRD spec, updating to match Git", namespace, name);
                        use crate::events::reasons;
                        self.record_event_warning(
                            reasons::DRIFT_DETECTED,
                            &format!("NetBoxLocation {}/{} fields differ from CRD, updating to match Git", namespace, name),
                            location_crd,
                        ).await;
                        
                        // Resolve tags for update
                        let resolved_tags_json = self.resolve_tag_references(
                            netbox_client.as_ref(),
                            &location_crd.spec.tags,
                            namespace,
                            name,
                            Some(location.id),
                        ).await;
                        let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                        
                        // Note: NetBox API doesn't support updating site_id via update_location
                        // If site has changed, we log a warning but can't update it
                        if location.site.id != site_id {
                            warn!("NetBoxLocation {}/{} site changed from {} to {}, but NetBox API doesn't support updating site. Location must be recreated to change site.", 
                                namespace, name, location.site.id, site_id);
                        }
                        
                        match netbox_client.update_location(
                            LocationId(location.id),
                            Some(&location_crd.spec.name),
                            location_crd.spec.slug.as_deref(),
                            parent_id.map(LocationId),
                            Some(TenantId(tenant_id)),
                            location_crd.spec.facility.as_deref(),
                            location_crd.spec.description.clone(),
                            location_crd.spec.comments.clone(),
                            resolved_tags,
                        ).await {
                            Ok(updated) => {
                                use crate::events::reasons;
                                self.record_event_normal(
                                    reasons::UPDATED,
                                    &format!("Updated NetBoxLocation {}/{} in NetBox to match CRD (ID: {})", namespace, name, updated.id),
                                    location_crd,
                                ).await;
                                Some(updated)
                            }
                            Err(e) => {
                                error!("Failed to update NetBoxLocation {}/{} in NetBox: {}", namespace, name, e);
                                Some(location) // Use existing if update fails
                            }
                        }
                    } else {
                        // No drift - use existing
                        Some(location)
                    }
                } else {
                    // Drift detection disabled - use existing
                    Some(location)
                }
            }
            DriftCheckResult::StatusCleared { message } => {
                // Emit event for drift detection
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DRIFT_DETECTED,
                    &format!("NetBoxLocation {}/{} drift detected: {}", namespace, name, message),
                    location_crd,
                ).await;
                
                // Status was cleared - update it to Pending
                let status_patch = Self::create_resource_status_patch(
                    0, // Clear netbox_id
                    String::new(), // Clear URL
                    ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_location_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxLocation status: {}", update_err);
                }
                // Fall through to creation
                None
            }
            DriftCheckResult::Recreate => {
                // Need to create - fall through
                None
            }
        };
        
        // Handle existing location (from helper) or create new
        let netbox_location = match netbox_location {
            Some(location) => {
                // Always resolve tags (even if nothing else changed, tags might need updating)
                info!("Resolving tags for location {}/{}: {:?}", namespace, name, location_crd.spec.tags);
                let resolved_tags_json = self.resolve_tag_references(
                    netbox_client.as_ref(),
                    &location_crd.spec.tags,
                    namespace,
                    name,
                None,
                ).await;
                
                // Convert resolved tags from Vec<serde_json::Value> to Vec<String>
                let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                
                // Update tags if they differ
                // Note: Location requires resolving dependencies before update, so we check tags first
                let tags_need_update = crate::reconcile_helpers::tags_differ(&location.tags, &location_crd.spec.tags);
                
                let location = if tags_need_update {
                    info!("NetBoxLocation {}/{} tags differ, updating in NetBox", namespace, name);
                    
                    // Dependencies already resolved at top level (site_id, tenant_id, parent_id)
                    
                    let location_id = location.id;
                    let location_clone = location.clone();
                    let tenant_id_for_update = tenant_id; // Capture for closure
                    let parent_id_for_update = parent_id; // Capture for closure
                    match crate::reconcile_helpers::update_tags_if_differ(
                        location,
                        &location_crd.spec.tags,
                        resolved_tags.clone(),
                        |tags| async move {
                            netbox_client.update_location(
                                LocationId(location_id),
                                Some(&location_crd.spec.name),
                                location_crd.spec.slug.as_deref(),
                                parent_id_for_update.map(LocationId),
                                Some(TenantId(tenant_id_for_update)),
                                location_crd.spec.facility.as_deref(),
                                location_crd.spec.description.clone(),
                                location_crd.spec.comments.clone(),
                                tags,
                            ).await
                        },
                        &format!("NetBoxLocation {}/{}", namespace, name),
                    ).await {
                        Ok(Some(updated)) => {
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::UPDATED,
                                &format!("Updated NetBoxLocation {}/{} tags in NetBox", namespace, name),
                                location_crd,
                            ).await;
                            updated
                        }
                        Ok(None) => location_clone, // Tags are up-to-date
                        Err(e) => {
                            warn!("Failed to update NetBoxLocation {}/{} tags: {}", namespace, name, e);
                            location_clone // Use existing if update fails
                        }
                    }
                } else {
                    location // Tags are up-to-date
                };
                
                // Check if status needs updating
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    location_crd.status.as_ref(),
                    location.id,
                    &location.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    use crate::reconcile_helpers::update_resource_status;
                    let status_patch = Self::create_resource_status_patch(
                        location.id,
                        location.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    update_resource_status(
                        &*self.netbox_location_api,
                        name,
                        namespace,
                        &status_patch,
                        "NetBoxLocation",
                        location.id,
                    ).await?;
                    debug!("Updated NetBoxLocation {}/{} status: NetBox ID {}", namespace, name, location.id);
                } else {
                    debug!("NetBoxLocation {}/{} already has correct status (ID: {}), skipping update", namespace, name, location.id);
                }
                location // Return existing location
            }
            None => {
                // Need to create location - resolve dependencies first using helpers
                use crate::reconcile_helpers::{validate_reference_kind, resolve_required_dependency_id, resolve_optional_dependency_id};
                
                // Validate and resolve site ID (required)
                validate_reference_kind(&location_crd.spec.site, "NetBoxSite", "site", name)?;
                let site_id = match resolve_required_dependency_id(
                    &*self.netbox_site_api,
                    &location_crd.spec.site.name,
                    "Site",
                    name,
                    |crd| crd.status.as_ref(),
                ).await {
                    Ok(id) => id,
                    Err(e) => {
                        // Emit event for dependency not found
                        use crate::events::reasons;
                        self.record_event_warning(
                            reasons::DEPENDENCY_NOT_FOUND,
                            &format!("Site '{}' not found or not ready: {}", location_crd.spec.site.name, e),
                            location_crd,
                        ).await;
                        return Err(e);
                    }
                };
                
                // Resolve optional parent location ID
                let parent_id: Option<u64> = resolve_optional_dependency_id(
                    &*self.netbox_location_api,
                    location_crd.spec.parent.as_ref(),
                    "NetBoxLocation",
                    "parent",
                    name,
                    |crd| crd.status.as_ref(),
                ).await;
                
                // Validate and resolve tenant ID (required)
                validate_reference_kind(&location_crd.spec.tenant, "NetBoxTenant", "tenant", name)?;
                let tenant_id = match resolve_required_dependency_id(
                    &*self.netbox_tenant_api,
                    &location_crd.spec.tenant.name,
                    "Tenant",
                    name,
                    |crd| crd.status.as_ref(),
                ).await {
                    Ok(id) => id,
                    Err(e) => {
                        // Emit event for dependency not found
                        use crate::events::reasons;
                        self.record_event_warning(
                            reasons::DEPENDENCY_NOT_FOUND,
                            &format!("Tenant '{}' not found or not ready: {}", location_crd.spec.tenant.name, e),
                            location_crd,
                        ).await;
                        return Err(e);
                    }
                };
                
                // Try to find existing location by name and site
                let existing_location = match netbox_client.query_locations(
                    &[("site_id", &site_id.to_string()), ("name", &location_crd.spec.name)],
                    false,
                ).await {
                    Ok(locations) => locations.first().cloned(),
                    Err(_) => None
                };
                
                let netbox_location = if let Some(existing) = existing_location {
                    info!("Location {} already exists in NetBox (ID: {})", location_crd.spec.name, existing.id);
                    
                    // Resource exists but no status - check if tags need updating
                    let resolved_tags_json = self.resolve_tag_references(
                        netbox_client.as_ref(),
                        &location_crd.spec.tags,
                        namespace,
                        name,
                    None,
                ).await;
                    let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                    
                    // Update tags if they differ (location requires dependency resolution)
                    let tags_need_update = crate::reconcile_helpers::tags_differ(&existing.tags, &location_crd.spec.tags);
                    
                    if tags_need_update {
                        info!("NetBoxLocation {}/{} tags differ (idempotency path), updating in NetBox", namespace, name);
                        
                        // Resolve optional parent location ID
                        use crate::reconcile_helpers::resolve_optional_dependency_id;
                        let parent_id: Option<u64> = resolve_optional_dependency_id(
                            &*self.netbox_location_api,
                            location_crd.spec.parent.as_ref(),
                            "NetBoxLocation",
                            "parent",
                            name,
                            |crd| crd.status.as_ref(),
                        ).await;
                        
                        let existing_id = existing.id;
                        let existing_clone = existing.clone();
                        match crate::reconcile_helpers::update_tags_if_differ(
                            existing,
                            &location_crd.spec.tags,
                            resolved_tags,
                            |tags| async move {
                                netbox_client.update_location(
                                    LocationId(existing_id),
                                    Some(&location_crd.spec.name),
                                    location_crd.spec.slug.as_deref(),
                                    parent_id.map(LocationId),
                                    Some(TenantId(tenant_id)),
                                    location_crd.spec.facility.as_deref(),
                                    location_crd.spec.description.clone(),
                                    None, // comments
                                    tags,
                                ).await
                            },
                            &format!("NetBoxLocation {}/{} (idempotency path)", namespace, name),
                        ).await {
                            Ok(Some(updated)) => {
                                use crate::events::reasons;
                                self.record_event_normal(
                                    reasons::UPDATED,
                                    &format!("Updated NetBoxLocation {}/{} tags in NetBox", namespace, name),
                                    location_crd,
                                ).await;
                                updated
                            }
                            Ok(None) => existing_clone, // Tags are up-to-date
                            Err(e) => {
                                warn!("Failed to update NetBoxLocation {}/{} tags: {}", namespace, name, e);
                                existing_clone // Use existing if update fails
                            }
                        }
                    } else {
                        existing // Tags are up-to-date
                    }
                } else {
                    // Resolve tags before creation
                    info!("Resolving tags for location {}/{}: {:?}", namespace, name, location_crd.spec.tags);
                    let resolved_tags_json = self.resolve_tag_references(
                        netbox_client.as_ref(),
                        &location_crd.spec.tags,
                        namespace,
                        name,
                    None,
                ).await;
                    
                    let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                    
                    match netbox_client.create_location(
                        SiteId(site_id),
                        &location_crd.spec.name,
                        location_crd.spec.slug.as_deref(),
                        parent_id.map(LocationId),
                        Some(TenantId(tenant_id)),
                        location_crd.spec.facility.as_deref(),
                        location_crd.spec.description.clone(),
                        location_crd.spec.comments.clone(),
                        resolved_tags,
                    ).await {
                        Ok(created) => {
                            info!("Created location {} in NetBox (ID: {})", created.name, created.id);
                            created
                        }
                        Err(e) => {
                            // Handle CREATE conflicts using shared helper (GitOps idempotency)
                            use crate::reconcile_helpers::is_conflict_error;
                            
                            if is_conflict_error(&e) {
                                warn!("Location {} creation failed with conflict, attempting to retrieve existing location (idempotency)", location_crd.spec.name);
                                
                                // Try multiple query strategies
                                let mut found_location = None;
                                
                                // Strategy 1: Query by site_id and name
                                match netbox_client.query_locations(
                                    &[("site_id", &site_id.to_string()), ("name", &location_crd.spec.name)],
                                    false,
                                ).await {
                                    Ok(locations) => {
                                        if let Some(loc) = locations.first() {
                                            info!("Found existing location by name '{}' in NetBox (ID: {}) after conflict", location_crd.spec.name, loc.id);
                                            found_location = Some(loc.clone());
                                        }
                                    }
                                    Err(_) => {}
                                }
                                
                                // Strategy 2: Query by slug if not found
                                if found_location.is_none() {
                                    if let Some(slug) = &location_crd.spec.slug {
                                        match netbox_client.query_locations(
                                            &[("site_id", &site_id.to_string()), ("slug", slug)],
                                            false,
                                        ).await {
                                            Ok(locations) => {
                                                if let Some(loc) = locations.first() {
                                                    info!("Found existing location by slug '{}' in NetBox (ID: {}) after conflict", slug, loc.id);
                                                    found_location = Some(loc.clone());
                                                }
                                            }
                                            Err(_) => {}
                                        }
                                    }
                                }
                                
                                // Strategy 3: Fallback - query all locations for this site and filter
                                if found_location.is_none() {
                                    match netbox_client.query_locations(&[("site_id", &site_id.to_string())], true).await {
                                        Ok(all_locations) => {
                                            if let Some(loc) = all_locations.iter().find(|l| {
                                                l.name == location_crd.spec.name ||
                                                location_crd.spec.slug.as_ref().map(|slug| l.slug == *slug).unwrap_or(false)
                                            }) {
                                                info!("Found existing location in NetBox (ID: {}) via fallback query", loc.id);
                                                found_location = Some(loc.clone());
                                            }
                                        }
                                        Err(_) => {}
                                    }
                                }
                                
                                if let Some(found) = found_location {
                                    info!("Found existing location {} in NetBox (ID: {}) via conflict resolution (idempotency)", found.name, found.id);
                                    found
                                } else {
                                    let error_msg = format!("Location {} already exists in NetBox but could not retrieve it: {}", location_crd.spec.name, e);
                                    error!("{}", error_msg);
                                    return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                                }
                            } else {
                                // Not a conflict, return original error
                                let error_msg = format!("Failed to create location in NetBox: {}", e);
                                error!("{}", error_msg);
                                // Emit event for reconciliation failure
                                use crate::events::reasons;
                                self.record_event_warning(
                                    reasons::RECONCILIATION_FAILED,
                                    &error_msg,
                                    location_crd,
                                ).await;
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    }
                };
                
                netbox_location
            }
        };
        
        // Update status (use lowercase state to match CRD validation schema)
        use crate::reconcile_helpers::update_resource_status;
        let status_patch = Self::create_resource_status_patch(
            netbox_location.id,
            netbox_location.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_location_api,
            name,
            namespace,
            &status_patch,
            "NetBoxLocation",
            netbox_location.id,
        ).await?;
        info!("Updated NetBoxLocation {}/{} status: NetBox ID {}", namespace, name, netbox_location.id);
        Ok(())
    }
}
