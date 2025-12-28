//! NetBoxMACAddress reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxMACAddress, ResourceState};
use netbox_client::NetBoxClientTrait;

impl Reconciler {
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
                return Err(ControllerError::InvalidConfig(error_msg));
            }
        };
        
        // Get tenant from device
        let tenant_ref = &device_crd.spec.tenant;
        let netbox_client = self.token_resolver
            .create_client_for_tenant(namespace, tenant_ref)
            .await?;
        
        // Resolve device ID and interface ID
        // If device hasn't been created yet, return early and let controller requeue when device is ready
        use crate::reconcile_helpers::resolve_dependency_id;
        let device_id = match resolve_dependency_id(
            device_crd.status.as_ref(),
            "Device",
            device_name,
        ) {
            Some(id) => id,
            None => return Ok(()), // Return early - controller will requeue when device status updates
        };
        
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
        
        let netbox_mac_address = match drift_result {
            DriftCheckResult::UseExisting(mac_address) => Some(mac_address),
            DriftCheckResult::StatusCleared { message } => {
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
                    return Ok(());
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
