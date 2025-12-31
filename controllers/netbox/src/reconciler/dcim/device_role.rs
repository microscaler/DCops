//! NetBoxDeviceRole reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxDeviceRole, ResourceState};
use netbox_client::NetBoxClientTrait;

impl Reconciler {
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
        
        let netbox_device_role = match netbox_device_role {
            Some(device_role) => {
                // Always resolve tags (even if nothing else changed, tags might need updating)
                let resolved_tags_json = self.resolve_tag_references(
                    netbox_client.as_ref(),
                    &device_role_crd.spec.tags,
                    namespace,
                    name,
                ).await;
                
                // Convert resolved tags from Vec<serde_json::Value> to Vec<String>
                let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                
                // Update tags if they differ
                use netbox_client::DeviceRoleId;
                let device_role_id = device_role.id;
                let device_role_clone = device_role.clone();
                let device_role = match crate::reconcile_helpers::update_tags_if_differ(
                    device_role,
                    &device_role_crd.spec.tags,
                    resolved_tags.clone(),
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
                };
                
                // Check if status needs updating
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    device_role_crd.status.as_ref(),
                    device_role.id,
                    &device_role.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_typed_device_role_status_patch(
                        device_role.id,
                        device_role.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_device_role_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxDeviceRole {}/{} status: NetBox ID {}", namespace, name, device_role.id);
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update NetBoxDeviceRole status: {}", e);
                            error!("{}", error_msg);
                            // Emit event for reconciliation failure
                            use crate::events::reasons;
                            self.record_event_warning(
                                reasons::RECONCILIATION_FAILED,
                                &error_msg,
                                device_role_crd,
                            ).await;
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxDeviceRole {}/{} already has correct status (ID: {}), skipping update", namespace, name, device_role.id);
                    return Ok(());
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
