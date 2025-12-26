//! NetBoxMACAddress reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error};
use crds::{NetBoxMACAddress, ResourceState};
use netbox_client::MACAddress;

impl Reconciler {
    pub async fn reconcile_netbox_mac_address(&self, mac_crd: &NetBoxMACAddress) -> Result<(), ControllerError> {
        let name = mac_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxMACAddress missing name".to_string()))?;
        let namespace = mac_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxMACAddress {}/{}", namespace, name);
        
        // Note: MAC addresses are managed via interfaces in NetBox, not as standalone resources.
        // The netbox_id in status refers to the interface ID, not a MAC address ID.
        // We check if the interface exists and has the MAC address set correctly.
        
        // Parse interface reference (format: "device-name/interface-name")
        let (device_name, interface_name) = {
            let parts: Vec<&str> = mac_crd.spec.interface.split('/').collect();
            if parts.len() != 2 {
                return Err(ControllerError::InvalidConfig(
                    format!("Invalid interface format '{}', expected 'device-name/interface-name'", mac_crd.spec.interface)
                ));
            }
            (parts[0], parts[1])
        };
        
        // Resolve interface ID by querying NetBox directly
        let interface_id = {
            // First get the device to get its NetBox ID
            let device_crd = self.netbox_device_api.get(device_name).await
                .map_err(|_| ControllerError::InvalidConfig(
                    format!("Device CRD '{}' not found for MAC address {}", device_name, name)
                ))?;
            
            let device_id = device_crd.status
                .as_ref()
                .and_then(|s| s.netbox_id)
                .ok_or_else(|| ControllerError::InvalidConfig(
                    format!("Device '{}' has not been created in NetBox yet (no netbox_id in status)", device_name)
                ))?;
            
            // Query NetBox for the interface by device_id and interface name
            match self.netbox_client.query_interfaces(
                &[("device_id", &device_id.to_string()), ("name", interface_name)],
                false,
            ).await {
                Ok(interfaces) => {
                    if let Some(interface) = interfaces.first() {
                        interface.id
                    } else {
                        return Err(ControllerError::InvalidConfig(
                            format!("Interface '{}/{}' not found in NetBox for MAC address {}", device_name, interface_name, name)
                        ));
                    }
                }
                Err(e) => {
                    return Err(ControllerError::NetBox(e));
                }
            }
        };
        
        // In NetBox, MAC addresses are managed via the interface's mac_address field, not as standalone resources.
        // The /api/dcim/mac-addresses/ endpoint may not be accessible or may return HTML 404 responses.
        // We manage MAC addresses by setting the mac_address field on the interface directly.
        info!("Setting MAC address {} on interface {} (ID: {})", mac_crd.spec.mac_address, interface_name, interface_id);
        
        // Check if MAC address is already set correctly on the interface
        let interface = match self.netbox_client.get_interface(interface_id).await {
            Ok(iface) => iface,
            Err(e) => {
                let error_msg = format!("Failed to get interface {} from NetBox: {}", interface_id, e);
                error!("{}", error_msg);
                return Err(ControllerError::NetBox(e));
            }
        };
        
        // Check if MAC address is already set correctly
        let mac_already_set = interface.mac_address.as_ref()
            .map(|mac| {
                // Normalize MAC addresses for comparison (remove colons/dashes, lowercase)
                let existing = mac.to_lowercase().replace(":", "").replace("-", "");
                let desired = mac_crd.spec.mac_address.to_lowercase().replace(":", "").replace("-", "");
                existing == desired
            })
            .unwrap_or(false);
        
        let netbox_mac = if mac_already_set {
            info!("MAC address {} already set on interface {} (ID: {})", mac_crd.spec.mac_address, interface.name, interface_id);
            // Create a proxy MAC address object for status update
            MACAddress {
                id: interface_id,
                url: format!("{}/api/dcim/interfaces/{}/", self.netbox_client.base_url(), interface_id),
                display: format!("{} ({})", mac_crd.spec.mac_address, interface.name),
                mac_address: mac_crd.spec.mac_address.clone(),
                assigned_object_type: Some("dcim.interface".to_string()),
                assigned_object_id: Some(interface_id),
                assigned_object: None,
                description: mac_crd.spec.description.clone(),
                comments: mac_crd.spec.comments.clone(),
                tags: vec![],
                created: "".to_string(),
                last_updated: "".to_string(),
            }
        } else {
            // Update interface to set MAC address
            match self.netbox_client.update_interface(
                interface_id,
                None, // name
                None, // type
                None, // enabled
                Some(&mac_crd.spec.mac_address), // mac_address
                mac_crd.spec.description.as_deref(), // description
            ).await {
                Ok(updated_interface) => {
                    info!("Set MAC address {} on interface {} (ID: {})", mac_crd.spec.mac_address, updated_interface.name, interface_id);
                    // Create a proxy MAC address object for status update
                    MACAddress {
                        id: interface_id,
                        url: format!("{}/api/dcim/interfaces/{}/", self.netbox_client.base_url(), interface_id),
                        display: format!("{} ({})", mac_crd.spec.mac_address, updated_interface.name),
                        mac_address: mac_crd.spec.mac_address.clone(),
                        assigned_object_type: Some("dcim.interface".to_string()),
                        assigned_object_id: Some(interface_id),
                        assigned_object: None,
                        description: mac_crd.spec.description.clone(),
                        comments: mac_crd.spec.comments.clone(),
                        tags: vec![],
                        created: "".to_string(),
                        last_updated: "".to_string(),
                    }
                }
                Err(update_err) => {
                    let error_msg = format!("Failed to set MAC address on interface {}: {}", interface_id, update_err);
                    error!("{}", error_msg);
                    return Err(ControllerError::NetBox(update_err));
                }
            }
        };
        
        // Update status (use lowercase state to match CRD validation schema)
        let status_patch = Self::create_resource_status_patch(
            netbox_mac.id,
            netbox_mac.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_mac_address_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxMACAddress {}/{} status: NetBox ID {}", namespace, name, netbox_mac.id);
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
