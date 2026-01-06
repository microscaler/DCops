//! NetBoxVLAN reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxVLAN, ResourceState};
use netbox_client::{VlanId, SiteId, TenantId, RoleId, VlanGroupId};

impl Reconciler {
    fn vlan_needs_update(
        spec: &crds::NetBoxVLANSpec,
        existing: &netbox_client::Vlan,
        desired_site_id: Option<u64>,
        desired_tenant_id: u64,
        desired_role_id: Option<u64>,
        desired_group_id: Option<u64>,
    ) -> bool {
        use crate::reconcile_helpers::{
            compare_string_field,
            compare_optional_string_field,
            compare_optional_dependency_id,
            compare_enum_field,
        };
        
        let existing_site_id = existing.site.as_ref().map(|s| s.id);
        let existing_tenant_id = existing.tenant.as_ref().map(|t| t.id);
        let existing_role_id = existing.role.as_ref().map(|r| r.id);
        let existing_group_id = existing.group.as_ref().map(|g| g.id);
        
        // Note: description and comments are String in NetBox model but Option<String> in CRD
        let existing_description = Some(existing.description.clone());
        let existing_comments = Some(existing.comments.clone());
        
        // Convert NetBox VlanStatus to CRD VlanStatus for comparison
        let existing_status = match existing.status {
            netbox_client::VlanStatus::Active => crds::VlanStatus::Active,
            netbox_client::VlanStatus::Reserved => crds::VlanStatus::Reserved,
            netbox_client::VlanStatus::Deprecated => crds::VlanStatus::Deprecated,
        };
        
        // Evaluate all comparisons to log all field differences (no short-circuit)
        let vid_diff = spec.vid != existing.vid;
        let name_diff = compare_string_field(&spec.name, &existing.name);
        let site_diff = compare_optional_dependency_id(desired_site_id, existing_site_id);
        let tenant_diff = compare_optional_dependency_id(Some(desired_tenant_id), existing_tenant_id);
        let role_diff = compare_optional_dependency_id(desired_role_id, existing_role_id);
        let group_diff = compare_optional_dependency_id(desired_group_id, existing_group_id);
        let status_diff = compare_enum_field(&spec.status, &existing_status);
        let description_diff = compare_optional_string_field(&spec.description, &existing_description);
        let comments_diff = compare_optional_string_field(&spec.comments, &existing_comments);
        // Tags are handled separately
        
        vid_diff || name_diff || site_diff || tenant_diff || role_diff || group_diff || status_diff || description_diff || comments_diff
    }

    pub async fn reconcile_netbox_vlan(&self, vlan_crd: &NetBoxVLAN) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::extract_name_and_namespace;
        let (name, namespace) = extract_name_and_namespace(vlan_crd, "NetBoxVLAN")?;
        let tenant_ref = &vlan_crd.spec.tenant;
        
        // SINGLE POINT: Get tenant-specific client
        let netbox_client = self.token_resolver
            .create_client_for_tenant(namespace, tenant_ref)
            .await?;
        
