//! NetBoxInterface reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers;
use tracing::{info, error, debug, warn};
use crds::{NetBoxInterface, ResourceState};

impl Reconciler {
    pub async fn reconcile_netbox_interface(&self, interface_crd: &NetBoxInterface) -> Result<(), ControllerError> {
        let name = interface_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxInterface missing name".to_string()))?;
        let namespace = interface_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxInterface {}/{}", namespace, name);
        
        // Check if already created - use helper for drift detection
        let netbox_interface = if let Some(status) = &interface_crd.status {
            if status.state == ResourceState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    // Use simple helper function for drift detection (no update logic)
                    match reconcile_helpers::check_existing(
                        &self.netbox_client,
                        netbox_id,
                        &format!("NetBoxInterface {}/{}", namespace, name),
                        self.netbox_client.get_interface(netbox_id),
                    ).await {
                        Ok(Some(resource)) => {
                            // Resource exists and is up-to-date
                            Some(resource)
                        }
                        Ok(None) => {
                            // Drift detected - resource was deleted, clear status and recreate
                            warn!("NetBoxInterface {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
                            let status_patch = Self::create_resource_status_patch(
                                0, // Clear netbox_id
                                String::new(), // Clear URL
                                ResourceState::Pending,
                                Some("Resource was deleted in NetBox, will recreate".to_string()),
                            );
                            let pp = kube::api::PatchParams::default();
                            if let Err(e) = self.netbox_interface_api
                                .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                                .await
                            {
                                warn!("Failed to clear NetBoxInterface status after drift detection: {}", e);
                            }
                            // Fall through to creation
                            None
                        }
                        Err(e) => {
                            // Error during drift detection - return to retry
                            return Err(e);
                        }
                    }
                } else {
                    None // No netbox_id, need to create
                }
            } else {
                None // Not in Created state, need to create
            }
        } else {
            None // No status, need to create
        };
        
        // Handle existing interface (from helper) or create new
        let netbox_interface = match netbox_interface {
            Some(interface) => {
                // Resource exists and is up-to-date - only update status if it changed
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    interface_crd.status.as_ref(),
                    interface.id,
                    &interface.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_resource_status_patch(
                        interface.id,
                        interface.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_interface_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxInterface {}/{} status: NetBox ID {}", namespace, name, interface.id);
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update NetBoxInterface status: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxInterface {}/{} already has correct status (ID: {}), skipping update", namespace, name, interface.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create interface - resolve dependencies first
                // Resolve device ID
                let device_id = match self.netbox_device_api.get(&interface_crd.spec.device).await {
                    Ok(device_crd) => {
                        device_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                            .ok_or_else(|| ControllerError::InvalidConfig(
                                format!("Device '{}' has not been created in NetBox yet (no netbox_id in status)", interface_crd.spec.device)
                            ))?
                    }
                    Err(_) => {
                        return Err(ControllerError::InvalidConfig(
                            format!("Device CRD '{}' not found for interface {}", interface_crd.spec.device, name)
                        ));
                    }
                };
                
                // Try to find existing interface by device and name
                let existing_interface = match self.netbox_client.query_interfaces(
                    &[("device_id", &device_id.to_string()), ("name", &interface_crd.spec.name)],
                    false,
                ).await {
                    Ok(interfaces) => interfaces.first().cloned(),
                    Err(_) => None
                };
                
                let netbox_interface = if let Some(existing) = existing_interface {
                    info!("Interface {} on device {} already exists in NetBox (ID: {})", interface_crd.spec.name, interface_crd.spec.device, existing.id);
                    existing
                } else {
                    match self.netbox_client.create_interface(
                        device_id,
                        &interface_crd.spec.name,
                        &interface_crd.spec.r#type,
                        Some(interface_crd.spec.enabled),
                        interface_crd.spec.mac_address.as_deref(),
                        interface_crd.spec.mtu,
                        interface_crd.spec.description.clone(),
                    ).await {
                        Ok(created) => {
                            info!("Created interface {} on device {} in NetBox (ID: {})", interface_crd.spec.name, interface_crd.spec.device, created.id);
                            created
                        }
                        Err(e) => {
                            // Check if error is "already exists" - if so, try to find it (idempotency)
                            let error_str = format!("{}", e);
                            if error_str.contains("already exists") || error_str.contains("unique") || error_str.contains("duplicate") {
                                warn!("Interface {} on device {} already exists in NetBox, attempting to retrieve it (idempotency)", interface_crd.spec.name, interface_crd.spec.device);
                                
                                // Try to find the existing interface using fetch_all
                                match self.netbox_client.query_interfaces(
                                    &[("device_id", &device_id.to_string()), ("name", &interface_crd.spec.name)],
                                    true, // fetch_all
                                ).await {
                                    Ok(interfaces) => {
                                        if let Some(found) = interfaces.first() {
                                            info!("Found existing interface {} on device {} in NetBox (ID: {}) after create conflict", interface_crd.spec.name, interface_crd.spec.device, found.id);
                                            found.clone()
                                        } else {
                                            // Interface exists but we can't find it - this is unusual
                                            let error_msg = format!("Interface {} on device {} already exists in NetBox but could not retrieve it: {}", interface_crd.spec.name, interface_crd.spec.device, e);
                                            error!("{}", error_msg);
                                            return Err(ControllerError::NetBox(e));
                                        }
                                    }
                                    Err(query_err) => {
                                        // Couldn't query - this is a real error
                                        let error_msg = format!("Failed to create interface in NetBox (may already exist, but could not verify): {} (query error: {})", e, query_err);
                                        error!("{}", error_msg);
                                        return Err(ControllerError::NetBox(e));
                                    }
                                }
                            } else {
                                let error_msg = format!("Failed to create interface in NetBox: {}", e);
                                error!("{}", error_msg);
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    }
                };
                
                netbox_interface
            }
        };
        
        // Update status (use lowercase state to match CRD validation schema)
        let status_patch = Self::create_resource_status_patch(
            netbox_interface.id,
            netbox_interface.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_interface_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxInterface {}/{} status: NetBox ID {}", namespace, name, netbox_interface.id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxInterface status: {}", e);
                error!("{}", error_msg);
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}
