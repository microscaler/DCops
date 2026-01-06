//! NetBoxMACAddress reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxMACAddress, ResourceState};
use netbox_client::NetBoxClientTrait;

impl Reconciler {
    fn mac_address_needs_update(
        spec: &crds::NetBoxMACAddressSpec,
        existing: &netbox_client::MACAddress,
        desired_interface_id: u64,
    ) -> bool {
        use crate::reconcile_helpers::{
            compare_optional_string_field,
            compare_optional_dependency_id,
        };
        
        let existing_interface_id = existing.assigned_object_id;
        
        // Evaluate all comparisons to log all field differences (no short-circuit)
        let interface_diff = compare_optional_dependency_id(Some(desired_interface_id), existing_interface_id);
        let description_diff = compare_optional_string_field(&spec.description, &existing.description);
        let comments_diff = compare_optional_string_field(&spec.comments, &existing.comments);
        // Tags are handled separately
        // Note: mac_address field is immutable in NetBox
        
        // OR all results together (no short-circuit - all comparisons evaluated)
        // Store each result in a variable, then OR them at the end to ensure all comparisons are evaluated
        interface_diff || description_diff || comments_diff
    }

    pub async fn reconcile_netbox_mac_address(&self, mac_address_crd: &NetBoxMACAddress) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::extract_name_and_namespace;
        let (name, namespace) = extract_name_and_namespace(mac_address_crd, "NetBoxMACAddress")?;
        
        info!("Reconciling NetBoxMACAddress {}/{}", namespace, name);
        
        // Parse interface reference (format: "<device-name>/<interface-name>")
        let interface_parts: Vec<&str> = mac_address_crd.spec.interface.split('/').collect();
        if interface_parts.len() != 2 {
            return Err(ControllerError::InvalidConfig(
                format!("Invalid interface format '{}' in MAC address {}, expected '<device-name>/<interface-name>'", mac_address_crd.spec.interface, name)
            ));
        }
        let device_name = interface_parts[0];
        let interface_name = interface_parts[1];
        
