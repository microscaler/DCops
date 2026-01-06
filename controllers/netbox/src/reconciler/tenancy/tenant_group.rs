//! NetBoxTenantGroup reconciler

use crate::reconciler::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxTenantGroup, ResourceState};
use netbox_client::{NetBoxClientTrait, TenantGroupId};

impl Reconciler {
    pub async fn reconcile_netbox_tenant_group(&self, tenant_group_crd: &NetBoxTenantGroup) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::{extract_name_and_namespace, resolve_optional_dependency_id};
        let (name, namespace) = extract_name_and_namespace(tenant_group_crd, "NetBoxTenantGroup")?;
        
        info!("Reconciling NetBoxTenantGroup {}/{}", namespace, name);
        
        // Get client for shared resource (finds tenant from referencing Tenants)
        let netbox_client = self.token_resolver
            .create_client_for_shared_resource(namespace, "NetBoxTenantGroup", name)
            .await
            .map_err(|e| ControllerError::TokenResolution(e))?;
        
        // Resolve optional parent tenant group ID using helper
        let parent_id: Option<u64> = resolve_optional_dependency_id(
            &*self.netbox_tenant_group_api,
            tenant_group_crd.spec.parent.as_ref(),
            "NetBoxTenantGroup",
            "parent",
            name,
            |crd| crd.status.as_ref(),
        ).await;
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        // Check if drift detection is enabled (defaults to true)
        let drift_detection_enabled = tenant_group_crd.spec.drift_detection.unwrap_or(true);
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                tenant_group_crd.status.as_ref(),
                "NetBoxTenantGroup",
                namespace,
                name,
                |netbox_id: u64| async move {
                    netbox_client_ref.get_tenant_group(TenantGroupId(netbox_id)).await
                },
            ).await?
        };
        
        let netbox_tenant_group = match drift_result {
            DriftCheckResult::UseExisting(tenant_group) => {
                // Check for field drift if enabled
                if drift_detection_enabled {
                    if Self::tenant_group_needs_update(&tenant_group_crd.spec, &tenant_group, parent_id) {
                        // Field drift detected - update NetBox to match CRD (Git is source of truth)
                        warn!("NetBoxTenantGroup {}/{} fields differ from CRD spec, updating to match Git", namespace, name);
                        use crate::events::reasons;
                        self.record_event_warning(
                            reasons::DRIFT_DETECTED,
                            &format!("NetBoxTenantGroup {}/{} fields differ from CRD, updating to match Git", namespace, name),
                            tenant_group_crd,
                        ).await;
                        
                        // Resolve tags for update
                        let resolved_tags_json = self.resolve_tag_references(
                            netbox_client.as_ref(),
                            &tenant_group_crd.spec.tags,
                            namespace,
                            name,
                            Some(tenant_group.id),
                            "NetBoxTenantGroup",
                        ).await;
                        let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                        
                        match netbox_client.update_tenant_group(
                            TenantGroupId(tenant_group.id),
                            Some(&tenant_group_crd.spec.name),
                            tenant_group_crd.spec.slug.as_deref(),
                            tenant_group_crd.spec.description.clone(),
                            tenant_group_crd.spec.comments.clone(),
                            parent_id.map(TenantGroupId),
                            resolved_tags,
                        ).await {
                            Ok(updated) => {
                                info!("Updated NetBoxTenantGroup {}/{} in NetBox to match CRD (ID: {})", namespace, name, updated.id);
                                Some(updated)
                            }
                            Err(e) => {
                                error!("Failed to update NetBoxTenantGroup {}/{} in NetBox: {}", namespace, name, e);
                                use crate::events::reasons;
                                self.record_event_warning(
                                    reasons::RECONCILIATION_FAILED,
                                    &format!("Failed to update NetBoxTenantGroup {}/{} in NetBox: {}", namespace, name, e),
                                    tenant_group_crd,
                                ).await;
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    } else {
                        // No field drift - use existing
                        Some(tenant_group)
                    }
                } else {
                    // Drift detection disabled - use existing without checking
                    debug!("Drift detection disabled for NetBoxTenantGroup {}/{}", namespace, name);
                    Some(tenant_group)
                }
            }
            DriftCheckResult::StatusCleared { message } => {
                // Emit event for drift detection
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DRIFT_DETECTED,
                    &format!("NetBoxTenantGroup {}/{} drift detected: {}", namespace, name, message),
                    tenant_group_crd,
                ).await;
                
                // Status was cleared - update it to Pending
                let status_patch = Self::create_typed_tenant_group_status_patch(
                    0, // Clear netbox_id
                    String::new(), // Clear URL
                    ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_tenant_group_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxTenantGroup status: {}", update_err);
                }
                // Fall through to creation
                None
            }
            DriftCheckResult::Recreate => {
                // Need to create - fall through
                None
            }
        };
        
        // Handle existing tenant group (from helper) or create new
        let netbox_tenant_group = match netbox_tenant_group {
            Some(tenant_group) => {
                // Always resolve tags (even if nothing else changed, tags might need updating)
                let resolved_tags_json = self.resolve_tag_references(
                    netbox_client.as_ref(),
                    &tenant_group_crd.spec.tags,
                    namespace,
                    name,
                    Some(tenant_group.id),
                    "NetBoxTenantGroup",
                ).await;
                
                let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                
                // Update tags if they differ
                let tenant_group_id = tenant_group.id;
                let tenant_group_clone = tenant_group.clone();
                let tenant_group = match crate::reconcile_helpers::update_tags_if_differ(
                    tenant_group,
                    &tenant_group_crd.spec.tags,
                    resolved_tags.clone(),
                    |tags| async move {
                        netbox_client.update_tenant_group(
                            TenantGroupId(tenant_group_id),
                            Some(&tenant_group_crd.spec.name),
                            tenant_group_crd.spec.slug.as_deref(),
                            tenant_group_crd.spec.description.clone(),
                            tenant_group_crd.spec.comments.clone(),
                            parent_id.map(TenantGroupId),
                            tags,
                        ).await
                    },
                    &format!("NetBoxTenantGroup {}/{}", namespace, name),
                ).await {
                    Ok(Some(updated)) => {
                        use crate::events::reasons;
                        self.record_event_normal(
                            reasons::UPDATED,
                            &format!("Updated NetBoxTenantGroup {}/{} tags in NetBox", namespace, name),
                            tenant_group_crd,
                        ).await;
                        updated
                    }
                    Ok(None) => tenant_group_clone, // Tags are up-to-date
                    Err(e) => {
                        warn!("Failed to update NetBoxTenantGroup {}/{} tags: {}", namespace, name, e);
                        tenant_group_clone // Use existing if update fails
                    }
                };
                
                // Check if status needs updating
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    tenant_group_crd.status.as_ref(),
                    tenant_group.id,
                    &tenant_group.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    use crate::reconcile_helpers::update_resource_status;
                    let status_patch = Self::create_typed_tenant_group_status_patch(
                        tenant_group.id,
                        tenant_group.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    update_resource_status(
                        &*self.netbox_tenant_group_api,
                        name,
                        namespace,
                        &status_patch,
                        "NetBoxTenantGroup",
                        tenant_group.id,
                    ).await?;
                    debug!("Updated NetBoxTenantGroup {}/{} status: NetBox ID {}", namespace, name, tenant_group.id);
                } else {
                    debug!("NetBoxTenantGroup {}/{} already has correct status (ID: {}), skipping update", namespace, name, tenant_group.id);
                }
                tenant_group // Return existing tenant group
            }
            None => {
                // Resolve tags before creation
                let resolved_tags_json = self.resolve_tag_references(
                    netbox_client.as_ref(),
                    &tenant_group_crd.spec.tags,
                    namespace,
                    name,
                    None,
                    "NetBoxTenantGroup",
                ).await;
                
                let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                
                // Create tenant group
                debug!("Attempting to create tenant group {} in NetBox", tenant_group_crd.spec.name);
                match netbox_client.create_tenant_group(
                    &tenant_group_crd.spec.name,
                    tenant_group_crd.spec.slug.as_deref(),
                    tenant_group_crd.spec.description.clone(),
                    tenant_group_crd.spec.comments.clone(),
                    parent_id.map(TenantGroupId),
                    resolved_tags,
                ).await {
                    Ok(created) => {
                        info!("Created tenant group {} in NetBox (ID: {})", created.name, created.id);
                        // Emit event for successful creation
                        use crate::events::reasons;
                        self.record_event_normal(
                            reasons::CREATED,
                            &format!("Created tenant group {} in NetBox (ID: {})", created.name, created.id),
                            tenant_group_crd,
                        ).await;
                        created
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to create tenant group in NetBox: {}", e);
                        error!("{}", error_msg);
                        // Emit event for reconciliation failure
                        use crate::events::reasons;
                        self.record_event_warning(
                            reasons::RECONCILIATION_FAILED,
                            &error_msg,
                            tenant_group_crd,
                        ).await;
                        return Err(ControllerError::NetBox(e));
                    }
                }
            }
        };
        
        // Update status using helper
        use crate::reconcile_helpers::update_resource_status;
        let status_patch = Self::create_typed_tenant_group_status_patch(
            netbox_tenant_group.id,
            netbox_tenant_group.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_tenant_group_api,
            name,
            namespace,
            &status_patch,
            "NetBoxTenantGroup",
            netbox_tenant_group.id,
        ).await?;
        info!("Updated NetBoxTenantGroup {}/{} status: NetBox ID {}", namespace, name, netbox_tenant_group.id);
        Ok(())
    }
    
    fn tenant_group_needs_update(
        spec: &crds::NetBoxTenantGroupSpec,
        existing: &netbox_client::TenantGroup,
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
        // Note: Tags are handled separately using update_tags_if_differ helper
        
        name_diff || slug_diff || parent_diff || description_diff || comments_diff
    }
}

