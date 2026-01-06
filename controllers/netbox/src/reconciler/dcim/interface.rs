//! NetBoxInterface reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxInterface, ResourceState};
use netbox_client::{NetBoxClientTrait, InterfaceId, DeviceId};

impl Reconciler {
    fn interface_needs_update(
        spec: &crds::NetBoxInterfaceSpec,
        existing: &netbox_client::Interface,
        desired_device_id: u64,
    ) -> bool {
        use crate::reconcile_helpers::{
            compare_string_field,
            compare_optional_string_field,
            compare_optional_numeric_field,
        };
        
        let existing_device_id = existing.device.id;
        
        // Evaluate all comparisons to log all field differences (no short-circuit)
        // Store each result in a variable, then OR them at the end to ensure all comparisons are evaluated
        let device_diff = existing_device_id != desired_device_id;
        let name_diff = compare_string_field(&spec.name, &existing.name);
        let type_diff = compare_string_field(&spec.r#type, &existing.r#type);
        let enabled_diff = spec.enabled != existing.enabled;
        let mac_diff = compare_optional_string_field(&spec.mac_address, &existing.mac_address);
        let mtu_diff = compare_optional_numeric_field(&spec.mtu, &existing.mtu);
        let description_diff = compare_optional_string_field(&spec.description, &existing.description);
        let comments_diff = compare_optional_string_field(&spec.comments, &existing.comments);
        // Tags are handled separately
        
        // OR all results together (all comparisons already evaluated above)
        device_diff || name_diff || type_diff || enabled_diff || mac_diff || mtu_diff || description_diff || comments_diff
    }

    pub async fn reconcile_netbox_interface(&self, interface_crd: &NetBoxInterface) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::extract_name_and_namespace;
        let (name, namespace) = extract_name_and_namespace(interface_crd, "NetBoxInterface")?;
        
        info!("Reconciling NetBoxInterface {}/{}", namespace, name);
        
        // Get tenant from parent Device
        let device_name = &interface_crd.spec.device;
        let device_crd = match self.netbox_device_api.get(device_name).await {
            Ok(device) => device,
            Err(e) => {
                let error_msg = format!("Device CRD '{}' not found for interface {}: {}", device_name, name, e);
                error!("{}", error_msg);
                // Emit event for dependency not found
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DEPENDENCY_NOT_FOUND,
                    &error_msg,
                    interface_crd,
                ).await;
                return Err(ControllerError::InvalidConfig(error_msg));
            }
        };
        
        // Get tenant from device
        let tenant_ref = &device_crd.spec.tenant;
        let netbox_client = self.token_resolver
            .create_client_for_tenant(namespace, tenant_ref)
            .await?;
        