        // Get tenant from parent Device
        let device_crd = match self.netbox_device_api.get(device_name).await {
            Ok(device) => device,
            Err(e) => {
                let error_msg = format!("Device CRD '{}' not found for MAC address {}: {}", device_name, name, e);
                error!("{}", error_msg);
                // Emit event for dependency not found
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DEPENDENCY_NOT_FOUND,
                    &error_msg,
                    mac_address_crd,
                ).await;
                return Err(ControllerError::InvalidConfig(error_msg));
            }
        };
        
        // Get tenant from device
        let tenant_ref = &device_crd.spec.tenant;
        let netbox_client = self.token_resolver
            .create_client_for_tenant(namespace, tenant_ref)
            .await?;
        
        // Resolve device ID to ensure device has been created in NetBox
        // If device hasn't been created yet, return early and let controller requeue when device is ready
        use crate::reconcile_helpers::resolve_dependency_id;
        let _device_id: u64 = match resolve_dependency_id(
            device_crd.status.as_ref(),
            "Device",
            device_name,
        ) {
            Some(id) => id,
            None => {
                debug!("NetBoxMACAddress {}/{}: Device '{}' has not been created in NetBox yet (no netbox_id in status). Will requeue when device is ready.", namespace, name, device_name);
                return Ok(()); // Return early - controller will requeue when device status updates
            }
        };
        
        // Check if interface CRD exists and has been created in NetBox
        // Interface CRD name format: "<device-name>-<interface-name>"
        let interface_crd_name = format!("{}-{}", device_name, interface_name);
        let interface_crd = match self.netbox_interface_api.get(&interface_crd_name).await {
            Ok(interface) => interface,
            Err(e) => {
                let error_msg = format!("Interface CRD '{}' not found for MAC address {}: {}", interface_crd_name, name, e);
                error!("{}", error_msg);
                // Emit event for dependency not found
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DEPENDENCY_NOT_FOUND,
                    &error_msg,
                    mac_address_crd,
                ).await;
                return Err(ControllerError::InvalidConfig(error_msg));
            }
        };
        
        // Resolve interface ID from interface CRD status
        // If interface hasn't been created yet, return early and let controller requeue when interface is ready
        let interface_id: u64 = match resolve_dependency_id(
            interface_crd.status.as_ref(),
            "Interface",
            &interface_crd_name,
        ) {
            Some(id) => id,
            None => {
                debug!("NetBoxMACAddress {}/{}: Interface '{}' (CRD: {}) has not been created in NetBox yet (no netbox_id in status). Will requeue when interface is ready.", namespace, name, interface_name, interface_crd_name);
                return Ok(()); // Return early - controller will requeue when interface status updates
            }
        };
        
        // Get interface from NetBox using the resolved ID
        use netbox_client::InterfaceId;
        let interface = match netbox_client.get_interface(InterfaceId(interface_id)).await {
            Ok(interface) => interface,
            Err(e) => {
                let error_msg = format!("Failed to get interface {} (ID: {}) from NetBox: {}", interface_name, interface_id, e);
                error!("{}", error_msg);
                return Err(ControllerError::NetBox(e));
            }
        };
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let mac_address = mac_address_crd.spec.mac_address.clone();
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                mac_address_crd.status.as_ref(),
                "NetBoxMACAddress",
                namespace,
                name,
                |_netbox_id| async move {
                    netbox_client_ref.get_mac_address_by_address(&mac_address)
                        .await
                        .and_then(|opt| opt.ok_or_else(|| netbox_client::NetBoxError::NotFound(format!("MAC address {} not found", mac_address))))
                },
            ).await?
        };
        
        // Check if drift detection is enabled (defaults to true)
        let drift_detection_enabled = mac_address_crd.spec.drift_detection.unwrap_or(true);
        
        let netbox_mac_address = match drift_result {
            DriftCheckResult::UseExisting(mac_address) => {
                // Check for field drift if enabled
                if drift_detection_enabled {
                    if Self::mac_address_needs_update(&mac_address_crd.spec, &mac_address, interface_id) {
                        // Field drift detected - update NetBox to match CRD (Git is source of truth)
                        warn!("NetBoxMACAddress {}/{} fields differ from CRD spec, updating to match Git", namespace, name);
                        use crate::events::reasons;
                        self.record_event_warning(
                            reasons::DRIFT_DETECTED,
                            &format!("NetBoxMACAddress {}/{} fields differ from CRD, updating to match Git", namespace, name),
                            mac_address_crd,
                        ).await;
                        
                        // Resolve tags for update
                        let resolved_tags_json = self.resolve_tag_references(
                            netbox_client.as_ref(),
                            &mac_address_crd.spec.tags,
                            namespace,
                            name,
                            Some(mac_address.id),
                            "NetBoxMACAddress",
                        ).await;
                        let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                        
                        match netbox_client.update_mac_address(
                            mac_address.id,
                            Some("dcim.interface"),
                            Some(interface_id),
                            mac_address_crd.spec.description.clone(),
                            mac_address_crd.spec.comments.clone(),
                            resolved_tags,
                        ).await {
                            Ok(updated) => {
                                use crate::events::reasons;
                                self.record_event_normal(
                                    reasons::UPDATED,
                                    &format!("Updated NetBoxMACAddress {}/{} in NetBox to match CRD (ID: {})", namespace, name, updated.id),
                                    mac_address_crd,
                                ).await;
                                Some(updated)
                            }
                            Err(e) => {
                                error!("Failed to update NetBoxMACAddress {}/{} in NetBox: {}", namespace, name, e);
                                Some(mac_address) // Use existing if update fails
                            }
                        }
                    } else {
                        // No drift - use existing
                        Some(mac_address)
                    }
                } else {
                    // Drift detection disabled - use existing
                    Some(mac_address)
                }
            }
            DriftCheckResult::StatusCleared { message } => {
                // Emit event for drift detection
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DRIFT_DETECTED,
                    &format!("NetBoxMACAddress {}/{} drift detected: {}", namespace, name, message),
                    mac_address_crd,
                ).await;
                
                let status_patch = Self::create_typed_mac_address_status_patch(
                    0, String::new(), ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_mac_address_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxMACAddress status: {}", update_err);
                }
                None
            }
            DriftCheckResult::Recreate => None,
        };
        
        let netbox_mac_address = match netbox_mac_address {
            Some(mac_address) => {
                // Update tags if they differ (tags are handled separately from field drift)
                let mac_address_id = mac_address.id;
                let mac_address_clone = mac_address.clone();
                let resolved_tags_json = self.resolve_tag_references(
                    netbox_client.as_ref(),
                    &mac_address_crd.spec.tags,
                    namespace,
                    name,
                    Some(mac_address_id),
                    "NetBoxMACAddress",
                ).await;
                let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                
                let mac_address = match crate::reconcile_helpers::update_tags_if_differ(
                    mac_address,
                    &mac_address_crd.spec.tags,
                    resolved_tags.clone(),
                    |tags| {
                        let mac_address_id_clone = mac_address_id;
                        let interface_id_clone = interface_id;
                        let description_clone = mac_address_crd.spec.description.clone();
                        let comments_clone = mac_address_crd.spec.comments.clone();
                        async move {
                            netbox_client.update_mac_address(
                                mac_address_id_clone,
                                Some("dcim.interface"),
                                Some(interface_id_clone),
                                description_clone,
                                comments_clone,
                                tags,
                            ).await
                        }
                    },
                    &format!("NetBoxMACAddress {}/{}", namespace, name),
                ).await {
                    Ok(Some(updated)) => {
                        use crate::events::reasons;
                        self.record_event_normal(
                            reasons::UPDATED,
                            &format!("Updated NetBoxMACAddress {}/{} tags in NetBox", namespace, name),
                            mac_address_crd,
                        ).await;
                        updated
                    }
                    Ok(None) => mac_address_clone, // Tags are up-to-date
                    Err(e) => {
                        warn!("Failed to update NetBoxMACAddress {}/{} tags: {}", namespace, name, e);
                        mac_address_clone // Use existing if update fails
                    }
                };
                
                // Update status if needed
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    mac_address_crd.status.as_ref(),
                    mac_address.id,
                    &mac_address.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    use crate::reconcile_helpers::update_resource_status;
                    let status_patch = Self::create_typed_mac_address_status_patch(
                        mac_address.id,
                        mac_address.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    update_resource_status(
                        &*self.netbox_mac_address_api,
                        name,
                        namespace,
                        &status_patch,
                        "NetBoxMACAddress",
                        mac_address.id,
                    ).await?;
                    debug!("Updated NetBoxMACAddress {}/{} status: NetBox ID {}", namespace, name, mac_address.id);
                }
                return Ok(());
            }
            None => {
                // Try to find existing MAC address
                let existing_mac_address = match netbox_client.get_mac_address_by_address(&mac_address_crd.spec.mac_address).await {
                    Ok(Some(ma)) => {
                        info!("MAC address {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", mac_address_crd.spec.mac_address, ma.id);
                        Some(ma)
                    }
                    Ok(None) => None,
                    Err(e) => {
                        warn!("Failed to query MAC address: {}, will try to create", e);
                        None
                    }
                };
                
                if let Some(existing) = existing_mac_address {
                    existing
                } else {
                    debug!("Attempting to create MAC address {} in NetBox", mac_address_crd.spec.mac_address);
                    match netbox_client.create_mac_address(
                        &mac_address_crd.spec.mac_address,
                        "dcim.interface", // assigned_object_type
                        interface.id, // assigned_object_id
                        mac_address_crd.spec.description.clone(),
                        mac_address_crd.spec.comments.clone(),
                    ).await {
                        Ok(created) => {
                            info!("Created MAC address {} in NetBox (ID: {})", created.mac_address, created.id);
                            // Emit event for successful creation
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::CREATED,
                                &format!("Created MAC address {} in NetBox (ID: {})", created.mac_address, created.id),
                                mac_address_crd,
                            ).await;
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create MAC address in NetBox: {}", e);
                            error!("{}", error_msg);
                            // Emit event for reconciliation failure
                            use crate::events::reasons;
                            self.record_event_warning(
                                reasons::RECONCILIATION_FAILED,
                                &error_msg,
                                mac_address_crd,
                            ).await;
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                }
            }
        };
        
        use crate::reconcile_helpers::update_resource_status;
        let status_patch = Self::create_typed_mac_address_status_patch(
            netbox_mac_address.id,
            netbox_mac_address.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_mac_address_api,
            name,
            namespace,
            &status_patch,
            "NetBoxMACAddress",
            netbox_mac_address.id,
        ).await?;
        info!("Updated NetBoxMACAddress {}/{} status: NetBox ID {}", namespace, name, netbox_mac_address.id);
        Ok(())
    }
}
