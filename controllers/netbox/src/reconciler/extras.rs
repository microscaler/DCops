//! Extras reconcilers (Roles, Tags)

use super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxRole, NetBoxTag, ResourceState};
use netbox_client::NetBoxClientTrait;

impl Reconciler {
    /// Reconciles a NetBoxRole resource (Extras Role, not IPAM Role).
    pub async fn reconcile_netbox_role(&self, role_crd: &NetBoxRole) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::extract_name_and_namespace;
        let (name, namespace) = extract_name_and_namespace(role_crd, "NetBoxRole")?;
        
        info!("Reconciling NetBoxRole {}/{}", namespace, name);
        
        // Get client for shared resource (finds tenant from referencing resources or uses system tenant)
        let netbox_client = self.token_resolver
            .create_client_for_shared_resource(namespace, "NetBoxRole", name)
            .await
            .map_err(|e| ControllerError::TokenResolution(e))?;
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                role_crd.status.as_ref(),
                "NetBoxRole",
                namespace,
                name,
                |netbox_id: u64| async move {
                    let id_str = netbox_id.to_string();
                    netbox_client_ref.query_roles(&[("id", &id_str)], false)
                        .await
                        .and_then(|mut roles| {
                            roles.pop().ok_or_else(|| netbox_client::NetBoxError::NotFound(format!("Role {} not found", netbox_id)))
                        })
                },
            ).await?
        };
        
        let netbox_role = match drift_result {
            DriftCheckResult::UseExisting(role) => Some(role),
            DriftCheckResult::StatusCleared { message } => {
                // Emit event for drift detection
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DRIFT_DETECTED,
                    &format!("NetBoxRole {}/{} drift detected: {}", namespace, name, message),
                    role_crd,
                ).await;
                
                let status_patch = Self::create_typed_role_status_patch(
                    0, String::new(), ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_role_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxRole status: {}", update_err);
                }
                None
            }
            DriftCheckResult::Recreate => None,
        };
        
        let netbox_role = match netbox_role {
            Some(role) => {
                // Always resolve tags (even if nothing else changed, tags might need updating)
                let resolved_tags_json = self.resolve_tag_references(
                    netbox_client.as_ref(),
                    &role_crd.spec.tags,
                    namespace,
                    name,
                    role_crd.status.as_ref().and_then(|s| s.netbox_id).filter(|&id| id != 0),
                ).await;
                
                // Convert resolved tags from Vec<serde_json::Value> to Vec<String>
                let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                
                // Update tags if they differ
                use netbox_client::RoleId;
                let role_id = role.id;
                let role_clone = role.clone();
                let role = match crate::reconcile_helpers::update_tags_if_differ(
                    role,
                    &role_crd.spec.tags,
                    resolved_tags.clone(),
                    |tags| async move {
                        netbox_client.update_role(
                            RoleId(role_id),
                            Some(&role_crd.spec.name),
                            role_crd.spec.slug.as_deref(),
                            role_crd.spec.description.clone(),
                            role_crd.spec.weight,
                            role_crd.spec.comments.clone(),
                            tags,
                        ).await
                    },
                    &format!("NetBoxRole {}/{}", namespace, name),
                ).await {
                    Ok(Some(updated)) => {
                        use crate::events::reasons;
                        self.record_event_normal(
                            reasons::UPDATED,
                            &format!("Updated NetBoxRole {}/{} tags in NetBox", namespace, name),
                            role_crd,
                        ).await;
                        updated
                    }
                    Ok(None) => role_clone, // Tags are up-to-date
                    Err(e) => {
                        warn!("Failed to update NetBoxRole {}/{} tags: {}", namespace, name, e);
                        role_clone // Use existing if update fails
                    }
                };
                
                // Check if status needs updating
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    role_crd.status.as_ref(),
                    role.id,
                    &role.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    use crate::reconcile_helpers::update_resource_status;
                    let status_patch = Self::create_typed_role_status_patch(
                        role.id,
                        role.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    update_resource_status(
                        &*self.netbox_role_api,
                        name,
                        namespace,
                        &status_patch,
                        "NetBoxRole",
                        role.id,
                    ).await?;
                    debug!("Updated NetBoxRole {}/{} status: NetBox ID {}", namespace, name, role.id);
                    return Ok(());
                } else {
                    debug!("NetBoxRole {}/{} already has correct status (ID: {}), skipping update", namespace, name, role.id);
                    return Ok(());
                }
            }
            None => {
                let existing_role = match netbox_client.query_roles(&[("name", &role_crd.spec.name)], false).await {
                    Ok(mut roles) => {
                        roles.pop()
                    }
                    Err(e) => {
                        warn!("Failed to query role by name: {}, will try to create", e);
                        None
                    }
                };
                
                if let Some(r) = existing_role.as_ref() {
                    info!("Role {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", role_crd.spec.name, r.id);
                }
                
                if let Some(existing) = existing_role {
                    // Resource exists but no status - check if tags need updating
                    let resolved_tags_json = self.resolve_tag_references(
                        netbox_client.as_ref(),
                        &role_crd.spec.tags,
                        namespace,
                        name,
                    None,
                ).await;
                    let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                    
                    // Update tags if they differ
                    use netbox_client::RoleId;
                    let existing_id = existing.id;
                    let existing_clone = existing.clone();
                    match crate::reconcile_helpers::update_tags_if_differ(
                        existing,
                        &role_crd.spec.tags,
                        resolved_tags,
                        |tags| async move {
                            netbox_client.update_role(
                                RoleId(existing_id),
                                Some(&role_crd.spec.name),
                                role_crd.spec.slug.as_deref(),
                                role_crd.spec.description.clone(),
                                role_crd.spec.weight,
                                role_crd.spec.comments.clone(),
                                tags,
                            ).await
                        },
                        &format!("NetBoxRole {}/{} (idempotency path)", namespace, name),
                    ).await {
                        Ok(Some(updated)) => {
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::UPDATED,
                                &format!("Updated NetBoxRole {}/{} tags in NetBox", namespace, name),
                                role_crd,
                            ).await;
                            updated
                        }
                        Ok(None) => existing_clone, // Tags are up-to-date
                        Err(e) => {
                            warn!("Failed to update NetBoxRole {}/{} tags: {}", namespace, name, e);
                            existing_clone // Use existing if update fails
                        }
                    }
                } else {
                    // Resolve tags before create
                    let resolved_tags_json = self.resolve_tag_references(
                        netbox_client.as_ref(),
                        &role_crd.spec.tags,
                        namespace,
                        name,
                    None,
                ).await;
                    let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                    
                    debug!("Attempting to create role {} in NetBox", role_crd.spec.name);
                    match netbox_client.create_role(
                        &role_crd.spec.name,
                        role_crd.spec.slug.as_deref(),
                        role_crd.spec.description.clone(),
                        role_crd.spec.weight,
                        role_crd.spec.comments.clone(),
                        resolved_tags,
                    ).await {
                        Ok(created) => {
                            info!("Created role {} in NetBox (ID: {})", created.name, created.id);
                            // Emit event for successful creation
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::CREATED,
                                &format!("Created role {} in NetBox (ID: {})", created.name, created.id),
                                role_crd,
                            ).await;
                            created
                        }
                        Err(e) => {
                            use crate::reconcile_helpers::is_conflict_error;

                            if is_conflict_error(&e) {
                                warn!("Role {} creation conflicted, attempting idempotent lookup", role_crd.spec.name);

                                // Strategy 1: by name
                                let mut found_role = match netbox_client.query_roles(&[("name", &role_crd.spec.name)], false).await {
                                    Ok(mut roles) => roles.pop(),
                                    _ => None,
                                };

                                // Strategy 2: by slug if provided
                                if found_role.is_none() {
                                    if let Some(slug) = &role_crd.spec.slug {
                                        if let Ok(roles) = netbox_client.query_roles(&[("slug", slug)], false).await {
                                            if let Some(role) = roles.first() {
                                                info!("Found existing role by slug '{}' in NetBox (ID: {}) after conflict", slug, role.id);
                                                found_role = Some(role.clone());
                                            }
                                        }
                                    }
                                }

                                // Strategy 3: fallback query all and filter
                                if found_role.is_none() {
                                    if let Ok(all_roles) = netbox_client.query_roles(&[], true).await {
                                        if let Some(role) = all_roles.iter().find(|r| {
                                            let slug_match = role_crd
                                                .spec
                                                .slug
                                                .as_ref()
                                                .map(|spec_slug| r.slug == *spec_slug)
                                                .unwrap_or(false);
                                            r.name == role_crd.spec.name || slug_match
                                        }) {
                                            info!("Found existing role in NetBox (ID: {}) via fallback query", role.id);
                                            found_role = Some(role.clone());
                                        }
                                    }
                                }

                                if let Some(found) = found_role {
                                    found
                                } else {
                                    let error_msg = format!("Role {} already exists in NetBox but could not retrieve it: {}", role_crd.spec.name, e);
                                    error!("{}", error_msg);
                                    return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                                }
                            } else {
                                let error_msg = format!("Failed to create role in NetBox: {}", e);
                                error!("{}", error_msg);
                                // Emit event for reconciliation failure
                                use crate::events::reasons;
                                self.record_event_warning(
                                    reasons::RECONCILIATION_FAILED,
                                    &error_msg,
                                    role_crd,
                                ).await;
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    }
                }
            }
        };
        
        use crate::reconcile_helpers::update_resource_status;
        let status_patch = Self::create_typed_role_status_patch(
            netbox_role.id,
            netbox_role.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_role_api,
            name,
            namespace,
            &status_patch,
            "NetBoxRole",
            netbox_role.id,
        ).await?;
        info!("Updated NetBoxRole {}/{} status: NetBox ID {}", namespace, name, netbox_role.id);
        Ok(())
    }
    
    /// Reconciles a NetBoxTag resource.
    pub async fn reconcile_netbox_tag(&self, tag_crd: &NetBoxTag) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::extract_name_and_namespace;
        let (name, namespace) = extract_name_and_namespace(tag_crd, "NetBoxTag")?;
        
        info!("Reconciling NetBoxTag {}/{}", namespace, name);
        
        // Get client - use tenant from CRD if specified, otherwise fall back to shared resource logic
        let netbox_client = if let Some(tenant_ref) = &tag_crd.spec.tenant {
            // Tag has explicit tenant reference - use that tenant's token
            self.token_resolver
                .create_client_for_tenant(namespace, tenant_ref)
                .await
                .map_err(|e| ControllerError::TokenResolution(e))?
        } else {
            // No tenant specified - use shared resource logic (finds tenant from referencing resources or uses system tenant)
            self.token_resolver
                .create_client_for_shared_resource(namespace, "NetBoxTag", name)
                .await
                .map_err(|e| ControllerError::TokenResolution(e))?
        };
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                tag_crd.status.as_ref(),
                "NetBoxTag",
                namespace,
                name,
                |netbox_id: u64| async move {
                    let id_str = netbox_id.to_string();
                    netbox_client_ref.query_tags(&[("id", &id_str)], false)
                        .await
                        .and_then(|mut tags| {
                            tags.pop().ok_or_else(|| netbox_client::NetBoxError::NotFound(format!("Tag {} not found", netbox_id)))
                        })
                },
            ).await?
        };
        
        let netbox_tag = match drift_result {
            DriftCheckResult::UseExisting(tag) => Some(tag),
            DriftCheckResult::StatusCleared { message } => {
                // Emit event for drift detection
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DRIFT_DETECTED,
                    &format!("NetBoxTag {}/{} drift detected: {}", namespace, name, message),
                    tag_crd,
                ).await;
                
                let status_patch = Self::create_typed_tag_status_patch(
                    0, String::new(), ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_tag_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxTag status: {}", update_err);
                }
                None
            }
            DriftCheckResult::Recreate => None,
        };
        
        let netbox_tag = match netbox_tag {
            Some(tag) => {
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    tag_crd.status.as_ref(),
                    tag.id,
                    &tag.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    use crate::reconcile_helpers::update_resource_status;
                    let status_patch = Self::create_typed_tag_status_patch(
                        tag.id,
                        tag.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    update_resource_status(
                        &*self.netbox_tag_api,
                        name,
                        namespace,
                        &status_patch,
                        "NetBoxTag",
                        tag.id,
                    ).await?;
                    debug!("Updated NetBoxTag {}/{} status: NetBox ID {}", namespace, name, tag.id);
                    return Ok(());
                } else {
                    debug!("NetBoxTag {}/{} already has correct status (ID: {}), skipping update", namespace, name, tag.id);
                    return Ok(());
                }
            }
            None => {
                let existing_tag = match netbox_client.query_tags(&[("name", &tag_crd.spec.name)], false).await {
                    Ok(mut tags) => {
                        tags.pop()
                    }
                    Err(e) => {
                        warn!("Failed to query tag by name: {}, will try to create", e);
                        None
                    }
                };
                
                if let Some(t) = existing_tag.as_ref() {
                    info!("Tag {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", tag_crd.spec.name, t.id);
                }
                
                if let Some(existing) = existing_tag {
                    existing
                } else {
                    debug!("Attempting to create tag {} in NetBox", tag_crd.spec.name);
                    match netbox_client.create_tag(
                        &tag_crd.spec.name,
                        tag_crd.spec.slug.as_deref(),
                        tag_crd.spec.color.as_deref(),
                        tag_crd.spec.description.clone(),
                        tag_crd.spec.comments.clone(),
                    ).await {
                        Ok(created) => {
                            info!("Created tag {} in NetBox (ID: {})", created.name, created.id);
                            created
                        }
                        Err(e) => {
                            use crate::reconcile_helpers::is_conflict_error;

                            if is_conflict_error(&e) {
                                warn!("Tag {} creation conflicted, attempting idempotent lookup", tag_crd.spec.name);

                                // Strategy 1: by name
                                let mut found_tag = match netbox_client.query_tags(&[("name", &tag_crd.spec.name)], false).await {
                                    Ok(mut tags) => tags.pop(),
                                    _ => None,
                                };

                                // Strategy 2: by slug if provided
                                if found_tag.is_none() {
                                    if let Some(slug) = &tag_crd.spec.slug {
                                        if let Ok(tags) = netbox_client.query_tags(&[("slug", slug)], false).await {
                                            if let Some(tag) = tags.first() {
                                                info!("Found existing tag by slug '{}' in NetBox (ID: {}) after conflict", slug, tag.id);
                                                found_tag = Some(tag.clone());
                                            }
                                        }
                                    }
                                }

                                // Strategy 3: fallback query all and filter
                                if found_tag.is_none() {
                                    if let Ok(all_tags) = netbox_client.query_tags(&[], true).await {
                                        if let Some(tag) = all_tags.iter().find(|t| {
                                            let slug_match = tag_crd
                                                .spec
                                                .slug
                                                .as_ref()
                                                .map(|spec_slug| t.slug == *spec_slug)
                                                .unwrap_or(false);
                                            t.name == tag_crd.spec.name || slug_match
                                        }) {
                                            info!("Found existing tag in NetBox (ID: {}) via fallback query", tag.id);
                                            found_tag = Some(tag.clone());
                                        }
                                    }
                                }

                                if let Some(found) = found_tag {
                                    found
                                } else {
                                    let error_msg = format!("Tag {} already exists in NetBox but could not retrieve it: {}", tag_crd.spec.name, e);
                                    error!("{}", error_msg);
                                    return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                                }
                            } else {
                                let error_msg = format!("Failed to create tag in NetBox: {}", e);
                                error!("{}", error_msg);
                                // Emit event for reconciliation failure
                                use crate::events::reasons;
                                self.record_event_warning(
                                    reasons::RECONCILIATION_FAILED,
                                    &error_msg,
                                    tag_crd,
                                ).await;
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    }
                }
            }
        };
        
        use crate::reconcile_helpers::update_resource_status;
        let status_patch = Self::create_typed_tag_status_patch(
            netbox_tag.id,
            netbox_tag.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_tag_api,
            name,
            namespace,
            &status_patch,
            "NetBoxTag",
            netbox_tag.id,
        ).await?;
        info!("Updated NetBoxTag {}/{} status: NetBox ID {}", namespace, name, netbox_tag.id);
        Ok(())
    }
}
