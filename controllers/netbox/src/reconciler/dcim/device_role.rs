//! NetBoxDeviceRole reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxDeviceRole, ResourceState};

impl Reconciler {
    fn device_role_needs_update(
        spec: &crds::NetBoxDeviceRoleSpec,
        existing: &netbox_client::DeviceRole,
    ) -> bool {
        use crate::reconcile_helpers::{
            compare_string_field,
            compare_slug_field,
            compare_optional_string_field,
        };
        
        let auto_generated_slug = spec.name.to_lowercase().replace(' ', "-");
        
        compare_string_field(&spec.name, &existing.name)
            || compare_slug_field(&spec.slug, &existing.slug, auto_generated_slug)
            || compare_optional_string_field(&spec.color, &existing.color)
            || spec.vm_role != existing.vm_role
            || compare_optional_string_field(&spec.description, &existing.description)
            || compare_optional_string_field(&spec.comments, &existing.comments)
        // Tags are handled separately
    }

    pub async fn reconcile_netbox_device_role(&self, device_role_crd: &NetBoxDeviceRole) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::extract_name_and_namespace;
        let (name, namespace) = extract_name_and_namespace(device_role_crd, "NetBoxDeviceRole")?;
        
        info!("Reconciling NetBoxDeviceRole {}/{}", namespace, name);
        
