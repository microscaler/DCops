//! NetBoxMACAddress reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers;
use tracing::{info, error, debug, warn};
use crds::{NetBoxMACAddress, ResourceState};
use netbox_client::NetBoxClientTrait;

impl Reconciler {
    pub async fn reconcile_netbox_mac_address(&self, mac_address_crd: &NetBoxMACAddress) -> Result<(), ControllerError> {
        let name = mac_address_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxMACAddress missing name".to_string()))?;
        let namespace = mac_address_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
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
                return Err(ControllerError::InvalidConfig(error_msg));
            }
        };
        
        // Get tenant from device
        let tenant_ref = &device_crd.spec.tenant;
        let netbox_client = self.token_resolver
            .create_client_for_tenant(namespace, tenant_ref)
            .await?;
        
        // Resolve device ID and interface ID
        let device_id = device_crd.status
            .as_ref()
            .and_then(|s| s.netbox_id)
            .ok_or_else(|| ControllerError::InvalidConfig(
                format!("Device '{}' has not been created in NetBox yet (no netbox_id in status)", device_name)
            ))?;
        
        // Find interface by querying
        let interface = match netbox_client.query_interfaces(&[("device_id", &device_id.to_string()), ("name", interface_name)], false).await {
            Ok(mut interfaces) => {
                interfaces.pop().ok_or_else(|| ControllerError::InvalidConfig(
                    format!("Interface '{}' not found on device '{}'", interface_name, device_name)
                ))?
            }
            Err(e) => {
                return Err(ControllerError::NetBox(e));
            }
        };
        
        // Check if already created - use helper for drift detection
        let netbox_mac_address = if let Some(status) = &mac_address_crd.status {
            if status.state == ResourceState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    match reconcile_helpers::check_existing(
                        &netbox_client,
                        netbox_id,
                        &format!("NetBoxMACAddress {}/{}", namespace, name),
                        async {
                            netbox_client.get_mac_address_by_address(&mac_address_crd.spec.mac_address)
                                .await
                                .and_then(|opt| opt.ok_or_else(|| netbox_client::NetBoxError::NotFound(format!("MAC address {} not found", mac_address_crd.spec.mac_address))))
                        },
                    ).await {
                        Ok(Some(resource)) => Some(resource),
                        Ok(None) => {
                            warn!("NetBoxMACAddress {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
                            let status_patch = Self::create_typed_mac_address_status_patch(
                                0, String::new(), ResourceState::Pending,
                                Some("Resource was deleted in NetBox, will recreate".to_string()),
                            );
                            let pp = kube::api::PatchParams::default();
                            if let Err(e) = self.netbox_mac_address_api
                                .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                                .await
                            {
                                warn!("Failed to clear NetBoxMACAddress status after drift detection: {}", e);
                            }
                            None
                        }
                        Err(e) => return Err(e),
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        let netbox_mac_address = match netbox_mac_address {
            Some(mac_address) => {
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    mac_address_crd.status.as_ref(),
                    mac_address.id,
                    &mac_address.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_typed_mac_address_status_patch(
                        mac_address.id,
                        mac_address.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_mac_address_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxMACAddress {}/{} status: NetBox ID {}", namespace, name, mac_address.id);
                            return Ok(());
                        }
                        Err(e) => {
                            error!("Failed to update NetBoxMACAddress status: {}", e);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxMACAddress {}/{} already has correct status (ID: {}), skipping update", namespace, name, mac_address.id);
                    return Ok(());
                }
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
                    info!("Creating MAC address {} in NetBox", mac_address_crd.spec.mac_address);
                    match netbox_client.create_mac_address(
                        &mac_address_crd.spec.mac_address,
                        "dcim.interface", // assigned_object_type
                        interface.id, // assigned_object_id
                        mac_address_crd.spec.description.clone(),
                        mac_address_crd.spec.comments.clone(),
                    ).await {
                        Ok(created) => {
                            info!("Created MAC address {} in NetBox (ID: {})", created.mac_address, created.id);
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create MAC address in NetBox: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                }
            }
        };
        
        let status_patch = Self::create_typed_mac_address_status_patch(
            netbox_mac_address.id,
            netbox_mac_address.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_mac_address_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxMACAddress {}/{} status: NetBox ID {}", namespace, name, netbox_mac_address.id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxMACAddress status: {}", e);
                error!("{}", error_msg);
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}
