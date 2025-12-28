//! NetBoxInterface reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxInterface, ResourceState};
use netbox_client::{NetBoxClientTrait, InterfaceId, DeviceId};

impl Reconciler {
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
        
        let netbox_interface = match drift_result {
            DriftCheckResult::UseExisting(interface) => {
                // Resource exists and is up-to-date
                Some(interface)
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
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    interface_crd.status.as_ref(),
                    interface.id,
                    &interface.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    use crate::reconcile_helpers::update_resource_status;
                    let status_patch = Self::create_typed_interface_status_patch(
                        interface.id,
                        interface.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    update_resource_status(
                        &*self.netbox_interface_api,
                        name,
                        namespace,
                        &status_patch,
                        "NetBoxInterface",
                        interface.id,
                    ).await?;
                    debug!("Updated NetBoxInterface {}/{} status: NetBox ID {}", namespace, name, interface.id);
                    return Ok(());
                } else {
                    debug!("NetBoxInterface {}/{} already has correct status (ID: {}), skipping update", namespace, name, interface.id);
                    return Ok(());
                }
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
                    ).await {
                        Ok(created) => {
                            info!("Created interface {} on device {} in NetBox (ID: {})", created.name, device_name, created.id);
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create interface in NetBox: {}", e);
                            error!("{}", error_msg);
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