        // Get client for shared resource (finds tenant from referencing Devices)
        let netbox_client = self.token_resolver
            .create_client_for_shared_resource(namespace, "NetBoxDeviceRole", name)
            .await
            .map_err(|e| ControllerError::TokenResolution(e))?;
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                device_role_crd.status.as_ref(),
                "NetBoxDeviceRole",
                namespace,
                name,
                |netbox_id: u64| async move {
                    let id_str = netbox_id.to_string();
                    netbox_client_ref.query_device_roles(&[("id", &id_str)], false)
                        .await
                        .and_then(|mut device_roles| {
                            device_roles.pop().ok_or_else(|| netbox_client::NetBoxError::NotFound(format!("DeviceRole {} not found", netbox_id)))
                        })
                },
            ).await?
        };
        
        let netbox_device_role = match drift_result {
            DriftCheckResult::UseExisting(device_role) => Some(device_role),
            DriftCheckResult::StatusCleared { message } => {
                // Emit event for drift detection
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DRIFT_DETECTED,
                    &format!("NetBoxDeviceRole {}/{} drift detected: {}", namespace, name, message),
                    device_role_crd,
                ).await;
                
                let status_patch = Self::create_typed_device_role_status_patch(
                    0, String::new(), ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_device_role_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxDeviceRole status: {}", update_err);
                }
                None
            }
            DriftCheckResult::Recreate => None,
        };
        
        // Check if drift detection is enabled (defaults to true)
        let drift_detection_enabled = device_role_crd.spec.drift_detection.unwrap_or(true);
        
        let netbox_device_role = match netbox_device_role {
            Some(device_role) => {
                // Check for field drift if enabled
                if drift_detection_enabled {
                    if Self::device_role_needs_update(&device_role_crd.spec, &device_role) {
                        // Field drift detected - update NetBox to match CRD (Git is source of truth)
                        warn!("NetBoxDeviceRole {}/{} fields differ from CRD spec, updating to match Git", namespace, name);
                        use crate::events::reasons;
                        self.record_event_warning(
                            reasons::DRIFT_DETECTED,
                            &format!("NetBoxDeviceRole {}/{} fields differ from CRD, updating to match Git", namespace, name),
                            device_role_crd,
                        ).await;
                        
                        // Resolve tags for update
                        let resolved_tags_json = self.resolve_tag_references(
                            netbox_client.as_ref(),
                            &device_role_crd.spec.tags,
                            namespace,
                            name,
                            Some(device_role.id),
                        ).await;
                        let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                        
                        use netbox_client::DeviceRoleId;
                        match netbox_client.update_device_role(
                            DeviceRoleId(device_role.id),
                            Some(&device_role_crd.spec.name),
                            device_role_crd.spec.slug.as_deref(),
                            device_role_crd.spec.color.as_deref(),
                            Some(device_role_crd.spec.vm_role),
                            device_role_crd.spec.description.clone(),
                            device_role_crd.spec.comments.clone(),
                            resolved_tags,
                        ).await {
                            Ok(updated) => {
                                info!("Updated NetBoxDeviceRole {}/{} in NetBox to match CRD (ID: {})", namespace, name, updated.id);
                                use crate::events::reasons;
                                self.record_event_normal(
                                    reasons::UPDATED,
                                    &format!("Updated NetBoxDeviceRole {}/{} in NetBox to match CRD", namespace, name),
                                    device_role_crd,
                                ).await;
                                updated
                            }
                            Err(e) => {
                                error!("Failed to update NetBoxDeviceRole {}/{} in NetBox: {}", namespace, name, e);
                                use crate::events::reasons;
                                self.record_event_warning(
                                    reasons::RECONCILIATION_FAILED,
                                    &format!("Failed to update NetBoxDeviceRole {}/{} in NetBox: {}", namespace, name, e),
                                    device_role_crd,
                                ).await;
                                // Continue with existing device_role - don't fail reconciliation
                                device_role
                            }
                        }
                    } else {
                        // No field drift - check tags
                        let resolved_tags_json = self.resolve_tag_references(
                            netbox_client.as_ref(),
                            &device_role_crd.spec.tags,
                            namespace,
                            name,
                            Some(device_role.id),
                        ).await;
                        let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                        
                        use netbox_client::DeviceRoleId;
                        let device_role_id = device_role.id;
                        let device_role_clone = device_role.clone();
                        match crate::reconcile_helpers::update_tags_if_differ(
                            device_role,
                            &device_role_crd.spec.tags,
                            resolved_tags,
                            |tags| async move {
                                netbox_client.update_device_role(
                                    DeviceRoleId(device_role_id),
                                    Some(&device_role_crd.spec.name),
                                    device_role_crd.spec.slug.as_deref(),
                                    device_role_crd.spec.color.as_deref(),
                                    Some(device_role_crd.spec.vm_role),
                                    device_role_crd.spec.description.clone(),
                                    device_role_crd.spec.comments.clone(),
                                    tags,
                                ).await
                            },
                            &format!("NetBoxDeviceRole {}/{}", namespace, name),
                        ).await {
                            Ok(Some(updated)) => {
                                use crate::events::reasons;
                                self.record_event_normal(
                                    reasons::UPDATED,
                                    &format!("Updated NetBoxDeviceRole {}/{} tags in NetBox", namespace, name),
                                    device_role_crd,
                                ).await;
                                updated
                            }
                            Ok(None) => device_role_clone, // Tags are up-to-date
                            Err(e) => {
                                warn!("Failed to update NetBoxDeviceRole {}/{} tags: {}", namespace, name, e);
                                device_role_clone // Use existing if update fails
                            }
                        }
                    }
                } else {
                    // Drift detection disabled - only check tags
                    let resolved_tags_json = self.resolve_tag_references(
                        netbox_client.as_ref(),
                        &device_role_crd.spec.tags,
                        namespace,
                        name,
                        Some(device_role.id),
                    ).await;
                    let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                    
                    use netbox_client::DeviceRoleId;
                    let device_role_id = device_role.id;
                    let device_role_clone = device_role.clone();
                    match crate::reconcile_helpers::update_tags_if_differ(
                        device_role,
                        &device_role_crd.spec.tags,
                        resolved_tags,
                        |tags| async move {
                            netbox_client.update_device_role(
                                DeviceRoleId(device_role_id),
                                Some(&device_role_crd.spec.name),
                                device_role_crd.spec.slug.as_deref(),
                                device_role_crd.spec.color.as_deref(),
                                Some(device_role_crd.spec.vm_role),
                                device_role_crd.spec.description.clone(),
                                device_role_crd.spec.comments.clone(),
                                tags,
                            ).await
                        },
                        &format!("NetBoxDeviceRole {}/{}", namespace, name),
                    ).await {
                        Ok(Some(updated)) => {
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::UPDATED,
                                &format!("Updated NetBoxDeviceRole {}/{} tags in NetBox", namespace, name),
                                device_role_crd,
                            ).await;
                            updated
                        }
                        Ok(None) => device_role_clone, // Tags are up-to-date
                        Err(e) => {
                            warn!("Failed to update NetBoxDeviceRole {}/{} tags: {}", namespace, name, e);
                            device_role_clone // Use existing if update fails
                        }
                    }
                }
            }
            None => {
                let existing_device_role = match netbox_client.get_device_role_by_name(&device_role_crd.spec.name).await {
                    Ok(Some(dr)) => {
                        info!("DeviceRole {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", device_role_crd.spec.name, dr.id);
                        Some(dr)
                    }
                    Ok(None) => None,
                    Err(e) => {
                        warn!("Failed to query device role by name: {}, will try to create", e);
                        None
                    }
                };
                
                if let Some(existing) = existing_device_role {
                    // Resource exists but no status - check if tags need updating
                    let resolved_tags_json = self.resolve_tag_references(
                        netbox_client.as_ref(),
                        &device_role_crd.spec.tags,
                        namespace,
                        name,
                    None,
                ).await;
                    let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                    
                    // Update tags if they differ
                    use netbox_client::DeviceRoleId;
                    let existing_id = existing.id;
                    let existing_clone = existing.clone();
                    match crate::reconcile_helpers::update_tags_if_differ(
                        existing,
                        &device_role_crd.spec.tags,
                        resolved_tags,
                        |tags| async move {
                            netbox_client.update_device_role(
                                DeviceRoleId(existing_id),
                                Some(&device_role_crd.spec.name),
                                device_role_crd.spec.slug.as_deref(),
                                device_role_crd.spec.color.as_deref(),
                                Some(device_role_crd.spec.vm_role),
                                device_role_crd.spec.description.clone(),
                                device_role_crd.spec.comments.clone(),
                                tags,
                            ).await
                        },
                        &format!("NetBoxDeviceRole {}/{} (idempotency path)", namespace, name),
                    ).await {
                        Ok(Some(updated)) => {
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::UPDATED,
                                &format!("Updated NetBoxDeviceRole {}/{} tags in NetBox", namespace, name),
                                device_role_crd,
                            ).await;
                            updated
                        }
                        Ok(None) => existing_clone, // Tags are up-to-date
                        Err(e) => {
                            warn!("Failed to update NetBoxDeviceRole {}/{} tags: {}", namespace, name, e);
                            existing_clone // Use existing if update fails
                        }
                    }
                } else {
                    // Resolve tags before create
                    let resolved_tags_json = self.resolve_tag_references(
                        netbox_client.as_ref(),
                        &device_role_crd.spec.tags,
                        namespace,
                        name,
                    None,
                ).await;
                    let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                    
                    debug!("Attempting to create device role {} in NetBox", device_role_crd.spec.name);
                    match netbox_client.create_device_role(
                        &device_role_crd.spec.name,
                        device_role_crd.spec.slug.as_deref(),
                        device_role_crd.spec.color.as_deref(),
                        Some(device_role_crd.spec.vm_role),
                        device_role_crd.spec.description.clone(),
                        device_role_crd.spec.comments.clone(),
                        resolved_tags,
                    ).await {
                        Ok(created) => {
                            info!("Created device role {} in NetBox (ID: {})", created.name, created.id);
                            // Emit event for successful creation
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::CREATED,
                                &format!("Created device role {} in NetBox (ID: {})", created.name, created.id),
                                device_role_crd,
                            ).await;
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create device role in NetBox: {}", e);
                            error!("{}", error_msg);
                            // Emit event for reconciliation failure
                            use crate::events::reasons;
                            self.record_event_warning(
                                reasons::RECONCILIATION_FAILED,
                                &error_msg,
                                device_role_crd,
                            ).await;
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                }
            }
        };
        
        use crate::reconcile_helpers::update_resource_status;
        let status_patch = Self::create_typed_device_role_status_patch(
            netbox_device_role.id,
            netbox_device_role.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_device_role_api,
            name,
            namespace,
            &status_patch,
            "NetBoxDeviceRole",
            netbox_device_role.id,
        ).await?;
        info!("Updated NetBoxDeviceRole {}/{} status: NetBox ID {}", namespace, name, netbox_device_role.id);
        Ok(())
    }
}