        // Resolve device ID (required)
        // If device hasn't been created yet, return early and let controller requeue when device is ready
        use crate::reconcile_helpers::resolve_dependency_id;
        let device_id: u64 = match resolve_dependency_id(
            device_crd.status.as_ref(),
            "Device",
            device_name,
        ) {
            Some(id) => id,
            None => return Ok(()), // Return early - controller will requeue when device status updates
        };
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                interface_crd.status.as_ref(),
                "NetBoxInterface",
                namespace,
                name,
                |netbox_id: u64| async move {
                    netbox_client_ref.get_interface(InterfaceId(netbox_id)).await
                },
            ).await?
        };
        
        // Check if drift detection is enabled (defaults to true)
        let drift_detection_enabled = interface_crd.spec.drift_detection.unwrap_or(true);
        
        let netbox_interface = match drift_result {
            DriftCheckResult::UseExisting(interface) => {
                // Check for field drift if enabled
                if drift_detection_enabled {
                    if Self::interface_needs_update(&interface_crd.spec, &interface, device_id) {
                        // Field drift detected - update NetBox to match CRD (Git is source of truth)
                        warn!("NetBoxInterface {}/{} fields differ from CRD spec, updating to match Git", namespace, name);
                        use crate::events::reasons;
                        self.record_event_warning(
                            reasons::DRIFT_DETECTED,
                            &format!("NetBoxInterface {}/{} fields differ from CRD, updating to match Git", namespace, name),
                            interface_crd,
                        ).await;
                        
                        // Resolve tags for update
                        let resolved_tags_json = self.resolve_tag_references(
                            netbox_client.as_ref(),
                            &interface_crd.spec.tags,
                            namespace,
                            name,
                            Some(interface.id),
                            "NetBoxInterface",
                        ).await;
                        let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                        
                        match netbox_client.update_interface(
                            InterfaceId(interface.id),
                            Some(&interface_crd.spec.name),
                            Some(&interface_crd.spec.r#type),
                            Some(interface_crd.spec.enabled),
                            interface_crd.spec.mac_address.as_deref(),
                            interface_crd.spec.mtu,
                            interface_crd.spec.description.clone(),
                            interface_crd.spec.comments.clone(),
                            resolved_tags,
                        ).await {
                            Ok(updated) => {
                                use crate::events::reasons;
                                self.record_event_normal(
                                    reasons::UPDATED,
                                    &format!("Updated NetBoxInterface {}/{} in NetBox to match CRD (ID: {})", namespace, name, updated.id),
                                    interface_crd,
                                ).await;
                                Some(updated)
                            }
                            Err(e) => {
                                error!("Failed to update NetBoxInterface {}/{} in NetBox: {}", namespace, name, e);
                                Some(interface) // Use existing if update fails
                            }
                        }
                    } else {
                        // No drift - use existing
                        Some(interface)
                    }
                } else {
                    // Drift detection disabled - use existing
                    Some(interface)
                }
            }
            DriftCheckResult::StatusCleared { message } => {
                // Status was cleared - update it to Pending
                let status_patch = Self::create_typed_interface_status_patch(
                    0, String::new(), ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_interface_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxInterface status: {}", update_err);
                }
                // Fall through to creation
                None
            }
            DriftCheckResult::Recreate => {
                // Need to create - fall through
                None
            }
        };
        
        let netbox_interface = match netbox_interface {
            Some(interface) => {
                // Update tags if they differ (tags are handled separately from field drift)
                let interface_id = interface.id;
                let interface_clone = interface.clone();
                let resolved_tags_json = self.resolve_tag_references(
                    netbox_client.as_ref(),
                    &interface_crd.spec.tags,
                    namespace,
                    name,
                    Some(interface_id),
                    "NetBoxInterface",
                ).await;
                let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                
                let interface = match crate::reconcile_helpers::update_tags_if_differ(
                    interface,
                    &interface_crd.spec.tags,
                    resolved_tags.clone(),
                    |tags| {
                        let device_id_clone = device_id;
                        let name_clone = interface_crd.spec.name.clone();
                        let type_clone = interface_crd.spec.r#type.clone();
                        let enabled_clone = interface_crd.spec.enabled;
                        let mac_address_clone = interface_crd.spec.mac_address.clone();
                        let mtu_clone = interface_crd.spec.mtu;
                        let description_clone = interface_crd.spec.description.clone();
                        let comments_clone = interface_crd.spec.comments.clone();
                        async move {
                            netbox_client.update_interface(
                                InterfaceId(interface_id),
                                Some(&name_clone),
                                Some(&type_clone),
                                Some(enabled_clone),
                                mac_address_clone.as_deref(),
                                mtu_clone,
                                description_clone,
                                comments_clone,
                                tags,
                            ).await
                        }
                    },
                    &format!("NetBoxInterface {}/{}", namespace, name),
                ).await {
                    Ok(Some(updated)) => {
                        use crate::events::reasons;
                        self.record_event_normal(
                            reasons::UPDATED,
                            &format!("Updated NetBoxInterface {}/{} tags in NetBox", namespace, name),
                            interface_crd,
                        ).await;
                        updated
                    }
                    Ok(None) => interface_clone, // Tags are up-to-date
                    Err(e) => {
                        warn!("Failed to update NetBoxInterface {}/{} tags: {}", namespace, name, e);
                        interface_clone // Use existing if update fails
                    }
                };
                
                interface // Return existing Interface (status update happens at end)
            }
            None => {
                // Try to find existing interface by querying device interfaces
                let existing_interface = match netbox_client.query_interfaces(&[("device_id", &device_id.to_string()), ("name", &interface_crd.spec.name)], false).await {
                    Ok(mut interfaces) => {
                        interfaces.pop()
                    }
                    Err(e) => {
                        warn!("Failed to query interfaces: {}, will try to create", e);
                        None
                    }
                };
                
                if let Some(existing) = existing_interface {
                    info!("Interface {} on device {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", interface_crd.spec.name, device_name, existing.id);
                    existing
                } else {
                    debug!("Attempting to create interface {} on device {} in NetBox", interface_crd.spec.name, device_name);
                    match netbox_client.create_interface(
                        DeviceId(device_id),
                        &interface_crd.spec.name,
                        &interface_crd.spec.r#type,
                        Some(interface_crd.spec.enabled),
                        interface_crd.spec.mac_address.as_deref(),
                        interface_crd.spec.mtu,
                        interface_crd.spec.description.clone(),
                        interface_crd.spec.comments.clone(),
                    ).await {
                        Ok(created) => {
                            info!("Created interface {} on device {} in NetBox (ID: {})", created.name, device_name, created.id);
                            // Emit event for successful creation
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::CREATED,
                                &format!("Created interface {} on device {} in NetBox (ID: {})", created.name, device_name, created.id),
                                interface_crd,
                            ).await;
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create interface in NetBox: {}", e);
                            error!("{}", error_msg);
                            // Emit event for reconciliation failure
                            use crate::events::reasons;
                            self.record_event_warning(
                                reasons::RECONCILIATION_FAILED,
                                &error_msg,
                                interface_crd,
                            ).await;
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                }
            }
        };
        
        use crate::reconcile_helpers::update_resource_status;
        let status_patch = Self::create_typed_interface_status_patch(
            netbox_interface.id,
            netbox_interface.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_interface_api,
            name,
            namespace,
            &status_patch,
            "NetBoxInterface",
            netbox_interface.id,
        ).await?;
        info!("Updated NetBoxInterface {}/{} status: NetBox ID {}", namespace, name, netbox_interface.id);
        Ok(())
    }
}