        info!("Reconciling NetBoxVLAN {}/{}", namespace, name);
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                vlan_crd.status.as_ref(),
                "NetBoxVLAN",
                namespace,
                name,
                |netbox_id: u64| async move {
                    netbox_client_ref.get_vlan(VlanId(netbox_id as u32)).await
                },
            ).await?
        };
        
        // Resolve dependencies for drift detection
        use crate::reconcile_helpers::{resolve_required_dependency_id, resolve_optional_dependency_id};
        let site_id: Option<u64> = resolve_optional_dependency_id(
            &*self.netbox_site_api,
            vlan_crd.spec.site.as_ref(),
            "NetBoxSite",
            "site",
            name,
            |crd| crd.status.as_ref(),
        ).await;
        
        let tenant_id = match resolve_required_dependency_id(
            &*self.netbox_tenant_api,
            &vlan_crd.spec.tenant.name,
            "Tenant",
            name,
            |crd| crd.status.as_ref(),
        ).await {
            Ok(id) => id,
            Err(e) => {
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DEPENDENCY_NOT_FOUND,
                    &format!("Tenant '{}' not found or not ready: {}", vlan_crd.spec.tenant.name, e),
                    vlan_crd,
                ).await;
                return Err(e);
            }
        };
        
        let role_id: Option<u64> = resolve_optional_dependency_id(
            &*self.netbox_role_api,
            vlan_crd.spec.role.as_ref(),
            "NetBoxRole",
            "role",
            name,
            |crd| crd.status.as_ref(),
        ).await;
        
        // Note: VLAN group is not yet implemented as a CRD, so we skip it for now
        let group_id: Option<u64> = None;
        
        // Check if drift detection is enabled (defaults to true)
        let drift_detection_enabled = vlan_crd.spec.drift_detection.unwrap_or(true);
        
        let netbox_vlan = match drift_result {
            DriftCheckResult::UseExisting(vlan) => {
                // Check for field drift if enabled
                if drift_detection_enabled {
                    if Self::vlan_needs_update(&vlan_crd.spec, &vlan, site_id, tenant_id, role_id, group_id) {
                        // Field drift detected - update NetBox to match CRD (Git is source of truth)
                        warn!("NetBoxVLAN {}/{} fields differ from CRD spec, updating to match Git", namespace, name);
                        use crate::events::reasons;
                        self.record_event_warning(
                            reasons::DRIFT_DETECTED,
                            &format!("NetBoxVLAN {}/{} fields differ from CRD, updating to match Git", namespace, name),
                            vlan_crd,
                        ).await;
                        
                        // Resolve tags for update
                        let resolved_tags_json = self.resolve_tag_references(
                            netbox_client.as_ref(),
                            &vlan_crd.spec.tags,
                            namespace,
                            name,
                            Some(vlan.id),
                            "NetBoxVLAN",
                        ).await;
                        let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                        
                        // Convert status enum to string for API
                        let status_str = match vlan_crd.spec.status {
                            crds::VlanStatus::Active => Some("active"),
                            crds::VlanStatus::Reserved => Some("reserved"),
                            crds::VlanStatus::Deprecated => Some("deprecated"),
                        };
                        
                        match netbox_client.update_vlan(
                            VlanId(vlan.id as u32),
                            Some(vlan_crd.spec.vid),
                            Some(&vlan_crd.spec.name),
                            site_id.map(SiteId),
                            group_id.map(VlanGroupId),
                            Some(TenantId(tenant_id)),
                            role_id.map(RoleId),
                            status_str,
                            vlan_crd.spec.description.clone(),
                            vlan_crd.spec.comments.clone(),
                            resolved_tags,
                        ).await {
                            Ok(updated) => {
                                use crate::events::reasons;
                                self.record_event_normal(
                                    reasons::UPDATED,
                                    &format!("Updated NetBoxVLAN {}/{} in NetBox to match CRD (ID: {})", namespace, name, updated.id),
                                    vlan_crd,
                                ).await;
                                Some(updated)
                            }
                            Err(e) => {
                                error!("Failed to update NetBoxVLAN {}/{} in NetBox: {}", namespace, name, e);
                                Some(vlan) // Use existing if update fails
                            }
                        }
                    } else {
                        // No drift - use existing
                        Some(vlan)
                    }
                } else {
                    // Drift detection disabled - use existing
                    Some(vlan)
                }
            }
            DriftCheckResult::StatusCleared { message } => {
                // Emit event for drift detection
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DRIFT_DETECTED,
                    &format!("NetBoxVLAN {}/{} drift detected: {}", namespace, name, message),
                    vlan_crd,
                ).await;
                
                // Status was cleared - update it to Pending
                let status_patch = Self::create_resource_status_patch(
                    0, // Clear netbox_id
                    String::new(), // Clear URL
                    ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_vlan_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxVLAN status: {}", update_err);
                }
                // Fall through to creation
                None
            }
            DriftCheckResult::Recreate => {
                // Need to create - fall through
                None
            }
        };
        
        // Handle existing VLAN (from helper) or create new
        let netbox_vlan = match netbox_vlan {
            Some(vlan) => {
                // Resolve tag references
                let resolved_tags_json = self.resolve_tag_references(
                    netbox_client.as_ref(),
                    &vlan_crd.spec.tags,
                    namespace,
                    name,
                    None,
                    "NetBoxVLAN",
                ).await;
                
                // Convert resolved tags from Vec<serde_json::Value> to Vec<String>
                let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                
                // Update tags if they differ
                use netbox_client::VlanId;
                let vlan_id = vlan.id;
                let vlan_clone = vlan.clone();
                let vlan = match crate::reconcile_helpers::update_tags_if_differ(
                    vlan,
                    &vlan_crd.spec.tags,
                    resolved_tags.clone(),
                    |tags| async move {
                        netbox_client.update_vlan(
                            VlanId(vlan_id as u32),
                            None, // vid
                            None, // name
                            None, // site_id
                            None, // group_id
                            None, // tenant_id
                            None, // role_id
                            None, // status
                            None, // description
                            None, // comments
                            tags,
                        ).await
                    },
                    &format!("NetBoxVLAN {}/{}", namespace, name),
                ).await {
                    Ok(Some(updated)) => {
                        use crate::events::reasons;
                        self.record_event_normal(
                            reasons::UPDATED,
                            &format!("Updated NetBoxVLAN {}/{} tags in NetBox", namespace, name),
                            vlan_crd,
                        ).await;
                        updated
                    }
                    Ok(None) => vlan_clone, // Tags are up-to-date
                    Err(e) => {
                        warn!("Failed to update NetBoxVLAN {}/{} tags: {}", namespace, name, e);
                        vlan_clone // Use existing if update fails
                    }
                };
                
                vlan // Return existing VLAN (status update happens at end)
            }
            None => {
                // Need to create VLAN - resolve dependencies first using helpers
                use crate::reconcile_helpers::{validate_reference_kind, resolve_required_dependency_id, resolve_optional_dependency_id};
                
                // Resolve optional site ID
                // If site is specified in spec but not ready yet, return early to allow requeueing
                let site_id: Option<u64> = if vlan_crd.spec.site.is_some() {
                    let resolved_site_id: Option<u64> = resolve_optional_dependency_id(
                        &*self.netbox_site_api,
                        vlan_crd.spec.site.as_ref(),
                        "NetBoxSite",
                        "site",
                        name,
                        |crd| crd.status.as_ref(),
                    ).await;
                    
                    // If site is specified but not ready (None), return early for requeueing
                    if resolved_site_id.is_none() {
                        debug!("NetBoxVLAN {}/{}: Site '{}' has not been created in NetBox yet (no netbox_id in status). Will requeue when site is ready.", 
                            namespace, name, vlan_crd.spec.site.as_ref().unwrap().name);
                        return Ok(()); // Return early - controller will requeue when site status updates
                    }
                    resolved_site_id
                } else {
                    None // Site is truly optional
                };
                
                // Validate and resolve tenant ID (required)
                validate_reference_kind(&vlan_crd.spec.tenant, "NetBoxTenant", "tenant", name)?;
                let tenant_id = match resolve_required_dependency_id(
                    &*self.netbox_tenant_api,
                    &vlan_crd.spec.tenant.name,
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
                            &format!("Tenant '{}' not found or not ready: {}", vlan_crd.spec.tenant.name, e),
                            vlan_crd,
                        ).await;
                        return Err(e);
                    }
                };
                
                // Resolve optional role ID
                let _role_id = resolve_optional_dependency_id(
                    &*self.netbox_role_api,
                    vlan_crd.spec.role.as_ref(),
                    "NetBoxRole",
                    "role",
                    name,
                    |crd| crd.status.as_ref(),
                ).await;
                
                // Resolve tag references
                let resolved_tags_json = self.resolve_tag_references(
                    netbox_client.as_ref(),
                    &vlan_crd.spec.tags,
                    namespace,
                    name,
                    None,
                    "NetBoxVLAN",
                ).await;
                
                // Convert resolved tags from Vec<serde_json::Value> to Vec<String>
                let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                
                // Convert status enum to string
                let status_str = match vlan_crd.spec.status {
                    crds::VlanStatus::Active => Some("active"),
                    crds::VlanStatus::Reserved => Some("reserved"),
                    crds::VlanStatus::Deprecated => Some("deprecated"),
                };
                
                // Try to find existing VLAN by VID
                let existing_vlan = match netbox_client.query_vlans(
                    &[("vid", &vlan_crd.spec.vid.to_string())],
                    false,
                ).await {
                    Ok(vlans) => vlans.first().cloned(),
                    Err(_) => None
                };
                
                let netbox_vlan = if let Some(existing) = existing_vlan {
                    info!("VLAN {} already exists in NetBox (ID: {})", vlan_crd.spec.vid, existing.id);
                    // Update tags if they differ (idempotency path)
                    use netbox_client::VlanId;
                    let existing_id = existing.id;
                    let existing_clone = existing.clone();
                    match crate::reconcile_helpers::update_tags_if_differ(
                        existing,
                        &vlan_crd.spec.tags,
                        resolved_tags.clone(),
                        |tags| async move {
                            netbox_client.update_vlan(
                                VlanId(existing_id as u32),
                                None, // vid
                                None, // name
                                None, // site_id
                                None, // group_id
                                None, // tenant_id
                                None, // role_id
                                None, // status
                                None, // description
                                None, // comments
                                tags,
                            ).await
                        },
                        &format!("NetBoxVLAN {}/{}", namespace, name),
                    ).await {
                        Ok(Some(updated)) => {
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::UPDATED,
                                &format!("Updated NetBoxVLAN {}/{} tags in NetBox", namespace, name),
                                vlan_crd,
                            ).await;
                            updated
                        }
                        Ok(None) => existing_clone, // Tags are up-to-date
                        Err(e) => {
                            warn!("Failed to update NetBoxVLAN {}/{} tags: {}", namespace, name, e);
                            existing_clone // Use existing if update fails
                        }
                    }
                } else {
                    // Create VLAN - site is optional (only required if specified in spec)
                    match netbox_client.create_vlan(
                        vlan_crd.spec.vid,
                        &vlan_crd.spec.name,
                        site_id.map(SiteId),
                        None, // group_id
                        Some(TenantId(tenant_id)),
                        None, // role_id
                        status_str, // status_str is already Option<&str>
                        vlan_crd.spec.description.clone(),
                        None, // comments
                        resolved_tags.clone(),
                    ).await {
                        Ok(created) => {
                            info!("Created VLAN {} ({}) in NetBox (ID: {})", created.vid, created.name, created.id);
                            // Emit event for successful creation
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::CREATED,
                                &format!("Created VLAN {} ({}) in NetBox (ID: {})", created.vid, created.name, created.id),
                                vlan_crd,
                            ).await;
                            created
                        }
                        Err(e) => {
                            // Handle CREATE conflicts using shared helper (GitOps idempotency)
                            use crate::reconcile_helpers::is_conflict_error;
                            
                            if is_conflict_error(&e) {
                                warn!("VLAN {} creation failed with conflict, attempting to retrieve existing VLAN (idempotency)", vlan_crd.spec.vid);
                                
                                // Try multiple query strategies
                                let mut found_vlan = None;
                                
                                // Strategy 1: Query by VID
                                match netbox_client.query_vlans(
                                    &[("vid", &vlan_crd.spec.vid.to_string())],
                                    false,
                                ).await {
                                    Ok(vlans) => {
                                        if let Some(vlan) = vlans.first() {
                                            info!("Found existing VLAN by VID {} in NetBox (ID: {}) after conflict", vlan_crd.spec.vid, vlan.id);
                                            found_vlan = Some(vlan.clone());
                                        }
                                    }
                                    Err(_) => {}
                                }
                                
                                // Strategy 2: Query by name if not found
                                if found_vlan.is_none() {
                                    match netbox_client.query_vlans(
                                        &[("name", &vlan_crd.spec.name)],
                                        false,
                                    ).await {
                                        Ok(vlans) => {
                                            if let Some(vlan) = vlans.first() {
                                                info!("Found existing VLAN by name '{}' in NetBox (ID: {}) after conflict", vlan_crd.spec.name, vlan.id);
                                                found_vlan = Some(vlan.clone());
                                            }
                                        }
                                        Err(_) => {}
                                    }
                                }
                                
                                // Strategy 3: Fallback - query all VLANs and filter
                                if found_vlan.is_none() {
                                    match netbox_client.query_vlans(&[], true).await {
                                        Ok(all_vlans) => {
                                            if let Some(vlan) = all_vlans.iter().find(|v| {
                                                v.vid == vlan_crd.spec.vid as u16 || v.name == vlan_crd.spec.name
                                            }) {
                                                info!("Found existing VLAN in NetBox (ID: {}) via fallback query", vlan.id);
                                                found_vlan = Some(vlan.clone());
                                            }
                                        }
                                        Err(_) => {}
                                    }
                                }
                                
                                if let Some(found) = found_vlan {
                                    info!("Found existing VLAN {} (VID: {}) in NetBox (ID: {}) via conflict resolution (idempotency)", found.name, found.vid, found.id);
                                    // Update tags if they differ (idempotency path)
                                    use netbox_client::VlanId;
                                    let found_id = found.id;
                                    let found_clone = found.clone();
                                    match crate::reconcile_helpers::update_tags_if_differ(
                                        found,
                                        &vlan_crd.spec.tags,
                                        resolved_tags.clone(),
                                        |tags| async move {
                                            netbox_client.update_vlan(
                                                VlanId(found_id as u32),
                                                None, // vid
                                                None, // name
                                                None, // site_id
                                                None, // group_id
                                                None, // tenant_id
                                                None, // role_id
                                                None, // status
                                                None, // description
                                                None, // comments
                                                tags,
                                            ).await
                                        },
                                        &format!("NetBoxVLAN {}/{}", namespace, name),
                                    ).await {
                                        Ok(Some(updated)) => {
                                            use crate::events::reasons;
                                            self.record_event_normal(
                                                reasons::UPDATED,
                                                &format!("Updated NetBoxVLAN {}/{} tags in NetBox", namespace, name),
                                                vlan_crd,
                                            ).await;
                                            updated
                                        }
                                        Ok(None) => found_clone, // Tags are up-to-date
                                        Err(e) => {
                                            warn!("Failed to update NetBoxVLAN {}/{} tags: {}", namespace, name, e);
                                            found_clone // Use existing if update fails
                                        }
                                    }
                                } else {
                                    let error_msg = format!("VLAN {} already exists in NetBox but could not retrieve it: {}", vlan_crd.spec.vid, e);
                                    error!("{}", error_msg);
                                    return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                                }
                            } else {
                                // Not a conflict, return original error
                                let error_msg = format!("Failed to create VLAN in NetBox: {}", e);
                                error!("{}", error_msg);
                                return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                            }
                        }
                    }
                };
                
                netbox_vlan
            }
        };
        
        // Update status (use PascalCase state to match CRD validation schema)
        use crate::reconcile_helpers::update_resource_status;
        let status_patch = Self::create_resource_status_patch(
            netbox_vlan.id,
            netbox_vlan.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_vlan_api,
            name,
            namespace,
            &status_patch,
            "NetBoxVLAN",
            netbox_vlan.id,
        ).await?;
        info!("Updated NetBoxVLAN {}/{} status: NetBox ID {}", namespace, name, netbox_vlan.id);
        Ok(())
    }
}
